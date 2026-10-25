use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// OpenRouter provider-routing preferences, sent verbatim as the top-level
/// `provider` object on Responses (and chat completions) requests.
///
/// Configured per model provider as `[model_providers.<id>.provider]`, e.g.
/// ```toml
/// [model_providers.openrouter.provider]
/// only = ["wafer/mxfp4"]
/// allow_fallbacks = false
/// ```
///
/// Field meanings: <https://openrouter.ai/docs/guides/routing/provider-selection>
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct OpenRouterRouting {
    /// Route only to these upstream providers, e.g. `["wafer/mxfp4"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    /// Prefer providers in this order, still allowing fallbacks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Never route to these providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Restrict to these quantization levels (e.g. `["bf16", "fp8"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<String>>,
    /// Endpoint sort order: `"price"`, `"throughput"`, or `"latency"`.
    /// OpenRouter also accepts an object form, so this stays untyped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Value>,
    /// Whether OpenRouter may fall back to other providers. Set `false`
    /// together with `only`/`order` to pin an exact provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Only use providers that support every parameter in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Restrict to zero-data-retention providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zdr: Option<bool>,
    /// Only use endpoints whose terms permit text distillation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_distillable_text: Option<bool>,
    /// Whether providers may collect/store the request: `"allow"` or `"deny"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<String>,
    /// Price ceilings in USD per million tokens (or per request for that key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<HashMap<String, Value>>,
}
