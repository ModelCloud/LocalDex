use super::*;
use crate::create_model_provider;
use codex_model_provider_info::ModelProviderCapabilities;
use pretty_assertions::assert_eq;

#[test]
fn capability_overrides_preserve_unspecified_provider_defaults() {
    for info in [
        ModelProviderInfo::default(),
        ModelProviderInfo::create_openai_provider(/*base_url*/ None),
        ModelProviderInfo {
            name: "Azure".to_string(),
            ..Default::default()
        },
    ] {
        let defaults = create_model_provider(info.clone(), /*auth_manager*/ None).capabilities();
        let provider = create_model_provider(
            ModelProviderInfo {
                capabilities: Some(ModelProviderCapabilities {
                    external_web_access: Some(false),
                    ..Default::default()
                }),
                ..info
            },
            /*auth_manager*/ None,
        );
        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities {
                external_web_access: false,
                ..defaults
            },
        );
    }
}

#[test]
fn generic_provider_preserves_upstream_capabilities_without_openai_auth() {
    for name in ["custom", "Azure", "LocalDex display name only"] {
        let info = ModelProviderInfo {
            name: name.to_string(),
            requires_openai_auth: false,
            ..Default::default()
        };
        let expected_remote_compaction = if name == "Azure" {
            RemoteCompactionSupport::V2
        } else {
            RemoteCompactionSupport::Unsupported
        };
        assert_eq!(
            create_model_provider(info, /*auth_manager*/ None).capabilities(),
            ProviderCapabilities {
                remote_compaction: expected_remote_compaction,
                ..ProviderCapabilities::default()
            },
        );
    }
}
