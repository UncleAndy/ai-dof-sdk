//! LLM-backed Generator (DOF-SPEC §9, non-normative).
//!
//! Reads configuration from the environment and calls an OpenAI-compatible
//! Chat Completions endpoint (OpenRouter by default). When no API key is set,
//! or the call fails, it yields no options so the orchestrator falls back to
//! the deterministic minimal-risk option (§9: "MUST fall back ... on failure").
//!
//! Environment variables:
//! - `OPENROUTER_API_KEY` (or `DOF_LLM_API_KEY`) — bearer token. If absent, the
//!   generator is inactive and the orchestrator uses its deterministic fallback.
//! - `DOF_LLM_MODEL` — model id (default: `openai/gpt-4o-mini`).
//! - `DOF_LLM_BASE_URL` — API base (default: `https://openrouter.ai/api/v1`).

use std::collections::HashMap;
use std::time::Duration;

use serde_json::Value;

use crate::model::{ActionOption, SystemStateMatrix};
use crate::orchestrator::Generator;

/// An LLM-backed generator. Construct via [`LlmGenerator::from_env`]; if no API
/// key is present it reports `active() == false` and produces no options.
pub struct LlmGenerator {
    api_key: String,
    model: String,
    base_url: String,
    timeout: Duration,
}

impl LlmGenerator {
    /// Build from the environment. Returns `None` when no API key is configured
    /// (so callers can transparently fall back to a deterministic generator).
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .or_else(|_| std::env::var("DOF_LLM_API_KEY"))
            .ok()?;
        let model = std::env::var("DOF_LLM_MODEL").unwrap_or_else(|_| "openai/gpt-4o-mini".to_string());
        let base_url = std::env::var("DOF_LLM_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
        Some(LlmGenerator {
            api_key,
            model,
            base_url,
            timeout: Duration::from_secs(20),
        })
    }

    /// True iff an API key was configured.
    pub fn active(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// The model id that will be requested.
    pub fn model(&self) -> &str {
        &self.model
    }

    fn build_prompt(&self, state: &SystemStateMatrix) -> String {
        let state_json = serde_json::to_string_pretty(state).unwrap_or_default();
        format!(
            "You are the Synthesis layer of a DOF-Core decision system. \
Given the current SystemStateMatrix (JSON), propose 1-4 distinct, non-redundant \
candidate actions. Each action must be a JSON object with exactly these fields:\n\
- \"option_id\": short unique string\n\
- \"description\": one human-readable sentence\n\
- \"projected_dof_delta\": object mapping entity_id -> predicted change in that \
entity's current_dof (float, range about [-1.0, 1.0]); only include entities \
whose DoF you expect to change\n\
- \"is_reversible\": boolean (false only for irreversible physical actions)\n\n\
Never command actuators directly. Do not negotiate with entropy sources. \
Return ONLY a JSON array of these objects, no prose, no markdown.\n\n\
SystemStateMatrix:\n{state_json}"
        )
    }

    /// Parse the model's reply into options. Tolerant of a ```json fence or
    /// surrounding prose: it finds the first '[' ... last ']'.
    fn parse_options(&self, content: &str) -> Vec<ActionOption> {
        let trimmed = content.trim();
        let slice = if let (Some(s), Some(e)) = (trimmed.find('['), trimmed.rfind(']')) {
            &trimmed[s..=e]
        } else {
            trimmed
        };
        let raw: Value = match serde_json::from_str::<Value>(slice) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let arr = match raw {
            Value::Array(a) => a,
            _ => return Vec::new(),
        };
        let mut out = Vec::new();
        for item in arr {
            if let Some(opt) = parse_one(&item) {
                out.push(opt);
            }
        }
        out
    }
}

/// Deserialize one `ActionOption`, tolerating both `f64` and string numbers and
/// a map or object for `projected_dof_delta`.
fn parse_one(v: &Value) -> Option<ActionOption> {
    let obj = v.as_object()?;
    let option_id = obj.get("option_id")?.as_str()?.to_string();
    let description = obj
        .get("description")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let is_reversible = obj
        .get("is_reversible")
        .and_then(|x| x.as_bool())
        .unwrap_or(true);
    let mut projected = HashMap::new();
    if let Some(delta) = obj.get("projected_dof_delta") {
        if let Some(map) = delta.as_object() {
            for (k, val) in map {
                if let Some(f) = val.as_f64() {
                    projected.insert(k.clone(), f);
                } else if let Some(s) = val.as_str() {
                    if let Ok(f) = s.parse::<f64>() {
                        projected.insert(k.clone(), f);
                    }
                }
            }
        }
    }
    Some(ActionOption {
        option_id,
        description,
        projected_dof_delta: projected,
        is_reversible,
    })
}

impl Generator for LlmGenerator {
    /// Call the LLM and return its proposed options. On any error (missing key,
    /// network failure, malformed reply) returns an empty vector so the
    /// orchestrator falls back to the deterministic option (§9).
    fn generate(&self, state: &SystemStateMatrix) -> Vec<ActionOption> {
        let prompt = self.build_prompt(state);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "You output only valid JSON arrays of action objects."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.2,
            "max_tokens": 800
        });

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let agent = ureq::AgentBuilder::new().timeout(self.timeout).build();
        let body_str = match serde_json::to_string(&body) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let resp = match agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_string(&body_str)
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let value: Value = match resp.into_string() {
            Ok(s) => serde_json::from_str(&s).unwrap_or(Value::Null),
            Err(_) => return Vec::new(),
        };
        let content = value
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|m| m.as_str())
            .unwrap_or("");
        self.parse_options(content)
    }
}
