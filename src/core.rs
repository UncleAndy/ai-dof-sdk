//! Core mathematics for DOF-Core (DOF-SPEC §4).

use std::collections::HashMap;

use crate::model::{ActionOption, EntityState, SystemStateMatrix};

/// Protection against `ln(0)` (DOF-SPEC §4). Normative: `ε = 1e-6`.
pub const EPSILON: f64 = 1e-6;

/// Rigidity coefficient: the structural penalty subtracted from `NetDelta`
/// when an option is irreversible (§4.4). Normative: `0.5`.
pub const RIGIDITY_COEFFICIENT: f64 = 0.5;

/// The deterministic verification layer of DOF-Core: it computes the systemic
/// DoF, simulates options, and selects the best one. It never negotiates with
/// collapse sources and never trades one entity's collapse for another's gain
/// (Axiom 3).
#[derive(Clone, Debug)]
pub struct DofCalculusCore {
    /// Protection against `ln(0)` (DOF-SPEC §4). Normative: `ε = 1e-6`.
    pub epsilon: f64,
}

impl Default for DofCalculusCore {
    fn default() -> Self {
        Self::new()
    }
}

impl DofCalculusCore {
    /// Create a core with the normative `ε = 1e-6`.
    pub fn new() -> Self {
        DofCalculusCore { epsilon: EPSILON }
    }

    /// Whether an entity belongs to the calculation set `calc` (DOF-SPEC §4.2).
    ///
    /// Excluded if it is a collapse source, OR if its `current_dof <= 0` and no available
    /// option can raise its DoF (a node with no recovery path). A node at `DoF = 0` that
    /// *can* be revived stays in the set — excluding it would let the system ignore a
    /// salvageable being (Axiom 3).
    pub fn is_included(&self, entity: &EntityState, options: &[ActionOption]) -> bool {
        if entity.is_collapse_source {
            return false;
        }
        if entity.current_dof > 0.0 {
            return true;
        }
        // current_dof == 0 (or <= epsilon): keep only if some option can revive it
        for opt in options {
            if opt
                .projected_dof_delta
                .get(&entity.entity_id)
                .copied()
                .unwrap_or(0.0)
                > 0.0
            {
                return true;
            }
        }
        false
    }

    /// Total System DoF Evaluation Index (§4.1): the pure Nash product — sum of
    /// `ln(DoF)` over the calculation set `calc` (§4.2).
    ///
    /// The aggregate is an *evaluation index*, not an absolute measure: its values are
    /// negative and only their ordering matters for comparing options. Because the sum is
    /// over `ln(DoF)`, driving a revivable entity to `DoF = 0` contributes `ln(ε) ≈ −13.8`
    /// (a finite floor against `−∞`), an enormous penalty that makes any option collapsing
    /// such an entity dominated by options that spare or revive it (deontological veto on
    /// creating collapse, Axiom 3). A literal product would give `−∞`; the ε-floor keeps
    /// the index finite and comparable.
    pub fn calculate_system_dof(&self, state: &SystemStateMatrix, options: &[ActionOption]) -> f64 {
        let mut total = 0.0;
        for entity in state.entities.values() {
            if !self.is_included(entity, options) {
                continue;
            }
            let dof = entity.current_dof.max(self.epsilon);
            total += dof.ln();
        }
        total
    }

    /// Simulate an option's projected deltas into a new state, clamping every
    /// entity's `current_dof` to `[0.0, 1.0]` (§4.3). `global_time_to_collapse`
    /// and `context_switch_cost` are preserved.
    fn simulate(&self, current: &SystemStateMatrix, option: &ActionOption) -> SystemStateMatrix {
        let mut simulated: HashMap<String, EntityState> = current.entities.clone();
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

    /// Net Delta (§4.3–§4.4): `TotalDoF(S') - TotalDoF(S) - ΔT`, minus the
    /// rigidity penalty if the option is irreversible.
    fn net_delta(
        &self,
        current: &SystemStateMatrix,
        option: &ActionOption,
        projected: f64,
        current_dof: f64,
    ) -> f64 {
        let mut net = projected - current_dof - current.context_switch_cost;
        if !option.is_reversible {
            net -= RIGIDITY_COEFFICIENT;
        }
        net
    }

    /// Evaluate all options and select the one maximizing `NetDelta` (§4.5).
    ///
    /// Ties are broken deterministically by `option_id` lexicographic order
    /// (§4.5). If the option set is empty, returns `None` (no action).
    ///
    /// This function does **not** modify Axiom-3 semantics: it never consults
    /// an external "greater good" utility metric (§7.6).
    pub fn evaluate_and_select(
        &self,
        current_state: &SystemStateMatrix,
        options: &[ActionOption],
    ) -> Option<ActionOption> {
        if options.is_empty() {
            return None;
        }
        let current = self.calculate_system_dof(current_state, options);
        let mut best: Option<ActionOption> = None;
        let mut max_net: f64 = f64::NEG_INFINITY;
        let mut best_id: String = String::new();

        for option in options {
            let simulated = self.simulate(current_state, option);
            let projected = self.calculate_system_dof(&simulated, options);
            let net = self.net_delta(current_state, option, projected, current);
            let take = if net > max_net {
                true
            } else if (net - max_net).abs() < f64::EPSILON && option.option_id < best_id {
                // deterministic tie-break: smaller option_id wins
                true
            } else {
                false
            };
            if take {
                max_net = net;
                best_id = option.option_id.clone();
                best = Some(option.clone());
            }
        }
        best
    }
}
