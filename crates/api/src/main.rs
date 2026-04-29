use anyhow::Context;
use satori_api::{AppState, app};
use satori_core::load_cards_from_reader;
use std::{env, fs::File};
use tokio::net::TcpListener;

const DEFAULT_CARDS_PATH: &str = "data/processed/cards.json";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let address = "127.0.0.1:3000";
    let cards_path =
        env::var("SATORI_CARDS_PATH").unwrap_or_else(|_| DEFAULT_CARDS_PATH.to_owned());
    let cards_file =
        File::open(&cards_path).with_context(|| format!("failed to open {cards_path}"))?;
    let cards = load_cards_from_reader(cards_file)
        .with_context(|| format!("failed to load jargon cards from {cards_path}"))?;
    let state = AppState::new(cards).context("failed to build validated app state")?;
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind {address}"))?;

    axum::serve(listener, app(state))
        .await
        .context("api server failed")?;

    Ok(())
}
