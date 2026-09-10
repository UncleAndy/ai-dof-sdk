//! Proof-of-Implementation audit report (DOF-SPEC §6).
//!
//! Any conforming implementation MUST be able to emit a transparent audit of
//! its decision. A silent or opaque calculation is non-conforming.

use serde::Serialize;

use crate::core::DofCalculusCore;
use crate::model::{ActionOption, SystemStateMatrix};
use crate::reactive::Mode;

/// One entity row of the audit report (§6.1).
#[derive(Clone, Debug, Serialize)]
pub struct EntityReportRow {
    pub entity_id: String,
    pub is_entropy_source: bool,
    /// `false` iff `is_entropy_source`.
    pub included_in_sum: bool,
    pub current_dof: f64,
    /// `included ? ln(1 + max(current_dof, ε)) : 0.0`.
    pub contribution: f64,
}

/// One option row of the audit report (§6.3).
#[derive(Clone, Debug, Serialize)]
pub struct OptionReportRow {
    pub option_id: String,
    pub is_reversible: bool,
    /// `TotalDoF(S')` — systemic DoF after applying the option.
    pub projected_dof: f64,
    /// `NetDelta` per §4.3–§4.4.
    pub net_delta: f64,
    pub selected: bool,
}

/// Full Proof-of-Implementation audit (§6).
#[derive(Clone, Debug, Serialize)]
pub struct DofReport {
    pub entities: Vec<EntityReportRow>,
    /// `TotalDoF(S)`.
    pub total_system_dof: f64,
    pub context_switch_cost: f64,
    pub global_time_to_collapse: f64,
    pub mode: String,
    pub options: Vec<OptionReportRow>,
}

impl DofReport {
    /// Serialize the audit to JSON for logging and verification (§6 / §8).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("DofReport is always serializable")
    }
}

impl DofCalculusCore {
    /// Build the transparent audit (§6) for a decision.
    ///
    /// - `current_state`: the observed matrix `S`.
    /// - `options`: the candidate set evaluated.
    /// - `selected`: the option chosen by [`DofCalculusCore::evaluate_and_select`]
    ///   (or `None` if the set was empty).
    /// - `mode`: the reactive-circuit mode (§5).
    pub fn report(
        &self,
        current_state: &SystemStateMatrix,
        options: &[ActionOption],
        selected: &Option<ActionOption>,
        mode: Mode,
    ) -> DofReport {
        let mut entity_rows: Vec<EntityReportRow> = Vec::new();
        for ent in current_state.entities.values() {
            let included = !ent.is_entropy_source;
            let contribution = if included {
                (1.0 + ent.current_dof.max(self.epsilon)).ln()
            } else {
                0.0
            };
            entity_rows.push(EntityReportRow {
                entity_id: ent.entity_id.clone(),
                is_entropy_source: ent.is_entropy_source,
                included_in_sum: included,
                current_dof: ent.current_dof,
                contribution,
            });
        }

        let total = self.calculate_system_dof(current_state);

        let mut option_rows: Vec<OptionReportRow> = Vec::new();
        for option in options {
            let simulated = simulate_for_report(self, current_state, option);
            let projected = self.calculate_system_dof(&simulated);
            let net = net_delta_for_report(self, current_state, option, projected, total);
            let is_selected = match selected {
                Some(s) => s.option_id == option.option_id,
                None => false,
            };
            option_rows.push(OptionReportRow {
                option_id: option.option_id.clone(),
                is_reversible: option.is_reversible,
                projected_dof: projected,
                net_delta: net,
                selected: is_selected,
            });
        }

        DofReport {
            entities: entity_rows,
            total_system_dof: total,
            context_switch_cost: current_state.context_switch_cost,
            global_time_to_collapse: current_state.global_time_to_collapse,
            mode: mode.as_str().to_string(),
            options: option_rows,
        }
    }
}

// Mirror of `DofCalculusCore::simulate` / `net_delta` for the report path so the
// audit uses exactly the same math as selection.
fn simulate_for_report(
    _core: &DofCalculusCore,
    current: &SystemStateMatrix,
    option: &ActionOption,
) -> SystemStateMatrix {
    let mut simulated = current.entities.clone();
    for (eid, e_state) in &current.entities {
        let add = option.projected_dof_delta.get(eid).copied().unwrap_or(0.0);
        let mut new_dof = e_state.current_dof + add;
        if new_dof < 0.0 {
            new_dof = 0.0;
        } else if new_dof > 1.0 {
            new_dof = 1.0;
        }
        if let Some(ent) = simulated.get_mut(eid) {
            ent.current_dof = new_dof;
        }
    }
    SystemStateMatrix {
        global_time_to_collapse: current.global_time_to_collapse,
        context_switch_cost: current.context_switch_cost,
        entities: simulated,
    }
}

fn net_delta_for_report(
    _core: &DofCalculusCore,
    current: &SystemStateMatrix,
    option: &ActionOption,
    projected: f64,
    current_dof: f64,
) -> f64 {
    let mut net = projected - current_dof - current.context_switch_cost;
    if !option.is_reversible {
        net -= crate::core::RIGIDITY_COEFFICIENT;
    }
    net
}
