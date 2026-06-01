#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bot::l0::fts_worker::run_from_env().await
}
