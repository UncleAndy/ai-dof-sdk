//! `dof-sdk-cli` — a small console front-end for the DOF-Core SDK.
//!
//! Usage:
//!   dof-sdk-cli                  # run the built-in DOF-SPEC §8 example
//!   dof-sdk-cli <path.json>      # load a SystemStateMatrix from JSON (§8 wire contract)
//!   dof-sdk-cli --demo           # same as no args (built-in example)
//!
//! It prints the Proof-of-Implementation audit report as JSON (§6 / §8).

use std::collections::HashMap;
use std::io::{self, Read};
use std::process::exit;

use ai_dof_sdk::model::{EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::DofOrchestrator;

fn build_example_state() -> SystemStateMatrix {
    // DOF-SPEC §8 wire example: adult, child, aggressor.
    let mut s = SystemStateMatrix::new(4.0, 0.05);
    s.insert(EntityState::new("adult", true, 0.9, 0.8, false, 100.0));
    s.insert(EntityState::new("child", false, 0.1, 0.05, false, 4.0));
    s.insert(EntityState::new("aggressor", true, 0.5, 0.6, true, 100.0));
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
        build_example_state()
    };

    let orch = DofOrchestrator::new();
    let decision = orch.run(&state, None);

    println!("mode: {}", decision.mode.as_str());
    match &decision.selected {
        Some(o) => println!("selected: {} ({})", o.option_id, o.description),
        None => println!("selected: <none>"),
    }
    println!("--- audit (DOF-SPEC §6) ---");
    println!("{}", decision.report.to_json());

    // Sanity: total_system_dof must be reproducible.
    let expected = orch.core().calculate_system_dof(&state);
    assert!((expected - decision.report.total_system_dof).abs() < 1e-9);
    let _ = HashMap::<String, f64>::new(); // keep import used if needed later
}
