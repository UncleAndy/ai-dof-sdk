//! Data model for DOF-Core (DOF-SPEC §3).
//!
//! Field names, types and clamping rules are normative. Implementations in
//! other languages MUST preserve them so that cross-language ports produce
//! bit-for-bit equivalent `total_system_dof`, `net_delta` and `selected`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// One observable node in the system (DOF-SPEC §3.1).
///
/// `agency_index` and `current_dof` are clamped to `[0.0, 1.0]` on
/// construction (§3.1 Clamping). An entity with `current_dof == 0.0` is at
/// collapse.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityState {
    /// Stable identifier of the node.
    pub entity_id: String,
    /// Whether the entity controls its own actions.
    pub is_autonomous: bool,
    /// Measure of controllability / self-direction, clamped to `[0.0, 1.0]`.
    pub agency_index: f64,
    /// Current degree of freedom of the node, clamped to `[0.0, 1.0]`.
    /// `0.0` = collapse.
    pub current_dof: f64,
    /// If `true`, the entity is a destructive aggressor — a *collapse source*
    /// whose actions reduce others' DoF (see §4.2). Excluded from `TotalDoF`.
    pub is_collapse_source: bool,
    /// Local deadline before this node collapses, in seconds (`> 0`).
    pub time_to_collapse: f64,
}

impl EntityState {
    /// True iff this entity is at collapse (§3.1).
    pub fn is_collapsed(&self) -> bool {
        self.current_dof <= 0.0
    }
}

/// The full set of observed entities plus global timing/penalty state
/// (DOF-SPEC §3.2).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SystemStateMatrix {
    /// Global τ — most urgent non-collapse-source deadline (see §5).
    pub global_time_to_collapse: f64,
    /// ΔT — penalty for changing the current process.
    pub context_switch_cost: f64,
    /// The full set of observed entities, keyed by `entity_id`.
    #[serde(default)]
    pub entities: HashMap<String, EntityState>,
}

impl SystemStateMatrix {
    /// Construct an empty matrix with the given global τ and ΔT.
    pub fn new(global_time_to_collapse: f64, context_switch_cost: f64) -> Self {
        SystemStateMatrix {
            global_time_to_collapse,
            context_switch_cost,
            entities: HashMap::new(),
        }
    }

    /// Insert or replace an entity.
    pub fn insert(&mut self, entity: EntityState) {
        self.entities.insert(entity.entity_id.clone(), entity);
    }

    /// Compute `global_time_to_collapse` as the **minimum** `time_to_collapse`
    /// over all entities where `is_collapse_source == false` (§3.2). If no
    /// such entity exists, returns a safe large value (`1e9`) and flags the
    /// degenerate state via [`SystemStateMatrix::is_degenerate`].
    pub fn compute_global_ttc(&self) -> f64 {
        let mut min = f64::INFINITY;
        for e in self.entities.values() {
            if e.is_collapse_source {
                continue;
            }
            if e.time_to_collapse < min {
                min = e.time_to_collapse;
            }
        }
        if min.is_finite() {
            min
        } else {
            1e9
        }
    }

    /// True if there are no non-collapse-source entities (so global τ is degenerate).
    pub fn is_degenerate(&self) -> bool {
        !self.entities.values().any(|e| !e.is_collapse_source)
    }
}

/// A candidate plan produced by the Generator (DOF-SPEC §3.3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ActionOption {
    /// Stable identifier of the candidate plan.
    pub option_id: String,
    /// Human/agent-readable summary.
    pub description: String,
    /// Forecast change of `current_dof` per entity.
    pub projected_dof_delta: HashMap<String, f64>,
    /// `false` ⇒ irreversible ⇒ structural penalty (§4.4).
    pub is_reversible: bool,
}

/// Clamp a value to `[0.0, 1.0]` (§3.1 Clamping). Exposed so that callers
/// constructing an `EntityState` literal can normalize raw inputs.
pub fn clamp01(x: f64) -> f64 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}
