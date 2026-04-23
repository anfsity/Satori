use anyhow::Context;
use satori_api::{AppState, app, fixture_cards};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let address = "127.0.0.1:3000";
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind {address}"))?;

    axum::serve(listener, app(AppState::new(fixture_cards())))
        .await
        .context("api server failed")?;

    Ok(())
}
