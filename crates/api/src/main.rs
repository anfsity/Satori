use anyhow::Context;
use satori_api::{AppState, LanceDbSearch, LanceDbSearchConfig, app};
use satori_core::{JargonCard, load_cards_from_reader};
use std::{env, fs::File};
use tokio::net::TcpListener;

const DEFAULT_CARDS_PATH: &str = "data/processed/cards.json";
const DEFAULT_LANCEDB_TABLE: &str = "index_documents";
const DEFAULT_EMBEDDING_MODEL: &str = "paraphrase-multilingual-MiniLM-L12-v2";
const DEFAULT_ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let address = env::var("SATORI_API_ADDR").unwrap_or_else(|_| DEFAULT_ADDRESS.to_owned());
    let card_paths = card_paths();
    let cards = load_cards_from_paths(&card_paths)?;
    let state = app_state(cards).await?;
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("failed to bind {address}"))?;

    axum::serve(listener, app(state))
        .await
        .context("api server failed")?;

    Ok(())
}

fn card_paths() -> Vec<String> {
    if let Some(paths) = optional_env("SATORI_CARDS_PATHS") {
        return paths
            .split(':')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .collect();
    }

    vec![env::var("SATORI_CARDS_PATH").unwrap_or_else(|_| DEFAULT_CARDS_PATH.to_owned())]
}

fn load_cards_from_paths(paths: &[String]) -> anyhow::Result<Vec<JargonCard>> {
    let mut cards = Vec::new();

    for path in paths {
        let cards_file = File::open(path).with_context(|| format!("failed to open {path}"))?;
        let mut loaded_cards = load_cards_from_reader(cards_file)
            .with_context(|| format!("failed to load jargon cards from {path}"))?;
        cards.append(&mut loaded_cards);
    }

    Ok(cards)
}

async fn app_state(cards: Vec<JargonCard>) -> anyhow::Result<AppState> {
    let Some(db_path) = optional_env("SATORI_LANCEDB_PATH") else {
        return AppState::new(cards).context("failed to build validated app state");
    };
    let table_name =
        env::var("SATORI_LANCEDB_TABLE").unwrap_or_else(|_| DEFAULT_LANCEDB_TABLE.to_owned());
    let model_name =
        env::var("SATORI_EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_owned());
    let vector_search = LanceDbSearch::open(&LanceDbSearchConfig {
        db_path,
        table_name,
        model_name,
    })
    .await
    .context("failed to initialize LanceDB search")?;

    AppState::with_lancedb_search(cards, vector_search)
        .context("failed to build validated app state")
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}
