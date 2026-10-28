//! Model-owned effort normalization shared by native requests and configuration update items.
//!
//! UI aliases resolve through the same model defaults and fallbacks in both paths.

use super::ModelInfo;
use super::ReasoningEffort;

impl ModelInfo {
    /// Resolves a selected effort to the value sent in an ordinary inference request.
    pub fn resolve_reasoning_effort(&self, effort: ReasoningEffort) -> ReasoningEffort {
        let effort = match effort {
            ReasoningEffort::Ultra => self
                .multi_agent_reasoning_effort
                .as_ref()
                .filter(|effort| {
                    *effort != &ReasoningEffort::Ultra
                        && self
                            .supported_reasoning_levels
                            .iter()
                            .any(|preset| &preset.effort == *effort)
                })
                .cloned()
                .or_else(|| {
                    self.supported_reasoning_levels
                        .iter()
                        .find(|preset| preset.effort == ReasoningEffort::Max)
                        .or_else(|| {
                            self.supported_reasoning_levels
                                .iter()
                                .rev()
                                .find(|preset| preset.effort != ReasoningEffort::Ultra)
                        })
                        .map(|preset| preset.effort.clone())
                })
                .unwrap_or(ReasoningEffort::Medium),
            // Keep "persistent" in local settings; the Responses API calls it "disabled".
            ReasoningEffort::Persistent => ReasoningEffort::Custom("disabled".to_string()),
            effort => effort,
        };
        self.clamp_effort_to_closed_ladder(effort)
    }

    /// Coerces *effort* to a rung the model accepts when its ladder declares a
    /// closed vocabulary (a `Custom` rung such as DeepSeek's `"off"`). Models
    /// whose ladders only list picker-visible presets keep verbatim requests —
    /// their endpoints accept rungs the ladder omits.
    fn clamp_effort_to_closed_ladder(&self, effort: ReasoningEffort) -> ReasoningEffort {
        let supported = &self.supported_reasoning_levels;
        let is_closed = supported
            .iter()
            .any(|preset| matches!(preset.effort, ReasoningEffort::Custom(_)));
        let matches_rung = supported.iter().any(|preset| preset.effort == effort);
        if !is_closed || matches_rung {
            return effort;
        }
        // Unranked requests (free-form budgets like a numeric Custom) pass
        // through — the closed vocab still carries free-form entries.
        let Some(requested) = effort_rank(&effort) else {
            return effort;
        };
        supported
            .iter()
            .filter_map(|preset| effort_rank(&preset.effort).map(|rank| (rank, preset)))
            // Nearest rung wins; ties go downward so the wire value never
            // spends more reasoning than the user asked for.
            .min_by_key(|(rank, _)| (rank.abs_diff(requested), *rank))
            .map(|(_, preset)| preset.effort.clone())
            .or_else(|| self.default_reasoning_level.clone())
            .unwrap_or(effort)
    }
}

/// Shared ordinal for every named rung; custom spellings that mean "off" rank
/// with `None` so a `none` request lands on a model's `off` rung.
fn effort_rank(effort: &ReasoningEffort) -> Option<usize> {
    Some(match effort {
        ReasoningEffort::None => 0,
        ReasoningEffort::Minimal => 1,
        ReasoningEffort::Low => 2,
        ReasoningEffort::Medium => 3,
        ReasoningEffort::High => 4,
        ReasoningEffort::XHigh => 5,
        ReasoningEffort::Max => 6,
        ReasoningEffort::Ultra => 7,
        ReasoningEffort::Persistent => return None,
        ReasoningEffort::Custom(value) => match value.as_str() {
            "off" | "disabled" => 0,
            _ => return None,
        },
    })
}
#[cfg(test)]
#[path = "reasoning_effort_tests.rs"]
mod tests;
