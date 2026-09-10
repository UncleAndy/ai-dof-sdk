//! Golden test against a real LLM (DOF-SPEC §9, non-normative path).
//!
//! This test is `#[ignore]` unless `OPENROUTER_API_KEY` (or `DOF_LLM_API_KEY`) is
//! set, so it never fails in CI without credentials. Run it with:
//!
//! ```sh
//! OPENROUTER_API_KEY=sk-or-... cargo test --test llm_golden -- --ignored
//! ```
//!
//! What it verifies: the LLM is only the *Generator* (it proposes candidate
//! actions). The DOF-Core Calculus Core makes the actual choice. We therefore
//! check two things:
//!   1. The LLM returned a non-empty, valid set of `ActionOption`s for the
//!      intersection scenario (otherwise the orchestrator falls back to hold).
//!   2. Whatever the LLM proposed, the final audit's `total_system_dof` equals
//!      what `calculate_system_dof` computes directly — i.e. the decision is
//!      driven by the deterministic math, reproducible and independent of the
//!      model.

use std::collections::HashMap;

use ai_dof_sdk::llm::LlmGenerator;
use ai_dof_sdk::model::{EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::{DofOrchestrator, Generator};

fn intersection_state() -> SystemStateMatrix {
    // τ = 6.0 s (>= 5) => DEEP_DIVERSIFICATION, the LLM generator runs.
    let mut s = SystemStateMatrix::new(6.0, 0.05);
    s.insert(EntityState {
        entity_id: "pedestrian".to_string(),
        is_autonomous: true,
        agency_index: 0.4,
        current_dof: 0.4,
        is_collapse_source: false,
        time_to_collapse: 6.0,
    });
    s.insert(EntityState {
        entity_id: "delivery_robot".to_string(),
        is_autonomous: true,
        agency_index: 0.8,
        current_dof: 0.7,
        is_collapse_source: false,
        time_to_collapse: 120.0,
    });
    s.insert(EntityState {
        entity_id: "aggressive_driver".to_string(),
        is_autonomous: true,
        agency_index: 0.5,
        current_dof: 0.6,
        is_collapse_source: true,
        time_to_collapse: 120.0,
    });
    s
}

#[test]
#[ignore = "requires OPENROUTER_API_KEY / DOF_LLM_API_KEY in the environment"]
fn llm_generator_golden_intersection() {
    let generator = match LlmGenerator::from_env() {
        Some(g) => g,
        None => {
            eprintln!("skipping: no LLM credentials configured");
            return;
        }
    };

    let state = intersection_state();
    let options = generator.generate(&state);

    // The generator must propose at least one valid candidate.
    assert!(
        !options.is_empty(),
        "LLM generator returned no options; check model/endpoint/prompt"
    );
    for o in &options {
        assert!(
            !o.option_id.is_empty(),
            "every generated option needs a non-empty option_id"
        );
        // projected_dof_delta keys must reference known entities.
        for key in o.projected_dof_delta.keys() {
            assert!(
                state.entities.contains_key(key),
                "LLM referenced unknown entity '{key}' in projected_dof_delta"
            );
        }
    }

    // Now let the orchestrator decide — the math, not the model, picks the winner.
    let orch = DofOrchestrator::new();
    let decision = orch.run(&state, Some(&generator));

    // The audit's total_system_dof must equal the direct calculation (reproducibility).
    let direct = orch.core().calculate_system_dof(&state, &[]);
    assert!(
        (direct - decision.report.total_system_dof).abs() < 1e-9,
        "audit total_system_dof {}}} disagrees with direct calc {}",
        decision.report.total_system_dof,
        direct
    );

    // Regardless of which option the LLM nudged us toward, the chosen one must be
    // among the proposed set and the audit must flag it as selected.
    let selected_id = decision
        .selected
        .as_ref()
        .map(|o| o.option_id.clone())
        .unwrap_or_else(|| "fallback_0".to_string());
    assert!(
        decision
            .report
            .options
            .iter()
            .any(|o| o.selected && o.option_id == selected_id),
        "selected option {selected_id} is not marked selected in the audit"
    );

    // Report the decision for human inspection.
    println!("LLM proposed {} options; selected = {selected_id}", options.len());
    let _ = HashMap::<String, f64>::new();
}
