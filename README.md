# DOF-Core SDK — Rust

A conformant Rust implementation of the **DOF-Core** open standard. The normative contract is **`DOF-SPEC` v0.1**, published in the `DOF` open-standard repository: https://github.com/UncleAndy/DOF

This SDK lives at: https://github.com/UncleAndy/ai-dof-sdk

DOF-Core is a decision-verification protocol that separates generative creativity (the *Generator*) from deterministic mathematical validation (the *Calculus Core*). Its purpose is to maximize the system's total degrees of freedom (DoF) while structurally forbidding the destruction of any entity's DoF for local gain.

## What this crate provides

- `ai_dof_sdk` library — the full conformant implementation:
  - **`model`** — data types (`EntityState`, `SystemStateMatrix`, `ActionOption`) with serde JSON and the §3 clamping rules.
  - **`core`** — `DofCalculusCore`: `TotalDoF` (§4.1–§4.2), `NetDelta` (§4.3–§4.4), deterministic selection with tie-break (§4.5). Normative constants `EPSILON = 1e-6`, `RIGIDITY_COEFFICIENT = 0.5`.
  - **`reactive`** — the Time-Bounded Interrupter (§5): `FAST_PASS_THRESHOLD = 5.0` and `Mode` selection.
  - **`audit`** — the Proof-of-Implementation report (§6), serializable to JSON.
  - **`orchestrator`** — `DofOrchestrator` tying the layers into one decision cycle (Measurement → Generation → Calculation → Audit) per `skills/SKILL.md`.
- `ai-dof-sdk-cli` binary — load a `SystemStateMatrix` JSON (§8 wire contract) and print the audit report.

## Quick start

```rust
use std::collections::HashMap;
use ai_dof_sdk::model::{EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::DofOrchestrator;

let mut state = SystemStateMatrix::new(4.0, 0.05);
state.insert(EntityState {
    entity_id: "adult".to_string(),
    is_autonomous: true,
    agency_index: 0.9,
    current_dof: 0.8,
    is_entropy_source: false,
    time_to_collapse: 100.0,
});
state.insert(EntityState {
    entity_id: "child".to_string(),
    is_autonomous: false,
    agency_index: 0.1,
    current_dof: 0.05,
    is_entropy_source: false,
    time_to_collapse: 4.0,
});
state.insert(EntityState {
    entity_id: "aggressor".to_string(),
    is_autonomous: true,
    agency_index: 0.5,
    current_dof: 0.6,
    is_entropy_source: true,
    time_to_collapse: 100.0,
});

let orch = DofOrchestrator::new();
let decision = orch.run(&state, None);
println!("{}", decision.report.to_json());
```

## CLI

```sh
cargo run --bin ai-dof-sdk-cli            # built-in DOF-SPEC §8 example
cargo run --bin ai-dof-sdk-cli state.json # load from JSON (§8 wire contract)
```

## Conformance

The crate targets `DOF-SPEC` v0.1 §3–§7. Run the conformance suite with:

```sh
cargo test
```

## License

CC BY-SA 4.0 — see [`LICENSE`](./LICENSE). Any use must attribute Andrei Velikoredchanin and share derivatives under the same license. Implementations must satisfy the Proof-of-Implementation requirement (§6 of DOF-SPEC).

SHA-256 of the referenced `DOF-SPEC.md` (v0.1), for tamper detection: `7a2b9db2d4ce0a21b01b725c6f452bc2ee0d54c8f1749fe7fa3bbde55897fe23`
