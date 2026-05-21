//! LLM provider — stage 1 ships SubscriptionClaude only.
//!
//! Routing (§4) decides which provider to use; this module just executes.
//! Timeout, retry, and budget enforcement live one layer up (executor + §6 contract).

use anyhow::{Context, Result};
use std::process::Command;

pub trait LlmProvider {
    fn complete(&self, prompt: &str) -> Result<String>;
    fn name(&self) -> &'static str;
}

/// Shells `claude -p --output-format=json <prompt>` and pulls `result` out of the JSON.
/// The subscription tier means no per-token billing.
pub struct SubscriptionClaudeProvider {
    binary: String,
    model: Option<String>,
}

impl SubscriptionClaudeProvider {
    pub fn new() -> Self {
        Self { binary: "claude".into(), model: None }
    }

    pub fn with_binary(mut self, path: impl Into<String>) -> Self {
        self.binary = path.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl Default for SubscriptionClaudeProvider {
    fn default() -> Self { Self::new() }
}

impl LlmProvider for SubscriptionClaudeProvider {
    fn name(&self) -> &'static str { "SubscriptionClaude" }

    fn complete(&self, prompt: &str) -> Result<String> {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("-p")
            .arg("--output-format").arg("json")
            .arg(prompt);
        if let Some(m) = &self.model {
            cmd.arg("--model").arg(m);
        }
        let out = cmd.output()
            .with_context(|| format!("spawn {}", self.binary))?;
        if !out.status.success() {
            // claude prints auth errors ("Not logged in · Please run /login") to stdout, not stderr.
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            anyhow::bail!(
                "claude -p exited {}: stderr={:?} stdout={:?}",
                out.status, stderr.trim(), stdout.trim(),
            );
        }
        let json: serde_json::Value = serde_json::from_slice(&out.stdout)
            .context("parse claude json")?;
        let text = json.get("result")
            .and_then(|v| v.as_str())
            .context("claude json missing 'result' field")?;
        Ok(text.to_string())
    }
}

/// Declared per §4 but disabled until stage 2 (per 00-plan.html).
/// Wired so the router type-checks; never actually called in stage 1.
pub struct OllamaViaApertureProvider;

impl LlmProvider for OllamaViaApertureProvider {
    fn name(&self) -> &'static str { "OllamaViaAperture" }

    fn complete(&self, _prompt: &str) -> Result<String> {
        anyhow::bail!("OllamaViaAperture is wired but disabled until stage 2");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_provider_is_disabled() {
        let p = OllamaViaApertureProvider;
        let err = p.complete("anything").unwrap_err().to_string();
        assert!(err.contains("disabled"));
    }

    #[test]
    fn subscription_provider_names_itself() {
        assert_eq!(SubscriptionClaudeProvider::new().name(), "SubscriptionClaude");
    }
}
