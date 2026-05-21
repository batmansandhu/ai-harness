use harness_types::{Capability, ToolContract};

mod provider;
use provider::{LlmProvider, SubscriptionClaudeProvider};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // `harness --probe "prompt"` shells claude -p and prints the result.
    // Used by stage-1 1g smoke test; will be replaced by the webhook + executor in 1h.
    if args.len() >= 3 && args[1] == "--probe" {
        let prompt = &args[2];
        let provider = SubscriptionClaudeProvider::new();
        let reply = provider.complete(prompt)?;
        println!("{reply}");
        return Ok(());
    }

    eprintln!("aih harness v{} — stage 1 skeleton", env!("CARGO_PKG_VERSION"));
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
    eprintln!("usage: harness --probe \"<prompt>\"");
    Ok(())
}
