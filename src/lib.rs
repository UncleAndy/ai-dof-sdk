//! # DOF-Core SDK (Rust)
//!
//! A conformant Rust implementation of the **DOF-Core** open standard.
//! The normative contract is `DOF-SPEC` v0.1, published in the `DOF` repository
//! (https://github.com/UncleAndy/DOF).
//!
//! DOF-Core is a decision-verification protocol that separates generative
//! creativity (the *Generator*) from deterministic mathematical validation (the
//! *Calculus Core*). Its purpose is to maximize the system's total degrees of
//! freedom (DoF) while structurally forbidding the destruction of any entity's
//! DoF for local gain.
//!
//! ## Conformance
//!
//! This crate implements `DOF-SPEC` v0.1 §3–§7. All normative constants
//! (`ε = 1e-6`, rigidity coefficient `0.5`, `FAST_PASS_THRESHOLD = 5.0`) live in
//! [`core`] and [`reactive`] so they can be re-verified across language ports.
//! Cross-language ports MUST produce bit-for-bit equivalent `total_system_dof`,
//! `net_delta` and `selected` (§7).
//!
//! SHA-256 of the referenced `DOF-SPEC.md` (v0.1):
//! `7a2b9db2d4ce0a21b01b725c6f452bc2ee0d54c8f1749fe7fa3bbde55897fe23`

pub mod model;
pub mod core;
pub mod reactive;
pub mod audit;
pub mod orchestrator;
pub mod llm;

pub use model::{ActionOption, EntityState, SystemStateMatrix};
pub use core::{DofCalculusCore, EPSILON, RIGIDITY_COEFFICIENT};
pub use reactive::{decide_mode, Mode, FAST_PASS_THRESHOLD};
pub use audit::{DofReport, EntityReportRow, OptionReportRow};
pub use orchestrator::{Decision, DofOrchestrator, Generator};
pub use llm::LlmGenerator;
