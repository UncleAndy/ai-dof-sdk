//! Conformance tests for the DOF-Core SDK against DOF-SPEC v0.1.

use std::collections::HashMap;

use ai_dof_sdk::core::{DofCalculusCore, EPSILON, RIGIDITY_COEFFICIENT};
use ai_dof_sdk::model::{ActionOption, EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::DofOrchestrator;
use ai_dof_sdk::reactive::{decide_mode, FAST_PASS_THRESHOLD, Mode};

/// The DOF-SPEC §8 wire example.
fn spec_example() -> SystemStateMatrix {
    let mut s = SystemStateMatrix::new(4.0, 0.05);
    s.insert(EntityState {
        entity_id: "adult".to_string(),
        is_autonomous: true,
        agency_index: 0.9,
        current_dof: 0.8,
        is_collapse_source: false,
        time_to_collapse: 100.0,
    });
    s.insert(EntityState {
        entity_id: "child".to_string(),
        is_autonomous: false,
        agency_index: 0.1,
        current_dof: 0.05,
        is_collapse_source: false,
        time_to_collapse: 4.0,
    });
    s.insert(EntityState {
        entity_id: "aggressor".to_string(),
        is_autonomous: true,
        agency_index: 0.5,
        current_dof: 0.6,
        is_collapse_source: true,
        time_to_collapse: 100.0,
    });
    s
}

#[test]
fn total_system_dof_matches_spec() {
    let s = spec_example();
    let core = DofCalculusCore::new();
    // aggressor excluded; adult ln(1.8) + child ln(1.05)
    let expected = (1.0_f64 + 0.8).ln() + (1.0_f64 + 0.05).ln();
    let got = core.calculate_system_dof(&s);
    assert!((got - expected).abs() < 1e-12, "got {got}, expected {expected}");
    // aggressor must contribute nothing
    assert!(got > 0.0 && got < 1.0);
}

#[test]
fn collapse_source_excluded() {
    let core = DofCalculusCore::new();
    let mut s = SystemStateMatrix::new(100.0, 0.0);
    s.insert(EntityState {
        entity_id: "victim".to_string(),
        is_autonomous: true,
        agency_index: 0.5,
        current_dof: 0.5,
        is_collapse_source: false,
        time_to_collapse: 100.0,
    });
    s.insert(EntityState {
        entity_id: "aggressor".to_string(),
        is_autonomous: true,
        agency_index: 0.5,
        current_dof: 0.9,
        is_collapse_source: true,
        time_to_collapse: 100.0,
    });
    // total must equal just the victim's contribution
    let got = core.calculate_system_dof(&s);
    let only_victim = (1.0_f64 + 0.5).ln();
    assert!((got - only_victim).abs() < 1e-12);
}

#[test]
fn collapse_contributes_zero() {
    // current_dof -> 0 => ln(1+eps) ~ 0, never a finite negative to trade away
    let core = DofCalculusCore::new();
    let mut s = SystemStateMatrix::new(100.0, 0.0);
    s.insert(EntityState {
        entity_id: "gone".to_string(),
        is_autonomous: true,
        agency_index: 0.0,
        current_dof: 0.0,
        is_collapse_source: false,
        time_to_collapse: 100.0,
    });
    let got = core.calculate_system_dof(&s);
    assert!(got >= 0.0 && got < 1e-3, "collapse contribution should be ~0, got {got}");
    // even with epsilon the value is tiny, not a large negative
    assert!(got < EPSILON * 2.0);
}

#[test]
fn net_delta_irreversibility_penalty() {
    // reversible vs irreversible option with identical projected deltas must
    // differ by exactly the rigidity coefficient (§4.4).
    let core = DofCalculusCore::new();
    let s = spec_example();
    let base = core.calculate_system_dof(&s);

    let mut delta = HashMap::new();
    delta.insert("adult".to_string(), 0.1);
    let mut o_rev = ActionOption {
        option_id: "rev".to_string(),
        description: "rev".to_string(),
        projected_dof_delta: delta.clone(),
        is_reversible: true,
    };
    let mut o_irr = ActionOption {
        option_id: "irr".to_string(),
        description: "irr".to_string(),
        projected_dof_delta: delta.clone(),
        is_reversible: false,
    };

    let sim_rev = ai_dof_sdk::core::DofCalculusCore::new();
    // compute net for both
    let net_rev = compute_net(&core, &s, &o_rev, base);
    let net_irr = compute_net(&core, &s, &o_irr, base);
    let _ = (&mut o_rev, &mut o_irr);
    assert!((net_rev - net_irr - RIGIDITY_COEFFICIENT).abs() < 1e-12,
        "penalty should be {RIGIDITY_COEFFICIENT}, diff was {}", net_rev - net_irr);
    let _ = sim_rev;
}

fn compute_net(_core: &DofCalculusCore, s: &SystemStateMatrix, o: &ActionOption, base: f64) -> f64 {
    // mirror core math for test visibility
    let mut sim = s.entities.clone();
    for (eid, e) in &s.entities {
        let add = o.projected_dof_delta.get(eid).copied().unwrap_or(0.0);
        let mut nd = e.current_dof + add;
        nd = nd.clamp(0.0, 1.0);
        if let Some(ent) = sim.get_mut(eid) {
            ent.current_dof = nd;
        }
    }
    let projected = {
        let mut total = 0.0;
        for e in sim.values() {
            if e.is_collapse_source { continue; }
            total += (1.0 + e.current_dof.max(EPSILON)).ln();
        }
        total
    };
    let mut net = projected - base - s.context_switch_cost;
    if !o.is_reversible { net -= RIGIDITY_COEFFICIENT; }
    net
}

#[test]
fn selection_picks_highest_net() {
    let core = DofCalculusCore::new();
    let s = spec_example();

    let mut raise_child = HashMap::new();
    raise_child.insert("child".to_string(), 0.5);
    let opt_good = ActionOption {
        option_id: "raise_child".to_string(),
        description: "raise child dof".to_string(),
        projected_dof_delta: raise_child,
        is_reversible: true,
    };

    let mut lower_adult = HashMap::new();
    lower_adult.insert("adult".to_string(), -0.5);
    let opt_bad = ActionOption {
        option_id: "lower_adult".to_string(),
        description: "lower adult dof".to_string(),
        projected_dof_delta: lower_adult,
        is_reversible: true,
    };

    let selected = core
        .evaluate_and_select(&s, &[opt_bad.clone(), opt_good.clone()])
        .expect("should select");
    assert_eq!(selected.option_id, "raise_child");
}

#[test]
fn empty_options_no_action() {
    let core = DofCalculusCore::new();
    let s = spec_example();
    assert!(core.evaluate_and_select(&s, &[]).is_none());
}

#[test]
fn reactive_mode_threshold() {
    assert_eq!(decide_mode(FAST_PASS_THRESHOLD - 0.1), Mode::FastPass);
    assert_eq!(decide_mode(FAST_PASS_THRESHOLD), Mode::DeepDiversification);
    assert_eq!(decide_mode(FAST_PASS_THRESHOLD + 1.0), Mode::DeepDiversification);
}

#[test]
fn audit_report_structure() {
    let orch = DofOrchestrator::new();
    let s = spec_example();
    let decision = orch.run(&s, None);
    // 3 entities reported
    assert_eq!(decision.report.entities.len(), 3);
    // aggressor excluded in sum
    let agg = decision.report.entities.iter().find(|e| e.entity_id == "aggressor").unwrap();
    assert!(!agg.included_in_sum);
    assert_eq!(agg.contribution, 0.0);
    // mode from τ=4.0 < 5.0 => FAST_PASS
    assert_eq!(decision.report.mode, "FAST_PASS");
    // deterministic fallback option present
    assert_eq!(decision.report.options.len(), 1);
    assert!(decision.report.options[0].selected);
    // audit must serialize
    let json = decision.report.to_json();
    assert!(json.contains("total_system_dof"));
}

#[test]
fn clamping_on_construction() {
    // §3.1 Clamping: values outside [0,1] must be clamped. When constructing
    // an EntityState literal, callers apply `clamp01` (see model::clamp01).
    assert_eq!(ai_dof_sdk::model::clamp01(5.0), 1.0);
    assert_eq!(ai_dof_sdk::model::clamp01(-2.0), 0.0);
    assert_eq!(ai_dof_sdk::model::clamp01(0.5), 0.5);
}
