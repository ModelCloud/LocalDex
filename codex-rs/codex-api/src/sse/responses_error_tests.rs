//! Regression coverage for retry advice carried by failed Responses events.

use super::process_responses_event;
use super::spawn_response_stream;
use crate::api_bridge::map_api_error;
use crate::common::ResponseEvent;
use bytes::Bytes;
use codex_client::StreamResponse;
use codex_http_client::RetryAfter;
use codex_protocol::error::CodexErrorDetails;
use codex_protocol::protocol::CodexErrorInfo;
use futures::StreamExt;
use futures::stream;
use http::HeaderMap;
use http::StatusCode;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn incomplete_response_is_terminal_without_waiting_for_eof() {
    for reason in [Some("max_output_tokens"), Some("unknown_reason"), None] {
        for trailing_completed in [false, true] {
            let incomplete = json!({
                "type": "response.incomplete",
                "response": {
                    "id": "resp-incomplete",
                    "incomplete_details": {"reason": reason}
                }
            });
            let mut body = format!("data: {incomplete}\n\n");
            if trailing_completed {
                let completed = json!({
                    "type": "response.completed",
                    "response": {"id": "later"}
                });
                body.push_str(&format!("data: {completed}\n\n"));
            }
            let bytes = stream::iter([Ok(Bytes::from(body))]).chain(stream::pending());
            let mut response_stream = spawn_response_stream(
                StreamResponse {
                    status: StatusCode::OK,
                    headers: HeaderMap::new(),
                    bytes: Box::pin(bytes),
                },
                Duration::from_secs(60),
                None,
                None,
            );
            let started = tokio::time::Instant::now();
            let error = loop {
                match response_stream.next().await {
                    Some(Ok(ResponseEvent::RateLimits(_))) => {}
                    Some(Err(error)) => break map_api_error(error),
                    event => panic!("expected incomplete-response error, got {event:?}"),
                }
            };
            assert_eq!(
                error.to_string(),
                format!(
                    "Incomplete response returned, reason: {}",
                    reason.unwrap_or("unknown")
                )
            );
            assert_eq!(error.retry_delay(1), None);
            assert_eq!(started.elapsed(), Duration::ZERO);
            assert!(response_stream.next().await.is_none());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn incomplete_context_window_triggers_compaction_without_waiting_for_eof() {
    let incomplete = json!({
        "type": "response.incomplete",
        "response": {
            "id": "resp-context-full",
            "incomplete_details": {"reason": "context_length_exceeded"}
        }
    });
    let bytes =
        stream::iter([Ok(Bytes::from(format!("data: {incomplete}\n\n")))]).chain(stream::pending());
    let mut response_stream = spawn_response_stream(
        StreamResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            bytes: Box::pin(bytes),
        },
        Duration::from_secs(60),
        None,
        None,
    );
    let started = tokio::time::Instant::now();
    let error = loop {
        match response_stream.next().await {
            Some(Ok(ResponseEvent::RateLimits(_))) => {}
            Some(Err(error)) => break map_api_error(error),
            event => panic!("expected context-window error, got {event:?}"),
        }
    };
    assert!(
        matches!(error.details(), CodexErrorDetails::ContextWindowExceeded),
        "{error:?}"
    );
    assert_eq!(started.elapsed(), Duration::ZERO);
    assert!(response_stream.next().await.is_none());
}

#[test]
fn incomplete_response_preserves_content_filter_and_interruption() {
    let filtered = serde_json::from_value(json!({
        "type": "response.incomplete",
        "response": {"incomplete_details": {"reason": "content_filter"}}
    }))
    .unwrap();
    let error = map_api_error(
        process_responses_event(filtered)
            .unwrap_err()
            .into_api_error(),
    );
    assert!(matches!(
        error.details(),
        codex_protocol::error::CodexErrorDetails::ContentFilter
    ));
    assert!(error.retry_delay(1).is_some());

    let interrupted = serde_json::from_value(json!({
        "type": "response.incomplete",
        "response": {
            "id": "resp-interrupted",
            "incomplete_details": {"reason": "interrupted"},
            "usage": {"input_tokens": 10, "output_tokens": 2, "total_tokens": 12}
        }
    }))
    .unwrap();
    let Some(ResponseEvent::Completed {
        response_id,
        token_usage,
        end_turn,
        ..
    }) = process_responses_event(interrupted).unwrap()
    else {
        panic!("interrupted response must remain continuable");
    };
    assert_eq!(response_id, "resp-interrupted");
    assert_eq!(end_turn, Some(false));
    assert_eq!(token_usage.unwrap().total_tokens, 12);
}

/// Flex failures remain terminal even with retry headers and a later completed event.
#[tokio::test(start_paused = true)]
async fn flex_failure_with_retry_header_ends_stream_immediately() {
    let error = json!({
        "code": "flex_unavailable",
        "plan_type": 42,
        "headers": {"retry-after": "300"}
    });
    for failure in [
        json!({"type": "error", "error": error}),
        json!({"type": "response.failed", "response": {"error": error}}),
    ] {
        let completed = json!({"type": "response.completed", "response": {"id": "later"}});
        let bytes = stream::iter([Ok(Bytes::from(format!(
            "data: {failure}\n\ndata: {completed}\n\n"
        )))])
        .chain(stream::pending());
        let mut stream = spawn_response_stream(
            StreamResponse {
                status: StatusCode::OK,
                headers: HeaderMap::new(),
                bytes: Box::pin(bytes),
            },
            Duration::from_secs(60),
            /*telemetry*/ None,
            /*turn_state*/ None,
        );
        let error = loop {
            match stream.next().await {
                Some(Ok(ResponseEvent::RateLimits(_))) => {}
                Some(Err(error)) => break map_api_error(error),
                event => panic!("expected a Flex error, got {event:?}"),
            }
        };
        assert_eq!(
            (
                error.to_codex_protocol_error(),
                error.retry_delay(/*retry_count*/ 1)
            ),
            (CodexErrorInfo::FlexUnavailable, None)
        );
        assert!(stream.next().await.is_none());
    }
}

/// Error headers beat rate-limit message advice; invalid ones fall back.
#[tokio::test(start_paused = true)]
async fn rate_limit_plaintext_loses_to_valid_error_header() {
    for code in ["rate_limit_exceeded", "slow_down"] {
        for (error_header, expected_seconds) in [
            (Some("2"), 2),
            (Some("0"), 0),
            (Some("Wed, 21 Oct 2015 07:28:00 GMT"), 0),
            (Some("bad"), 35),
            (None, 35),
        ] {
            let event = serde_json::from_value(json!({
                "type": "response.failed",
                "response": {
                    "error": {
                        "code": code,
                        "message": "Try again in 35 seconds.",
                        "headers": {"Retry-After": error_header}
                    }
                }
            }))
            .unwrap();
            let error = map_api_error(process_responses_event(event).unwrap_err().into_api_error());
            assert_eq!(
                error.retry_after(),
                RetryAfter::from_delay(Duration::from_secs(expected_seconds))
            );
        }
    }
}

/// Retry advice uses the same HTTP value validation as other event headers.
#[tokio::test(start_paused = true)]
async fn streamed_retry_after_uses_http_header_validation() {
    for (error_headers, expected_seconds) in [
        (json!({"retry-after": "\n5\n"}), 12),
        (json!({"retry-after": "\t5\t"}), 5),
        (json!({"Retry-After": "5", "retry-after": "30"}), 30),
    ] {
        let event = serde_json::from_value(json!({
            "type": "response.failed",
            "response": {"error": {"code": "rate_limit_exceeded", "message": "Try again in 12 seconds.", "headers": error_headers}}
        }))
        .unwrap();
        let error = map_api_error(process_responses_event(event).unwrap_err().into_api_error());
        assert_eq!(
            error.retry_after(),
            RetryAfter::from_delay(Duration::from_secs(expected_seconds))
        );
    }
}

/// Event headers retain their deadlines for overload and generic failures.
#[tokio::test(start_paused = true)]
async fn retry_header_survives_other_failed_responses() {
    for (code, expected_code) in [
        ("server_is_overloaded", CodexErrorInfo::ServerOverloaded),
        ("unknown_failure", CodexErrorInfo::Other),
    ] {
        let event = serde_json::from_value(json!({
            "type": "response.failed",
            "response": {"error": {"code": code, "headers": {"retry-after": "7"}}},
        }))
        .unwrap();
        let error = map_api_error(process_responses_event(event).unwrap_err().into_api_error());
        assert_eq!(
            (
                error.to_codex_protocol_error(),
                error.retry_after(),
                error.retry_delay(/*retry_count*/ 1),
            ),
            (
                expected_code,
                RetryAfter::from_delay(Duration::from_secs(7)),
                Some(Duration::from_secs(7)),
            )
        );
    }
}

/// An event header cannot make a quota or policy failure retryable.
#[tokio::test(start_paused = true)]
async fn retry_header_does_not_override_terminal_errors() {
    for code in ["insufficient_quota", "cyber_policy", "bio_policy"] {
        let event = serde_json::from_value(json!({
            "type": "response.failed",
            "response": {"error": {"code": code, "headers": {"retry-after": "1"}}}
        }))
        .unwrap();
        let error = map_api_error(process_responses_event(event).unwrap_err().into_api_error());
        assert_eq!(error.retry_delay(/*retry_count*/ 1), None, "{code}");
    }
}
