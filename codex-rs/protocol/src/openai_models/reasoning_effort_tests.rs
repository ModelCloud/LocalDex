use super::super::ReasoningEffortPreset;
use super::super::tests::test_model;
use super::ReasoningEffort;
use pretty_assertions::assert_eq;

#[test]
fn resolves_ultra_using_model_override_and_supported_fallbacks() {
    use ReasoningEffort::High;
    use ReasoningEffort::Low;
    use ReasoningEffort::Max;
    use ReasoningEffort::Medium;
    use ReasoningEffort::Ultra;
    use ReasoningEffort::XHigh;

    for (supported, multi_agent_effort, expected) in [
        (vec![High, Max, Ultra], Some(High), High),
        (vec![High, Max, Ultra], None, Max),
        (vec![High, XHigh, Ultra], None, XHigh),
        (vec![High, XHigh, Ultra], Some(Ultra), XHigh),
        (vec![High, XHigh, Ultra], Some(Low), XHigh),
        (vec![], None, Medium),
    ] {
        let mut model = test_model(/*spec*/ None);
        model.supported_reasoning_levels = supported
            .into_iter()
            .map(|effort| ReasoningEffortPreset {
                effort,
                description: String::new(),
            })
            .collect();
        model.multi_agent_reasoning_effort = multi_agent_effort;
        assert_eq!(model.resolve_reasoning_effort(Ultra), expected);
    }
}

#[test]
fn preserves_native_efforts_and_translates_persistent_mode() {
    let model = test_model(/*spec*/ None);
    for effort in [
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Max,
        ReasoningEffort::Custom("future-effort".to_string()),
    ] {
        assert_eq!(model.resolve_reasoning_effort(effort.clone()), effort);
    }
    assert_eq!(
        model.resolve_reasoning_effort(ReasoningEffort::Persistent),
        ReasoningEffort::Custom("disabled".to_string()),
    );
}

fn deepseek_style_model() -> super::super::ModelInfo {
    let mut model = test_model(/*spec*/ None);
    // DeepSeek v4.1's wire vocabulary: a `Custom` rung marks the ladder closed.
    model.supported_reasoning_levels = [
        ReasoningEffort::Custom("off".to_string()),
        ReasoningEffort::Low,
        ReasoningEffort::High,
        ReasoningEffort::Max,
    ]
    .into_iter()
    .map(|effort| ReasoningEffortPreset {
        effort,
        description: String::new(),
    })
    .collect();
    model.default_reasoning_level = Some(ReasoningEffort::Max);
    model
}

#[test]
fn clamps_to_closed_vocabulary_when_ladder_declares_custom_rungs() {
    let model = deepseek_style_model();
    for (requested, expected) in [
        // OpenAI-only rungs left over by a swap or resume land on the nearest
        // native rung, ties downward.
        (ReasoningEffort::XHigh, ReasoningEffort::High),
        (ReasoningEffort::Medium, ReasoningEffort::Low),
        (
            ReasoningEffort::Minimal,
            ReasoningEffort::Custom("off".to_string()),
        ),
        (
            ReasoningEffort::None,
            ReasoningEffort::Custom("off".to_string()),
        ),
        // "disabled"/"off" spellings unify onto the model's off rung.
        (
            ReasoningEffort::Persistent,
            ReasoningEffort::Custom("off".to_string()),
        ),
        (
            ReasoningEffort::Custom("disabled".to_string()),
            ReasoningEffort::Custom("off".to_string()),
        ),
        // Ultra resolves to the model's top rung, which is a valid rung.
        (ReasoningEffort::Ultra, ReasoningEffort::Max),
        // Exact rungs and free-form budgets pass through untouched.
        (ReasoningEffort::High, ReasoningEffort::High),
        (
            ReasoningEffort::Custom("1024".to_string()),
            ReasoningEffort::Custom("1024".to_string()),
        ),
    ] {
        assert_eq!(model.resolve_reasoning_effort(requested), expected);
    }
}

#[test]
fn open_vocabulary_ladders_keep_verbatim_requests() {
    // Enum-only ladders list picker presets, not the wire's full vocabulary;
    // those endpoints accept rungs the ladder omits.
    let mut model = test_model(/*spec*/ None);
    model.supported_reasoning_levels = [ReasoningEffort::Low, ReasoningEffort::High]
        .into_iter()
        .map(|effort| ReasoningEffortPreset {
            effort,
            description: String::new(),
        })
        .collect();
    for effort in [
        ReasoningEffort::None,
        ReasoningEffort::Minimal,
        ReasoningEffort::Medium,
        ReasoningEffort::XHigh,
    ] {
        assert_eq!(model.resolve_reasoning_effort(effort.clone()), effort);
    }
}
