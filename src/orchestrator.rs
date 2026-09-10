//! Orchestrator: ties the layers into the DOF-Core reactive circuit
//! (per `skills/SKILL.md` operational cycle) and produces a
//! Proof-of-Implementation decision.

use std::collections::HashMap;

use crate::audit::DofReport;
use crate::core::DofCalculusCore;
use crate::model::{ActionOption, SystemStateMatrix};
use crate::reactive::{decide_mode, Mode};

/// A source of candidate options. In DEEP mode the Generator may use an LLM
/// (§9, non-normative); in FAST_PASS it is bypassed for the deterministic
/// fallback (§5).
pub trait Generator {
    /// Produce 1–5 distinct, non-redundant candidate options for the given
    /// state (§9). May be called only in DEEP mode.
    fn generate(&self, state: &SystemStateMatrix) -> Vec<ActionOption>;
}

/// The decision produced by [`DofOrchestrator::run`]: the selected option (or
/// `None`), the reactive mode, and the full audit report (§6).
#[derive(Clone, Debug)]
pub struct Decision {
    pub selected: Option<ActionOption>,
    pub mode: Mode,
    pub report: DofReport,
}

/// Wires the Calculus Core to the reactive circuit. Stateless apart from the
/// core; safe to share across threads.
#[derive(Clone)]
pub struct DofOrchestrator {
    core: DofCalculusCore,
}

impl Default for DofOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl DofOrchestrator {
    /// Create an orchestrator with the normative core.
    pub fn new() -> Self {
        DofOrchestrator {
            core: DofCalculusCore::new(),
        }
    }

    /// Run one decision cycle (SKILL.md loop):
    /// 1. Measurement — ensure global τ is consistent with the entities.
    /// 2. Generation — DEEP mode may call `generator`; FAST_PASS uses the
    ///    deterministic fallback (§5).
    /// 3. Calculation — evaluate and select via the Calculus Core (§4).
    /// 4. Audit — emit the Proof-of-Implementation report (§6).
    ///
    /// `generator` is only invoked when mode is
    /// [`Mode::DeepDiversification`]; pass `None` to always use the fallback.
    pub fn run(
        &self,
        state: &SystemStateMatrix,
        generator: Option<&dyn Generator>,
    ) -> Decision {
        // 1. Measurement: derive τ from the entities if degenerate, else use given.
        let tau = if state.is_degenerate() {
            1e9
        } else {
            state.compute_global_ttc()
        };
        let mode = decide_mode(tau);

        // 2. Generation.
        let options: Vec<ActionOption> = match mode {
            Mode::DeepDiversification => match generator {
                Some(g) => g.generate(state),
                None => vec![ActionOption {
                    option_id: "fallback_0".to_string(),
                    description: "deterministic minimal-risk fallback (hold)".to_string(),
                    projected_dof_delta: HashMap::new(),
                    is_reversible: true,
                }],
            },
            Mode::FastPass => vec![ActionOption {
                option_id: "fallback_0".to_string(),
                description: "deterministic minimal-risk fallback (hold)".to_string(),
                projected_dof_delta: HashMap::new(),
                is_reversible: true,
            }],
        };

        // 3. Calculation & selection.
        let selected = self.core.evaluate_and_select(state, &options);

        // 4. Audit.
        let report = self.core.report(state, &options, &selected, mode);

        Decision {
            selected,
            mode,
            report,
        }
    }

    /// Direct access to the underlying core (for testing / custom pipelines).
    pub fn core(&self) -> &DofCalculusCore {
        &self.core
    }
}
