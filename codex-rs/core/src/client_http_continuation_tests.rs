use super::super::LastResponse;
use super::Prompt;
use super::test_model_client;
use super::test_model_info;
use super::test_responses_metadata_for_client;
use codex_model_provider::create_model_provider;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::SessionSource;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn http_continuation_retains_pending_completion_and_checks_reuse_boundaries() -> anyhow::Result<()>
{
    for scenario in [
        "same_provider",
        "other_provider",
        "other_owner",
        "other_model",
        "edited_history",
    ] {
        let base = test_model_client(SessionSource::Cli);
        let mut provider = base.provider_info().clone();
        provider.supports_responses_continuation = true;
        let client =
            base.with_provider(create_model_provider(provider, /*auth_manager*/ None));
        let first: ResponseItem = serde_json::from_value(json!({
            "type": "message", "role": "user",
            "content": [{"type": "input_text", "text": "first prompt"}],
        }))?;
        let next: ResponseItem = serde_json::from_value(json!({
            "type": "message", "role": "user",
            "content": [{"type": "input_text", "text": "next prompt"}],
        }))?;
        let previous = client.build_responses_request(
            &Prompt {
                input: vec![first],
                ..Default::default()
            },
            &test_model_info(),
            /*effort*/ None,
            ReasoningSummary::None,
            /*service_tier*/ None,
            &test_responses_metadata_for_client(
                &client,
                /*turn_id*/ None,
                format!("{}:0", client.state.thread_id),
                /*parent_thread_id*/ None,
                super::TestCodexResponsesRequestKind::Turn,
            ),
            /*include_internal*/ false,
        )?;
        let mut current = previous.clone();
        current.input.push(next.clone());
        let mut turn = client.new_session();
        turn.http_session.last_request = Some(previous);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        turn.http_session.last_response_rx = Some(receiver);
        assert!(turn.get_last_http_response().is_none());
        if scenario == "other_owner" {
            turn.http_session.auth_owner_generation = Some(1);
        }
        drop(turn);
        sender
            .send(LastResponse {
                response_id: "resp-1".to_string(),
                items_added: Vec::new(),
            })
            .unwrap();

        let next_client = if scenario == "other_provider" {
            let mut provider = create_oss_provider_with_base_url(
                "https://other.example.com/v1",
                WireApi::Responses,
            );
            provider.supports_responses_continuation = true;
            client.with_provider(create_model_provider(provider, /*auth_manager*/ None))
        } else {
            client.with_provider(client.state.provider.clone())
        };
        if scenario == "other_model" {
            current.model = "another-model".to_string();
        }
        if scenario == "edited_history" {
            current.input[0] = next.clone();
        }
        let mut turn = next_client.new_session();
        assert_eq!(
            turn.prepare_http_request(&current)
                .map(|continuation| (continuation.response_id, continuation.items)),
            (scenario == "same_provider").then_some(("resp-1".to_string(), vec![next])),
            "{scenario}",
        );
    }
    Ok(())
}
