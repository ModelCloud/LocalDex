use crate::legacy_core::config::Config;
use codex_features::Feature;
use codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::openai_models::ModelPreset;

pub(crate) fn configured_service_tier(
    config: &Config,
    notices: &codex_config::types::Notice,
) -> Option<String> {
    config.service_tier.clone().or_else(|| {
        (notices.fast_default_opt_out == Some(true))
            .then(|| SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    })
}

pub(crate) fn effective_service_tier(
    config: &Config,
    notices: &codex_config::types::Notice,
    model: &str,
    models: &[ModelPreset],
) -> Option<String> {
    let configured = configured_service_tier(config, notices);
    if configured.as_deref() == Some(ServiceTier::Flex.request_value()) {
        return configured;
    }
    if !config.features.enabled(Feature::FastMode) {
        return None;
    }

    let Some(preset) = models.iter().find(|preset| preset.model == model) else {
        return configured;
    };

    match configured.as_deref() {
        Some(service_tier) if service_tier == SERVICE_TIER_DEFAULT_REQUEST_VALUE => configured,
        Some(service_tier) if model_supports_service_tier(preset, service_tier) => configured,
        Some(_) => None,
        // The catalog advertises available tiers, but must not opt the user
        // into a paid Fast tier merely because a model is selected or resumed.
        None => None,
    }
}

pub(crate) fn service_tier_update_for_core(
    config: &Config,
    notices: &codex_config::types::Notice,
    model: &str,
    models: &[ModelPreset],
) -> Option<Option<String>> {
    let effective = effective_service_tier(config, notices, model, models);
    if let Some(service_tier) = effective {
        return Some(Some(service_tier));
    }

    if !config.features.enabled(Feature::FastMode) {
        return None;
    }

    if !models.iter().any(|preset| preset.model == model) {
        return None;
    }

    Some(Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()))
}

pub(crate) fn model_supports_service_tier(model: &ModelPreset, service_tier: &str) -> bool {
    model
        .service_tiers
        .iter()
        .any(|tier| tier.id == service_tier)
}
