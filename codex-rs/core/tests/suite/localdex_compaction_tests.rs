//! Local compatibility compaction compares history estimates, not server usage.
use super::*;
use codex_model_provider_info::LOCALDEX_PROVIDER_ID;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_model_provider_info::merge_configured_model_providers;
use pretty_assertions::assert_eq;

#[test_case::test_case(false; "shrinking_history_with_larger_estimate_than_server_usage")]
#[test_case::test_case(true; "growing_history_stops_with_warning")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn localdex_post_turn_compaction_measures_history_progress(growing: bool) -> Result<()> {
    let server = MockServer::start().await;
    let answer = "Detailed original answer. ".repeat(4_000);
    let summary = if growing {
        "Expanded summary that increases the history. ".repeat(8_000)
    } else {
        "Concise summary preserving the answer.".to_string()
    };
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("answer", &answer),
                ev_completed_with_tokens("r1", 1_500),
            ]),
            sse(vec![
                ev_assistant_message("summary", &summary),
                ev_completed_with_tokens("r2", 10),
            ]),
        ],
    )
    .await;
    let mut providers = merge_configured_model_providers(
        Default::default(),
        std::collections::HashMap::from([(
            LOCALDEX_PROVIDER_ID.to_string(),
            create_oss_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses),
        )]),
    )
    .expect("LocalDex provider should normalize");
    let provider = providers.remove(LOCALDEX_PROVIDER_ID).unwrap();
    let test = test_codex()
        .with_model_info_override("QB/DSV4.1-Flash", |info| {
            info.context_window = Some(100_000);
            info.max_context_window = None;
        })
        .with_config(move |config| {
            config.model_provider_id = LOCALDEX_PROVIDER_ID.to_string();
            config
                .model_providers
                .insert(LOCALDEX_PROVIDER_ID.to_string(), provider.clone());
            config.model_provider = provider;
            config.model_auto_compact_token_limit = Some(100_000);
            config.model_post_turn_compact_threshold_percent = 1;
            let _ = config.features.disable(Feature::TokenBudget);
            set_test_compact_prompt(config);
        })
        .build_with_auto_env(&server)
        .await?;
    test.codex
        .start_or_steer_turn(
            TurnInputRequest::user_input(vec![UserInput::Text {
                text: "Give a detailed answer".to_string(),
                text_elements: vec![],
            }])
            .with_thread_settings(ThreadSettingsOverrides {
                model: Some("QB/DSV4.1-Flash".to_string()),
                ..Default::default()
            }),
        )
        .await?;
    let mut warnings = Vec::new();
    let completed = wait_for_event(&test.codex, |event| {
        if let EventMsg::Warning(warning) = event {
            warnings.push(warning.message.clone());
        }
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    let EventMsg::TurnComplete(completed) = completed else {
        unreachable!()
    };
    assert_eq!(
        completed.error, None,
        "post-turn compaction preserves the completed answer"
    );
    assert_eq!(
        completed.last_agent_message.as_deref(),
        Some(answer.as_str())
    );
    assert_eq!(
        mock.requests().len(),
        2,
        "one answer and one compaction request"
    );
    assert_eq!(
        warnings
            .iter()
            .any(|warning| warning.contains("without reducing estimated context")),
        growing,
        "guard must compare local history on both sides: {warnings:?}"
    );
    assert!(body_contains_text(
        &mock.requests()[1].body_json().to_string(),
        SUMMARIZATION_PROMPT
    ));
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_localdex_compaction_allows_new_user_context() -> Result<()> {
    let server = MockServer::start().await;
    let answer = "A long answer. ".repeat(4_000);
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_assistant_message("answer-1", &answer),
                ev_completed_with_tokens("answer-1", 1_500),
            ]),
            sse_failed(
                "failed-compact",
                "server_error",
                "malformed compaction output",
            ),
            sse(vec![
                ev_assistant_message("answer-2", &answer),
                ev_completed_with_tokens("answer-2", 1_500),
            ]),
            sse(vec![
                ev_assistant_message("summary", "Concise summary."),
                ev_completed_with_tokens("compact-2", 10),
            ]),
        ],
    )
    .await;
    let mut providers = merge_configured_model_providers(
        Default::default(),
        std::collections::HashMap::from([(
            LOCALDEX_PROVIDER_ID.to_string(),
            create_oss_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses),
        )]),
    )
    .expect("LocalDex provider should normalize");
    let mut provider = providers.remove(LOCALDEX_PROVIDER_ID).unwrap();
    provider.stream_max_retries = Some(0);
    let test = test_codex()
        .with_model_info_override("QB/DSV4.1-Flash", |info| {
            info.context_window = Some(100_000);
            info.max_context_window = None;
        })
        .with_config(move |config| {
            config.model_provider_id = LOCALDEX_PROVIDER_ID.to_string();
            config
                .model_providers
                .insert(LOCALDEX_PROVIDER_ID.to_string(), provider.clone());
            config.model_provider = provider;
            config.model_auto_compact_token_limit = Some(100_000);
            config.model_post_turn_compact_threshold_percent = 1;
            let _ = config.features.disable(Feature::TokenBudget);
            set_test_compact_prompt(config);
        })
        .build_with_auto_env(&server)
        .await?;

    for message in ["First user turn", "New user turn after failed compaction"] {
        test.codex
            .start_or_steer_turn(
                TurnInputRequest::user_input(vec![UserInput::Text {
                    text: message.to_string(),
                    text_elements: vec![],
                }])
                .with_thread_settings(ThreadSettingsOverrides {
                    model: Some("QB/DSV4.1-Flash".to_string()),
                    ..Default::default()
                }),
            )
            .await?;
        let completed = wait_for_event_match(&test.codex, |event| match event {
            EventMsg::TurnComplete(completed) => Some(completed.clone()),
            _ => None,
        })
        .await;
        assert_eq!(completed.error, None, "user turn should keep its answer");
    }

    let requests = mock.requests();
    assert_eq!(
        requests.len(),
        4,
        "new history should allow one new compaction"
    );
    assert!(body_contains_text(
        &requests[1].body_json().to_string(),
        SUMMARIZATION_PROMPT
    ));
    assert!(body_contains_text(
        &requests[3].body_json().to_string(),
        SUMMARIZATION_PROMPT
    ));
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[test_case::test_case("openai", "localdex", "gpt-5.5", "QB/DSV4.1-Flash"; "openai_to_localdex")]
#[test_case::test_case("localdex", "openai", "QB/DSV4.1-Flash", "gpt-5.5"; "localdex_to_openai")]
#[test_case::test_case("omnigent-localdex-test", "openai", "QB/DSV4.1-Flash", "gpt-5.5"; "namespaced_localdex_to_openai")]
#[test_case::test_case("custom", "localdex", "custom/old", "QB/DSV4.1-Flash"; "custom_to_localdex")]
#[test_case::test_case("openai", "custom", "gpt-5.5", "custom/next"; "openai_to_custom")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn previous_model_compaction_keeps_its_provider(
    old_id: &str,
    next_id: &str,
    old_model: &str,
    next_model: &str,
) -> Result<()> {
    let old_server = MockServer::start().await;
    let next_server = MockServer::start().await;
    let decoy_server = MockServer::start().await;
    let make_provider = |id: &str, server: &MockServer| {
        let mut provider = if id == "openai" {
            built_in_model_providers(None)["openai"].clone()
        } else {
            create_oss_provider_with_base_url(&format!("{}/v1", server.uri()), WireApi::Responses)
        };
        provider.base_url = Some(format!("{}/v1", server.uri()));
        provider.supports_websockets = false;
        provider
    };
    // The unused literal LocalDex endpoint catches loss of a namespaced provider ID.
    let mut configured = std::collections::HashMap::from([
        (
            LOCALDEX_PROVIDER_ID.to_string(),
            make_provider(LOCALDEX_PROVIDER_ID, &decoy_server),
        ),
        ("openai".to_string(), make_provider("openai", &decoy_server)),
    ]);
    configured.insert(old_id.to_string(), make_provider(old_id, &old_server));
    configured.insert(next_id.to_string(), make_provider(next_id, &next_server));
    let providers = merge_configured_model_providers(Default::default(), configured)
        .expect("provider normalization");
    // Hash transitions may legitimately increase short histories.
    let compact_response = if old_id == "openai" {
        remote_v2_compaction_response()
    } else {
        sse(vec![
            ev_assistant_message("summary", &"Expanded transition summary. ".repeat(4_000)),
            ev_completed_with_tokens("r2", 10),
        ])
    };
    let old_mock = mount_sse_sequence(
        &old_server,
        vec![
            sse(vec![
                ev_assistant_message("answer", "Short answer"),
                ev_completed_with_tokens("r1", 150),
            ]),
            compact_response,
        ],
    )
    .await;
    let next_mock = mount_sse_sequence(
        &next_server,
        vec![sse(vec![
            ev_assistant_message("next", "Next answer"),
            ev_completed_with_tokens("r3", 100),
        ])],
    )
    .await;
    let old_id_owned = old_id.to_string();
    let test = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_model_info_override(old_model, |info| {
            info.comp_hash = Some("old-hash".to_string());
            info.context_window = Some(273_000);
        })
        .with_model_info_override(next_model, |info| {
            info.comp_hash = Some("next-hash".to_string());
            info.context_window = Some(273_000);
        })
        .with_model(old_model)
        .with_config(move |config| {
            config.model_provider_id = old_id_owned.clone();
            config.model_provider = providers[&old_id_owned].clone();
            config.model_providers.extend(providers);
            config.model_post_turn_compact_threshold_percent = 0;
            let _ = config.features.disable(Feature::TokenBudget);
            set_test_compact_prompt(config);
        })
        .build_with_auto_env(&old_server)
        .await?;
    for (index, model) in [old_model, next_model].into_iter().enumerate() {
        let mut request = TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Answer using the selected model".to_string(),
            text_elements: vec![],
        }]);
        if index == 1 {
            request = request.with_thread_settings(ThreadSettingsOverrides {
                model: Some(model.to_string()),
                model_provider: Some(next_id.to_string()),
                ..Default::default()
            });
        }
        test.codex.start_or_steer_turn(request).await?;
        let completed = wait_for_event_match(&test.codex, |event| match event {
            EventMsg::TurnComplete(completed) => Some(completed.clone()),
            _ => None,
        })
        .await;
        assert_eq!(completed.error, None, "turn for {model} failed");
    }
    let old_requests = old_mock.requests();
    let next_requests = next_mock.requests();
    assert_eq!(
        old_requests.len(),
        2,
        "old provider receives sampling and compaction"
    );
    assert_eq!(
        next_requests.len(),
        1,
        "new provider receives only the next model request"
    );
    assert_eq!(decoy_server.received_requests().await.unwrap().len(), 0);
    for request in &old_requests {
        assert_eq!(request.body_json()["model"], old_model);
    }
    assert_eq!(next_requests[0].body_json()["model"], next_model);
    for (id, requests) in [(old_id, &old_requests), (next_id, &next_requests)] {
        if codex_model_provider_info::is_localdex_provider_id(id) {
            assert!(
                requests
                    .iter()
                    .all(|request| request.header("authorization").is_none()),
                "cached ChatGPT credentials must not reach LocalDex"
            );
        } else if id == "openai" {
            assert!(
                requests
                    .iter()
                    .all(|request| request.header("authorization").is_some())
            );
        }
    }
    test.codex.shutdown_and_wait().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cold_resume_hash_change_compacts_with_current_provider_below_budget() -> Result<()> {
    let old_server = MockServer::start().await;
    let current_server = MockServer::start().await;
    let old_mock = mount_sse_sequence(
        &old_server,
        vec![sse(vec![
            ev_assistant_message("old-answer", "Short answer"),
            ev_completed_with_tokens("r1", 100),
        ])],
    )
    .await;
    let current_mock = mount_sse_sequence(
        &current_server,
        vec![
            sse(vec![
                ev_assistant_message("summary", "Compatible summary"),
                ev_completed_with_tokens("r2", 10),
            ]),
            sse(vec![
                ev_assistant_message("answer", "Resumed answer"),
                ev_completed_with_tokens("r3", 100),
            ]),
        ],
    )
    .await;
    let mut providers = merge_configured_model_providers(
        Default::default(),
        std::collections::HashMap::from([(
            LOCALDEX_PROVIDER_ID.to_string(),
            create_oss_provider_with_base_url(
                &format!("{}/v1", current_server.uri()),
                WireApi::Responses,
            ),
        )]),
    )
    .expect("LocalDex normalization");
    let local_provider = providers.remove(LOCALDEX_PROVIDER_ID).unwrap();
    let mut initial_builder = test_codex()
        .with_model_info_override("gpt-5.5", |info| {
            info.comp_hash = Some("old-hash".to_string());
        })
        .with_model_info_override("QB/DSV4.1-Flash", |info| {
            info.comp_hash = Some("new-hash".to_string());
        })
        .with_model("gpt-5.5")
        .with_config(|config| {
            config.model_post_turn_compact_threshold_percent = 0;
        });
    let initial = initial_builder.build_with_auto_env(&old_server).await?;
    initial.submit_turn("First answer").await?;
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");
    initial.codex.shutdown_and_wait().await?;
    let catalog = initial.config.model_catalog.clone();
    let mut resumed_builder =
        test_codex()
            .with_model("QB/DSV4.1-Flash")
            .with_config(move |config| {
                config.model_catalog = catalog;
                config.model_provider_id = LOCALDEX_PROVIDER_ID.to_string();
                config.model_provider = local_provider.clone();
                config
                    .model_providers
                    .insert(LOCALDEX_PROVIDER_ID.to_string(), local_provider);
                config.model_post_turn_compact_threshold_percent = 0;
                config.model_auto_compact_token_limit = Some(100_000);
                let _ = config.features.disable(Feature::TokenBudget);
                set_test_compact_prompt(config);
            });
    let resumed = resumed_builder
        .resume(&current_server, initial.home.clone(), rollout_path)
        .await?;
    resumed
        .codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Continue after resume".to_string(),
            text_elements: vec![],
        }]))
        .await?;
    let completed = wait_for_event_match(&resumed.codex, |event| match event {
        EventMsg::TurnComplete(completed) => Some(completed.clone()),
        _ => None,
    })
    .await;
    assert_eq!(completed.error, None);
    assert_eq!(
        old_mock.requests().len(),
        1,
        "resumed compaction must not guess the old endpoint"
    );
    let requests = current_mock.requests();
    assert_eq!(
        requests.len(),
        2,
        "hash boundary requires compaction even at only 100 tokens"
    );
    assert!(
        requests
            .iter()
            .all(|request| request.body_json()["model"] == "QB/DSV4.1-Flash")
    );
    assert!(
        requests
            .iter()
            .all(|request| request.header("authorization").is_none())
    );
    assert!(body_contains_text(
        &requests[0].body_json().to_string(),
        SUMMARIZATION_PROMPT
    ));
    resumed.codex.shutdown_and_wait().await?;
    Ok(())
}
