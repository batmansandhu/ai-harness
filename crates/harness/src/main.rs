use harness_types::{Capability, ToolContract};

mod provider;
mod webhook;

use provider::{LlmProvider, SubscriptionClaudeProvider};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // `harness --probe "prompt"` shells claude -p and prints the result.
    if args.len() >= 3 && args[1] == "--probe" {
        let prompt = &args[2];
        let provider = SubscriptionClaudeProvider::new();
        let reply = provider.complete(prompt)?;
        println!("{reply}");
        return Ok(());
    }

    // `harness serve [addr]` starts the webhook (stage-1 surface).
    if args.len() >= 2 && args[1] == "serve" {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("harness=info".parse().unwrap())
                .add_directive("tower_http=info".parse().unwrap()))
            .init();
        let addr: std::net::SocketAddr = args.get(2)
            .map(|s| s.as_str())
            .unwrap_or("0.0.0.0:8080")
            .parse()?;
        let app = webhook::router()
            .layer(tower_http::trace::TraceLayer::new_for_http());
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!(%addr, "aih harness serving");
        axum::serve(listener, app).await?;
        return Ok(());
    }

    eprintln!("aih harness v{} — stage 1", env!("CARGO_PKG_VERSION"));
    eprintln!("provider: SubscriptionClaude (Ollama wired, disabled)");
    eprintln!("capabilities (stage 1 ships W1 only):");
    for cap in [
        Capability::CareerOpsScan,
        Capability::CiFixBot,
        Capability::BlogFromSession,
        Capability::HomelabHaAction,
        Capability::HomelabDockerOp,
    ] {
        eprintln!("  - {cap:?}");
    }
    eprintln!("tool registry: {} contracts", ToolContract::registry().len());
    eprintln!();
    eprintln!("usage:");
    eprintln!("  harness --probe \"<prompt>\"        — shell claude -p");
    eprintln!("  harness serve [addr=0.0.0.0:8080]  — run the webhook");
    Ok(())
}
