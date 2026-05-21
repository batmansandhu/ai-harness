//! Stage-1 webhook surface. Two routes only: /health and /jobs/career-ops-scan.
//!
//! /jobs/career-ops-scan WRAPS the existing batch-runner.sh on HOMENET; it does
//! not reimplement the worker pool, claude -p invocation, or state machine.
//! See plans/AIH/03-routing-policy.html §1 W1 for why.

use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::{get, post}, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::Stdio;
use ulid::Ulid;

const JOBS_NDJSON: &str = "/var/log/aih/jobs.ndjson";

#[derive(Debug, Deserialize, Default)]
pub struct CareerOpsScanRequest {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub parallel: Option<u8>,
    #[serde(default)]
    pub retry_failed: bool,
}

#[derive(Debug, Serialize)]
pub struct JobAccepted {
    pub job_id: String,
    pub capability: &'static str,
    pub accepted_at: String,
    pub log_file: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/jobs/career-ops-scan", post(career_ops_scan))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

/// Append one NDJSON row to /var/log/aih/jobs.ndjson.
/// Per plans/AIH/03-routing-policy.html §8. Stage-1 writes the `accepted`
/// event only — terminal/verify fields land when the reaper ships.
fn append_job_event(path: &str, row: &serde_json::Value) {
    let line = match serde_json::to_string(row) {
        Ok(s) => s,
        Err(e) => { tracing::warn!(error=%e, "ndjson serialize failed"); return; }
    };
    let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(e) => { tracing::warn!(error=%e, path=%path, "ndjson open failed"); return; }
    };
    if let Err(e) = writeln!(f, "{line}") {
        tracing::warn!(error=%e, "ndjson write failed");
    }
}

async fn career_ops_scan(Json(req): Json<CareerOpsScanRequest>) -> impl IntoResponse {
    let job_id = Ulid::new().to_string();
    let log_dir = "/var/log/aih";
    let _ = tokio::fs::create_dir_all(log_dir).await;
    let log_path = format!("{log_dir}/{job_id}.log");

    // Spawn batch-runner.sh on HOMENET via the ai-agent ssh key already on LXC 130.
    // Detached (no .await) — webhook returns immediately with job_id.
    let mut args: Vec<String> = vec![
        "ai-agent@192.168.1.250".into(),
        "cd /opt/agents-data/projects/career-ops/batch && ./batch-runner.sh".into(),
    ];
    if req.dry_run { args[1].push_str(" --dry-run"); }
    if let Some(p) = req.parallel { args[1].push_str(&format!(" --parallel {p}")); }
    if req.retry_failed { args[1].push_str(" --retry-failed"); }

    let log = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("log create failed: {e}")).into_response(),
    };
    let log_err = match log.try_clone() {
        Ok(f) => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("log clone failed: {e}")).into_response(),
    };

    let spawn_res = tokio::process::Command::new("ssh")
        .args([
            "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=accept-new",
            "-i", "/home/ai-agent/.ssh/id_ed25519",
        ])
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn();

    let child = match spawn_res {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("ssh spawn failed: {e}")).into_response(),
    };
    let pid = child.id();
    tracing::info!(%job_id, ?pid, "career-ops-scan dispatched");
    drop(child);

    let accepted_at = Utc::now().to_rfc3339();

    // §8 row — stage-1 acceptance event. Terminal fields filled in by the
    // (future) reaper. Schema-compatible: extra fields, no field renames.
    append_job_event(JOBS_NDJSON, &serde_json::json!({
        "job_id": job_id,
        "capability": "career-ops-scan",
        "event": "accepted",
        "task_class": "T0",
        "risk_class": "R2",
        "route": {"provider": "DeterministicShell", "model": null},
        "timing": {"accepted_at": accepted_at},
        "inputs": {
            "dry_run": req.dry_run,
            "parallel": req.parallel,
            "retry_failed": req.retry_failed,
        },
        "pid": pid,
        "log_file": log_path,
        "status": "accepted"
    }));

    let body = JobAccepted {
        job_id,
        capability: "career-ops-scan",
        accepted_at,
        log_file: log_path,
    };
    (StatusCode::ACCEPTED, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_200() {
        let app = router();
        let res = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[test]
    fn append_job_event_writes_valid_ndjson() {
        let dir = std::env::temp_dir().join(format!("aih-test-{}", Ulid::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("jobs.ndjson");
        let row = serde_json::json!({"job_id": "01TEST", "event": "accepted"});
        append_job_event(path.to_str().unwrap(), &row);
        append_job_event(path.to_str().unwrap(), &row);
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed["job_id"], "01TEST");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
