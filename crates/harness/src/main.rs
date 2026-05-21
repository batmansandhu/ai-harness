use harness_types::{ToolContract, Capability};

fn main() -> anyhow::Result<()> {
    eprintln!("aih harness v{} — stage 1 skeleton", env!("CARGO_PKG_VERSION"));
    eprintln!("registered capabilities (stage 1 ships W1 only):");
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
    Ok(())
}
