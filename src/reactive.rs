//! Reactive circuit / Time-Bounded Interrupter (DOF-SPEC §5).

/// Global τ below which the system switches from deep diversification to a
/// fast deterministic pass. Normative: `5.0` seconds.
pub const FAST_PASS_THRESHOLD: f64 = 5.0;

/// Operating mode selected by the reactive circuit (§5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// τ < 5.0 s: bypass the LLM, use the deterministic fallback generator.
    FastPass,
    /// τ ≥ 5.0 s: activate the LLM-backed Generator for hidden alternatives.
    DeepDiversification,
}

impl Mode {
    /// The string used in the audit report (§6.2).
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::FastPass => "FAST_PASS",
            Mode::DeepDiversification => "DEEP_DIVERSIFICATION",
        }
    }
}

/// Decide the reactive mode from the remaining time before collapse τ
/// (§5). `τ < FAST_PASS_THRESHOLD` ⇒ [`Mode::FastPass`].
pub fn decide_mode(tau: f64) -> Mode {
    if tau < FAST_PASS_THRESHOLD {
        Mode::FastPass
    } else {
        Mode::DeepDiversification
    }
}
