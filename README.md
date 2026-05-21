# AI-Harness (aih)

A typed, deterministic Rust harness around subscription-tier Claude. Webhook in, audited side-effect out.

The harness does the parts an LLM is bad at — routing, classification, budget enforcement, retry tiers, hard-verify gates, structured logging — and delegates only the judgment step to `claude -p`. R3 (credential-touching) actions never reach the LLM.

## Why this exists

LLM "agent frameworks" tend to put the model in charge and bolt safety on as an afterthought. I wanted the opposite: deterministic code owns the state machine, the LLM is one tool among twelve, and every job produces a row of training data for whatever comes next.

Subscription Claude — not metered API — runs the LLM step. The harness shells `claude -p --output-format=stream-json`, which means cost is constant and predictable instead of per-token.

## Status

**Stage 1 (in progress, 2026-05).** Single workflow live: `career-ops-scan` — wraps an existing job-search batch pipeline behind a typed `POST /jobs/career-ops-scan` endpoint.

Future stages add a local CI-fix bot, a session-to-blog drafter, and homelab ops via Home Assistant / Docker.

## Design (one paragraph)

A `Job` is a typed value with a `capability` (which workflow), a `risk_class` (R0 read → R3 credential), a `task_class` (T0 deterministic, T1 LLM-required, T2 cheap-iteration), a `budget`, and a list of `success_criteria` the verify gate checks after execute. Routing is `(task, risk) → provider`. R3 never reaches the LLM. The verify gate is non-negotiable; soft-verify (LLM-as-judge) is opt-in per workflow.

## Architecture

See [`ARCHITECTURE.html`](./ARCHITECTURE.html) for the visual overview, and `plans/AIH/03-routing-policy.html` (in the project's planning corpus) for the full spec — every type in `harness-types` lifts verbatim from §2/§3/§6 of that document.

## Build

```
cargo test -p harness-types
cargo run -p harness
```

Requires Rust 1.75+ and a working `claude` CLI on `$PATH` for the LLM step.

## Repo layout

```
crates/
  harness/         bin — the webhook + executor (stage 1 = skeleton)
  harness-types/   lib — Job, RiskClass, TaskClass, ToolContract (§2/§3/§6)
```

## Author

Amar Sandhu · `batmansandhu@gmail.com`
