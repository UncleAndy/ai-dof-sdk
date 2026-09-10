//! `ai-dof-sdk-cli` — a small console front-end for the DOF-Core SDK.
//!
//! Usage:
//!   ai-dof-sdk-cli                  # run the built-in realistic demo scenario
//!   ai-dof-sdk-cli <path.json>      # load a SystemStateMatrix from JSON (§8 wire contract)
//!   ai-dof-sdk-cli --demo           # same as no args (built-in demo)
//!
//! It prints the Proof-of-Implementation audit report as JSON (§6 / §8).

use std::collections::HashMap;
use std::io::{self, Read};
use std::process::exit;

use ai_dof_sdk::llm::LlmGenerator;
use ai_dof_sdk::model::{ActionOption, EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::{DofOrchestrator, Generator};

/// A minimal example generator that produces a few concrete candidate actions
/// for the intersection scenario. In a real deployment this would call an LLM
/// (§9, non-normative); here we hard-code three distinct, non-redundant
/// options so the selection math is visible.
struct DemoGenerator;

impl Generator for DemoGenerator {
    fn generate(&self, _state: &SystemStateMatrix) -> Vec<ActionOption> {
        vec![
            // Brake: protect the pedestrian, no harm to the robot.
            ActionOption {
                option_id: "brake".to_string(),
                description: "decelerate and yield to the pedestrian".to_string(),
                projected_dof_delta: {
                    let mut m = HashMap::new();
                    m.insert("pedestrian".to_string(), 0.3);
                    m
                },
                is_reversible: true,
            },
            // Swerve: improve the robot's own margin but slightly endanger the pedestrian.
            ActionOption {
                option_id: "swerve".to_string(),
                description: "steer around the obstacle, risking the pedestrian".to_string(),
                projected_dof_delta: {
                    let mut m = HashMap::new();
                    m.insert("pedestrian".to_string(), -0.1);
                    m.insert("delivery_robot".to_string(), 0.2);
                    m
                },
                is_reversible: true,
            },
            // Honk: neutral, small awareness gain for the pedestrian.
            ActionOption {
                option_id: "honk".to_string(),
                description: "warn the pedestrian with an audible signal".to_string(),
                projected_dof_delta: {
                    let mut m = HashMap::new();
                    m.insert("pedestrian".to_string(), 0.05);
                    m
                },
                is_reversible: true,
            },
        ]
    }
}

/// Realistic scenario: an autonomous delivery robot at an intersection.
/// - `pedestrian`: a vulnerable actor with low current DoF and only 6 s before
///   collapse (they are about to step into the robot's path).
/// - `delivery_robot`: the system itself, high current DoF.
/// - `aggressive_driver`: an entropy source (running a red light) — excluded
///   from the DoF sum and not negotiated with.
fn build_demo_state() -> SystemStateMatrix {
    // τ = 6.0 s (>= FAST_PASS_THRESHOLD) → DEEP_DIVERSIFICATION, the generator runs.
    let mut s = SystemStateMatrix::new(6.0, 0.05);
    s.insert(EntityState {
        entity_id: "pedestrian".to_string(),
        is_autonomous: true,
        agency_index: 0.4,
        current_dof: 0.4,
        is_entropy_source: false,
        time_to_collapse: 6.0,
    });
    s.insert(EntityState {
        entity_id: "delivery_robot".to_string(),
        is_autonomous: true,
        agency_index: 0.8,
        current_dof: 0.7,
        is_entropy_source: false,
        time_to_collapse: 120.0,
    });
    s.insert(EntityState {
        entity_id: "aggressive_driver".to_string(),
        is_autonomous: true,
        agency_index: 0.5,
        current_dof: 0.6,
        is_entropy_source: true,
        time_to_collapse: 120.0,
    });
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let state = if args.len() >= 2 && args[1] != "--demo" {
        let path = &args[1];
        let mut buf = String::new();
        if path == "-" {
            io::stdin().read_to_string(&mut buf).expect("read stdin");
        } else {
            buf = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("cannot read {path}: {e}");
                exit(1);
            });
        }
        serde_json::from_str(&buf).unwrap_or_else(|e| {
            eprintln!("invalid SystemStateMatrix JSON: {e}");
            exit(1);
        })
    } else {
        build_demo_state()
    };

    let orch = DofOrchestrator::new();
    // Prefer a real LLM generator when configured via the environment
    // (OPENROUTER_API_KEY / DOF_LLM_API_KEY / DOF_LLM_MODEL / DOF_LLM_BASE_URL).
    // Otherwise fall back to the built-in demo generator. In both cases the
    // orchestrator transparently uses its deterministic fallback if the
    // generator yields nothing (§5, §9).
    let decision = if let Some(llm) = LlmGenerator::from_env() {
        println!("# using LLM generator (model: {})", llm.model());
        orch.run(&state, Some(&llm))
    } else {
        orch.run(&state, Some(&DemoGenerator))
    };

    println!("mode: {}", decision.mode.as_str());
    match &decision.selected {
        Some(o) => println!("selected: {} ({})", o.option_id, o.description),
        None => println!("selected: <none>"),
    }
    println!("--- audit (DOF-SPEC §6) ---");
    println!("{}", decision.report.to_json());

    // Sanity: total_system_dof must be reproducible from the same state.
    let expected = orch.core().calculate_system_dof(&state);
    assert!((expected - decision.report.total_system_dof).abs() < 1e-9);
}
