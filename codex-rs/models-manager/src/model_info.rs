use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelMessages;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::ToolMode;
use codex_protocol::openai_models::TruncationMode;
use codex_protocol::openai_models::TruncationPolicyConfig;
use codex_protocol::openai_models::WebSearchToolType;
use codex_protocol::openai_models::default_input_modalities;

use crate::config::ModelsManagerConfig;
use codex_utils_output_truncation::approx_bytes_for_tokens;
use tracing::warn;

pub const BASE_INSTRUCTIONS: &str = include_str!("../prompt.md");
const PERSONALITY_SECTION_HEADER: &str = "# Personality";
const LOCALDEX_DSV41_FLASH: &str = "QB/DSV4.1-Flash";

pub fn with_config_overrides(mut model: ModelInfo, config: &ModelsManagerConfig) -> ModelInfo {
    if let Some(context_window) = config.model_context_window {
        model.context_window = Some(
            model
                .max_context_window
                .map_or(context_window, |max_context_window| {
                    context_window.min(max_context_window)
                }),
        );
    }
    if let Some(auto_compact_token_limit) = config.model_auto_compact_token_limit {
        model.auto_compact_token_limit = Some(auto_compact_token_limit);
    }
    if let Some(token_limit) = config.tool_output_token_limit {
        model.truncation_policy = match model.truncation_policy.mode {
            TruncationMode::Bytes => {
                let byte_limit =
                    i64::try_from(approx_bytes_for_tokens(token_limit)).unwrap_or(i64::MAX);
                TruncationPolicyConfig::bytes(byte_limit)
            }
            TruncationMode::Tokens => {
                let limit = i64::try_from(token_limit).unwrap_or(i64::MAX);
                TruncationPolicyConfig::tokens(limit)
            }
        };
    }

    if let Some(base_instructions) = &config.base_instructions {
        let model_messages = model.model_messages.get_or_insert_default();
        model_messages.instructions_template = Some(base_instructions.clone());
        model_messages.instructions_variables = None;
    } else if config.personality == Some(Personality::None)
        && let Some(instructions_template) = model
            .model_messages
            .as_mut()
            .and_then(|messages| messages.instructions_template.as_mut())
    {
        *instructions_template = strip_personality_section(std::mem::take(instructions_template));
    }

    model
}

fn strip_personality_section(mut instructions: String) -> String {
    let mut section_start = None;
    let mut section_end = None;
    let mut offset = 0;

    for line_with_ending in instructions.split_inclusive('\n') {
        let line = match line_with_ending.strip_suffix('\n') {
            Some(line) => line.strip_suffix('\r').unwrap_or(line),
            None => line_with_ending,
        };
        if section_start.is_some() {
            if is_h1_heading(line) {
                section_end = Some(offset);
                break;
            }
        } else if line == PERSONALITY_SECTION_HEADER {
            section_start = Some(offset);
        }
        offset += line_with_ending.len();
    }

    if let Some(section_start) = section_start {
        let section_end = section_end.unwrap_or(instructions.len());
        instructions.replace_range(section_start..section_end, "");
    }

    instructions
}

fn is_h1_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('#') else {
        return false;
    };
    rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')
}

/// Build a minimal fallback model descriptor for missing/unknown slugs.
pub fn model_info_from_slug(slug: &str) -> ModelInfo {
    if slug == LOCALDEX_DSV41_FLASH {
        return localdex_dsv41_flash_model_info();
    }
    warn!("Unknown model {slug} is used. This will use fallback model metadata.");
    ModelInfo {
        slug: slug.to_string(),
        display_name: slug.to_string(),
        description: None,
        default_reasoning_level: None,
        supported_reasoning_levels: Vec::new(),
        shell_type: ConfigShellToolType::UnifiedExec,
        visibility: ModelVisibility::None,
        supported_in_api: true,
        priority: 99,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        available_access_programs: None,
        availability_nux: None,
        upgrade: None,
        model_messages: Some(local_model_messages()),
        include_skills_usage_instructions: false,
        include_plugin_usage_instructions: false,
        include_apps_usage_instructions: false,
        supports_reasoning_summary_parameter: true,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        apply_patch_tool_type: None,
        web_search_tool_type: WebSearchToolType::Text,
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_image_detail_original: false,
        context_window: Some(272_000),
        max_context_window: Some(272_000),
        auto_compact_token_limit: None,
        comp_hash: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: default_input_modalities(),
        used_fallback_model_metadata: true, // this is the fallback model metadata
        supports_search_tool: false,
        supports_experimental_context: false,
        use_responses_lite: false,
        guardian: None,
        node_repl_auto_review_required: false,
        node_repl_disabled: false,
        auto_review_model_override: None,
        model_specialty: None,
        tool_mode: None,
        multi_agent_version: None,
        multi_agent_reasoning_effort: None,
    }
}

/// Metadata for the LocalDex provider's bundled DeepSeek endpoint.
///
/// This is intentionally part of LocalDex rather than a `model_catalog_json`
/// overlay: Codex treats that setting as a replacement for its full catalog,
/// which conflicts with the upstream ChatGPT model catalog and configured
/// provider routing. The inference server advertises matching capabilities
/// through its OpenAI-compatible model-discovery endpoint.
fn localdex_dsv41_flash_model_info() -> ModelInfo {
    ModelInfo {
        slug: LOCALDEX_DSV41_FLASH.to_string(),
        display_name: "DeepSeek V4.1 Flash".to_string(),
        description: Some("Local DeepSeek V4.1 Flash coding model.".to_string()),
        // DSV4.1 accepts `low`, `high`, `max`, or a numeric budget. Its
        // OpenAI-compatible endpoint explicitly rejects Codex's generic
        // `medium` default, so use a valid default even outside Omnigent.
        default_reasoning_level: Some(ReasoningEffort::High),
        supported_reasoning_levels: vec![
            ReasoningEffortPreset {
                effort: ReasoningEffort::Low,
                description: "Fast responses with lighter reasoning".to_string(),
            },
            ReasoningEffortPreset {
                effort: ReasoningEffort::High,
                description: "Greater reasoning depth for complex work".to_string(),
            },
            ReasoningEffortPreset {
                effort: ReasoningEffort::Max,
                description: "Maximum reasoning depth for difficult work".to_string(),
            },
        ],
        shell_type: ConfigShellToolType::UnifiedExec,
        visibility: ModelVisibility::None,
        supported_in_api: true,
        priority: 99,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        available_access_programs: None,
        availability_nux: None,
        upgrade: None,
        model_messages: Some(local_model_messages()),
        // LocalDex uses the same skill catalog and bridge as Codex. Include
        // the usage protocol so a local model knows to inspect a selected
        // SKILL.md before it acts on that skill.
        include_skills_usage_instructions: true,
        include_plugin_usage_instructions: false,
        include_apps_usage_instructions: false,
        // The local endpoint streams raw reasoning text. It does not expose a
        // distinct reasoning-summary stream, so never ask it for one: some
        // OpenAI-compatible servers mirror a requested summary alongside the
        // raw stream, which duplicates/interleaves visible thinking.
        supports_reasoning_summary_parameter: false,
        default_reasoning_summary: ReasoningSummary::None,
        support_verbosity: false,
        default_verbosity: None,
        apply_patch_tool_type: Some(ApplyPatchToolType::Freeform),
        web_search_tool_type: WebSearchToolType::Text,
        truncation_policy: TruncationPolicyConfig::tokens(/*limit*/ 10_000),
        supports_image_detail_original: false,
        context_window: Some(524_288),
        max_context_window: Some(524_288),
        auto_compact_token_limit: Some(471_859),
        comp_hash: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: vec![InputModality::Text],
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        supports_experimental_context: false,
        use_responses_lite: false,
        guardian: None,
        node_repl_auto_review_required: false,
        node_repl_disabled: false,
        auto_review_model_override: None,
        model_specialty: None,
        tool_mode: Some(ToolMode::Direct),
        multi_agent_version: None,
        multi_agent_reasoning_effort: None,
    }
}

fn local_model_messages() -> ModelMessages {
    ModelMessages {
        instructions_template: Some(BASE_INSTRUCTIONS.to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "model_info_tests.rs"]
mod tests;
