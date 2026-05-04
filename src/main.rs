use anyhow::Result;

// current_thread keeps all async tasks on one thread, which allows
// ContainerSession (containing Box<dyn MasterPty>, which is !Send) to be
// held in App across await points in the TUI event loop.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let exit_code = harness_hat::passthrough::run_and_get_exit_code().await?;
    std::process::exit(exit_code);
}
