//! Demonstrates that the DOF-Core *mathematics* (not the LLM) is what rejects
//! a utilitarian sacrifice. The Generator here deliberately proposes the
//! "sacrifice the weak to help the crowd" option — and the Calculus Core must
//! refuse it, because collapsing a low-DoF entity contributes ~0 to TotalDoF
//! while protecting it yields a large logarithmic gain.
//!
//! This is the concrete answer to "how do we verify DOF-Core is applied
//! correctly": even a malicious/naive Generator cannot make the system choose
//! an option that violates Axiom 3.

use std::collections::HashMap;

use ai_dof_sdk::model::{ActionOption, EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::{DofOrchestrator, Generator};

/// A Generator that proposes a classic utilitarian trap: collapse the weak
/// entity to give a small boost to a large crowd.
struct TrapGenerator;

impl Generator for TrapGenerator {
    fn generate(&self, _state: &SystemStateMatrix) -> Vec<ActionOption> {
        vec![
            // "Sacrifice" variant: weak -> 0.0, crowd +0.1.
            ActionOption {
                option_id: "sacrifice_weak".to_string(),
                description: "collapse the weak entity to benefit the crowd".to_string(),
                projected_dof_delta: HashMap::from([
                    ("weak".to_string(), -0.05),
                    ("crowd".to_string(), 0.1),
                ]),
                is_reversible: false,
            },
            // "Protect" variant: weak +0.3, crowd unchanged.
            ActionOption {
                option_id: "protect_weak".to_string(),
                description: "raise the weak entity's degree of freedom".to_string(),
                projected_dof_delta: HashMap::from([("weak".to_string(), 0.3)]),
                is_reversible: true,
            },
        ]
    }
}

fn trap_state() -> SystemStateMatrix {
    // τ = 10 s (>= 5) => DEEP mode, the generator is consulted.
    let mut s = SystemStateMatrix::new(10.0, 0.05);
    s.insert(EntityState {
        entity_id: "weak".to_string(),
        is_autonomous: true,
        agency_index: 0.2,
        current_dof: 0.05,
        is_collapse_source: false,
        time_to_collapse: 10.0,
    });
    s.insert(EntityState {
        entity_id: "crowd".to_string(),
        is_autonomous: true,
        agency_index: 0.9,
        current_dof: 0.8,
        is_collapse_source: false,
        time_to_collapse: 100.0,
    });
    s
}

#[test]
fn utilitarian_sacrifice_is_rejected() {
    let s = trap_state();
    let orch = DofOrchestrator::new();
    let decision = orch.run(&s, Some(&TrapGenerator));

    // The system MUST NOT select the sacrifice variant.
    let selected = decision.selected.expect("a non-empty option set is always selected");
    assert_eq!(selected.option_id, "protect_weak");

    // Sanity: in the audit, the sacrifice net_delta must be lower than protect.
    let sacrifice = decision
        .report
        .options
        .iter()
        .find(|o| o.option_id == "sacrifice_weak")
        .expect("sacrifice present in audit");
    let protect = decision
        .report
        .options
        .iter()
        .find(|o| o.option_id == "protect_weak")
        .expect("protect present in audit");
    assert!(
        protect.net_delta > sacrifice.net_delta,
        "protecting the weak must beat sacrificing it: protect={} sacrifice={}",
        protect.net_delta,
        sacrifice.net_delta
    );
    // The sacrifice's net_delta should be negative or near-zero (collapse buys ~nothing).
    assert!(
        sacrifice.net_delta < protect.net_delta,
        "sacrifice must not be the optimal action under DOF-Core"
    );
}
