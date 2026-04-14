use anyhow::Result;

// current_thread keeps all async tasks on one thread, which allows
// ContainerSession (containing Box<dyn MasterPty>, which is !Send) to be
// held in App across await points in the TUI event loop.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    void_claw::manager::run().await
}
