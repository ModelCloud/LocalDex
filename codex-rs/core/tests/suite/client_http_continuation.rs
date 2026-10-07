use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_response_sequence;
use core_test_support::responses::sse;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::MockServer;
use wiremock::ResponseTemplate;

#[test_case::test_case(false; "standard")]
#[test_case::test_case(true; "responses_lite")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn responses_http_continuation_survives_user_turns(
    responses_lite: bool,
) -> anyhow::Result<()> {
    let server = MockServer::start().await;
    let response_mock = mount_response_sequence(
        &server,
        [
            (
                "turn-one-routing",
                vec![
                    ev_response_created("resp-1"),
                    ev_function_call("call-1", "unsupported_tool", "{}"),
                    ev_completed("resp-1"),
                ],
            ),
            (
                "turn-one-routing",
                vec![
                    ev_response_created("resp-2"),
                    ev_assistant_message("msg-1", "first answer"),
                    ev_completed("resp-2"),
                ],
            ),
            (
                "turn-two-routing",
                vec![
                    ev_response_created("resp-3"),
                    ev_function_call("call-2", "unsupported_tool", "{}"),
                    ev_completed("resp-3"),
                ],
            ),
            (
                "turn-two-routing",
                vec![
                    ev_response_created("resp-4"),
                    ev_assistant_message("msg-2", "second answer"),
                    ev_completed("resp-4"),
                ],
            ),
        ]
        .into_iter()
        .map(|(turn_state, events)| {
            ResponseTemplate::new(/*status*/ 200)
                .insert_header("content-type", "text/event-stream")
                .insert_header("x-codex-turn-state", turn_state)
                .set_body_string(sse(events))
        })
        .collect(),
    )
    .await;
    let mut provider =
        create_oss_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses);
    provider.supports_responses_continuation = true;
    provider.request_max_retries = Some(0);
    provider.stream_max_retries = Some(0);
    let test = test_codex()
        .with_model_info_override("gpt-5.5", move |model_info| {
            model_info.use_responses_lite = responses_lite;
        })
        .with_config(move |config| {
            config.model_provider_id = provider.name.clone();
            config.model_provider = provider;
            config.base_instructions = Some("Stable base instructions".to_string());
        })
        .build_with_auto_env(&server)
        .await?;

    test.submit_turn("turn one").await?;
    test.submit_turn("turn two").await?;

    let requests = response_mock.requests();
    assert_eq!(requests.len(), 4);
    let first_input = requests[0].input();
    let instructions_index = usize::from(responses_lite);
    assert_eq!(first_input[instructions_index]["role"], "developer");
    assert_eq!(
        first_input[instructions_index]["content"],
        json!([{"type": "input_text", "text": "Stable base instructions"}]),
    );
    let first_body = requests[0].body_json();
    let tools = if responses_lite {
        assert_eq!(first_input[0]["type"], "additional_tools");
        first_input[0]["tools"].as_array().unwrap()
    } else {
        first_body["tools"].as_array().unwrap()
    };
    assert!(!tools.is_empty());
    if !responses_lite {
        assert!(tools.iter().all(|tool| tool["type"] != "namespace"));
    }
    assert_eq!(
        requests
            .iter()
            .map(|request| request.body_json()["previous_response_id"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!(null),
            json!("resp-1"),
            json!("resp-2"),
            json!("resp-3")
        ],
    );
    assert_eq!(
        requests
            .iter()
            .map(|request| request.header("x-codex-turn-state"))
            .collect::<Vec<_>>(),
        vec![
            None,
            Some("turn-one-routing".to_string()),
            None,
            Some("turn-two-routing".to_string()),
        ],
    );
    assert_eq!(requests[2].message_input_texts("user"), vec!["turn two"]);
    assert_eq!(requests[2].input().len(), 1);
    for (request, call_id) in [(&requests[1], "call-1"), (&requests[3], "call-2")] {
        assert_eq!(request.input().len(), 1);
        assert_eq!(request.input()[0]["type"], "function_call_output");
        assert_eq!(request.input()[0]["call_id"], call_id);
    }
    Ok(())
}
