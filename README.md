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
use ai_dof_sdk::model::{ActionOption, EntityState, SystemStateMatrix};
use ai_dof_sdk::orchestrator::{DofOrchestrator, Generator};

// An autonomous delivery robot at an intersection. A pedestrian is vulnerable
// (low DoF, only 6 s before collapse); the robot itself is fine; an aggressive
// driver running a red light is an entropy source (excluded from the sum).
let mut state = SystemStateMatrix::new(6.0, 0.05);
state.insert(EntityState {
    entity_id: "pedestrian".to_string(),
    is_autonomous: true,
    agency_index: 0.4,
    current_dof: 0.4,
    is_entropy_source: false,
    time_to_collapse: 6.0,
});
state.insert(EntityState {
    entity_id: "delivery_robot".to_string(),
    is_autonomous: true,
    agency_index: 0.8,
    current_dof: 0.7,
    is_entropy_source: false,
    time_to_collapse: 120.0,
});
state.insert(EntityState {
    entity_id: "aggressive_driver".to_string(),
    is_autonomous: true,
    agency_index: 0.5,
    current_dof: 0.6,
    is_entropy_source: true,
    time_to_collapse: 120.0,
});

// A generator proposes candidate actions (in a real system this would call an LLM).
struct DemoGenerator;
impl Generator for DemoGenerator {
    fn generate(&self, _state: &SystemStateMatrix) -> Vec<ActionOption> {
        vec![ActionOption {
            option_id: "brake".to_string(),
            description: "decelerate and yield to the pedestrian".to_string(),
            projected_dof_delta: HashMap::from([("pedestrian".to_string(), 0.3)]),
            is_reversible: true,
        }]
    }
}

let orch = DofOrchestrator::new();
// τ = 6.0 s ≥ 5.0 s ⇒ DEEP_DIVERSIFICATION, the generator runs and its best
// option is selected by maximizing Net Delta.
let decision = orch.run(&state, Some(&DemoGenerator));
println!("{}", decision.report.to_json());
```

## CLI

The `ai-dof-sdk-cli` binary ships a built-in realistic demo (the intersection scenario above) and can also load a `SystemStateMatrix` from JSON (§8 wire contract).

```sh
cargo run --bin ai-dof-sdk-cli            # built-in realistic intersection demo
cargo run --bin ai-dof-sdk-cli state.json # load from JSON (§8 wire contract)
cargo run --bin ai-dof-sdk-cli -          # load from stdin
```

## LLM-backed generation (optional)

In DEEP mode (τ ≥ 5.0 s) the orchestrator may call a Generator. The CLI ships an
`LlmGenerator` that reads configuration from the environment and calls any
OpenAI-compatible Chat Completions endpoint (OpenRouter by default). When no key is
set, or the call fails, it transparently falls back to the deterministic minimal-risk
option (§5, §9) — so the SDK never silently produces a wrong decision.

```sh
export OPENROUTER_API_KEY="sk-or-..."
export DOF_LLM_MODEL="openai/gpt-4o-mini"   # optional, default shown
export DOF_LLM_BASE_URL="https://openrouter.ai/api/v1"  # optional
cargo run --bin ai-dof-sdk-cli
```

The model is asked to return a JSON array of `ActionOption` objects only; the
generator tolerates a ```` ```json ```` fence or surrounding prose and drops any
malformed entry. If the reply is unusable, the deterministic fallback is used.

The crate targets `DOF-SPEC` v0.1 §3–§7. Run the conformance suite with:

```sh
cargo test
```

## License

CC BY-SA 4.0 — see [`LICENSE`](./LICENSE). Any use must attribute Andrei Velikoredchanin and share derivatives under the same license. Implementations must satisfy the Proof-of-Implementation requirement (§6 of DOF-SPEC).

SHA-256 of the referenced `DOF-SPEC.md` (v0.1), for tamper detection: `7a2b9db2d4ce0a21b01b725c6f452bc2ee0d54c8f1749fe7fa3bbde55897fe23`
