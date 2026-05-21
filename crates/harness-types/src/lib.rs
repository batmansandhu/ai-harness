//! Types for the AI-Harness — lifted verbatim from
//! plans/AIH/03-routing-policy.html §2 (Job), §3 (Risk), §4 (Task), §6 (Tools).
//!
//! The spec is the source of truth. If you change a variant here, update the spec.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ulid::Ulid;

pub type JobId = Ulid;

/// §1 — Golden-Path Workflows. W1 is the only stage-1 capability;
/// W2..W5 are declared so the type system anchors them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    CareerOpsScan,
    CiFixBot,
    BlogFromSession,
    HomelabHaAction,
    HomelabDockerOp,
}

/// §3 — Risk Classes.
/// R3 NEVER reaches the LLM stage (deterministic-only path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskClass {
    /// read-only: git clone, gh api GET, file read, cargo check
    R0,
    /// reversible-write: local file edit, cargo build, git commit (local)
    R1,
    /// external-side-effect: gh pr create, Telegram send, HA service call
    R2,
    /// credential-touching: SOPS decrypt, age key, token mint
    R3,
}

/// §4 — Task Classes (collapsed from iqv2's 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskClass {
    /// deterministic-only — no LLM needed
    T0,
    /// llm-required — judgment, default route: subscription Claude
    T1,
    /// cheap-iteration — high volume, low stakes (stage-2 → Ollama)
    T2,
}

/// §4 — Stage-1 provider routing. OllamaViaAperture is declared but
/// disabled until stage 2 (per 00-plan.html).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    None,
    SubscriptionClaude,
    OllamaViaAperture,
}

/// §7 — Approval policy. Hard means humans must ack before execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalPolicy {
    None,
    SoftConfirm,
    Hard,
}

/// §5 — A check the verify(HARD) gate runs after execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    /// e.g. "exec.cargo test", "gh api /repos/.../pulls/123 -> 200"
    pub spec: String,
}

/// §2 — Budget on a job. Enforced by the executor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Budget {
    pub max_seconds: u32,
    pub max_tokens: u32,
    pub max_retries: u8,
}

/// §8 — Trace context. Threads through every tool call for the per-job log row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCtx {
    pub job_id: JobId,
    pub started_at: DateTime<Utc>,
    pub route_history: Vec<String>,
}

/// §2 — The Job schema. Every webhook / cron / chat trigger materializes into this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub capability: Capability,
    pub inputs: Value,
    pub risk_class: RiskClass,
    pub task_class: TaskClass,
    pub budget: Budget,
    pub success_criteria: Vec<Check>,
    pub approval: ApprovalPolicy,
    pub trace: TraceCtx,
}

/// §6 — Tool contracts. Every tool the harness invokes has a fixed shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    ExecShell,
    ExecCargo,
    ExecGit,
    GhPrCreate,
    GhApi,
    LlmClaudeP,
    LlmOllama,
    FsRead,
    FsWriteScoped,
    HttpGet,
    HttpPostLan,
    ScrubRun,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToolContract {
    pub name: ToolName,
    pub timeout_secs: u32,
    pub retries: u8,
    pub default_risk: RiskClass,
}

impl ToolContract {
    /// §6 — static registry, one row per tool.
    pub const fn registry() -> &'static [ToolContract] {
        use RiskClass::*;
        use ToolName::*;
        &[
            ToolContract { name: ExecShell,     timeout_secs: 120, retries: 0, default_risk: R0 },
            ToolContract { name: ExecCargo,     timeout_secs: 600, retries: 1, default_risk: R1 },
            ToolContract { name: ExecGit,       timeout_secs: 60,  retries: 1, default_risk: R1 },
            ToolContract { name: GhPrCreate,    timeout_secs: 30,  retries: 0, default_risk: R2 },
            ToolContract { name: GhApi,         timeout_secs: 30,  retries: 2, default_risk: R0 },
            ToolContract { name: LlmClaudeP,    timeout_secs: 180, retries: 2, default_risk: R0 },
            ToolContract { name: LlmOllama,     timeout_secs: 120, retries: 1, default_risk: R0 },
            ToolContract { name: FsRead,        timeout_secs: 5,   retries: 0, default_risk: R0 },
            ToolContract { name: FsWriteScoped, timeout_secs: 5,   retries: 0, default_risk: R1 },
            ToolContract { name: HttpGet,       timeout_secs: 30,  retries: 2, default_risk: R0 },
            ToolContract { name: HttpPostLan,   timeout_secs: 30,  retries: 1, default_risk: R1 },
            ToolContract { name: ScrubRun,      timeout_secs: 2,   retries: 0, default_risk: R0 },
        ]
    }
}

/// §7 — Failure taxonomy. Tier = how the harness reacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureTier {
    HardFail,
    SoftRetry,
    HumanPause,
}

/// §7 — Category = which subsystem failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Routing,
    Tool,
    Model,
    State,
    Permission,
    Verification,
}

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{tier:?}/{category:?}: {message}")]
pub struct Failure {
    pub tier: FailureTier,
    pub category: FailureCategory,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r3_never_routes_to_llm() {
        // §3: R3 NEVER reaches LLM stage.
        // This is enforced at the router level; here we just lock the ordering.
        assert!(RiskClass::R3 > RiskClass::R2);
        assert!(RiskClass::R0 < RiskClass::R1);
    }

    #[test]
    fn tool_registry_has_all_twelve_tools() {
        // §6: registry must list every tool the harness can invoke.
        assert_eq!(ToolContract::registry().len(), 12);
    }

    #[test]
    fn job_roundtrips_json() {
        let job = Job {
            id: Ulid::new(),
            capability: Capability::CareerOpsScan,
            inputs: serde_json::json!({"roles": []}),
            risk_class: RiskClass::R1,
            task_class: TaskClass::T1,
            budget: Budget { max_seconds: 600, max_tokens: 8000, max_retries: 2 },
            success_criteria: vec![Check {
                name: "pdf-emitted".into(),
                spec: "fs.read /opt/agents-data/projects/career-ops/output/*.pdf".into(),
            }],
            approval: ApprovalPolicy::None,
            trace: TraceCtx {
                job_id: Ulid::new(),
                started_at: Utc::now(),
                route_history: vec![],
            },
        };
        let s = serde_json::to_string(&job).unwrap();
        let back: Job = serde_json::from_str(&s).unwrap();
        assert_eq!(back.capability, Capability::CareerOpsScan);
    }
}
