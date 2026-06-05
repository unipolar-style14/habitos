use anyhow::Result;

mod cli;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}
