use anyhow::Context;
use satori_core::{load_cards_from_reader, validate_cards};
use std::{env, fs::File};

const DEFAULT_CARDS_PATH: &str = "data/processed/cards.json";

fn main() -> anyhow::Result<()> {
    let cards_path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CARDS_PATH.to_owned());
    let cards_file =
        File::open(&cards_path).with_context(|| format!("failed to open {cards_path}"))?;
    let cards = load_cards_from_reader(cards_file)
        .with_context(|| format!("failed to load jargon cards from {cards_path}"))?;

    validate_cards(&cards)?;

    println!("validated {} card(s) from {cards_path}", cards.len());
    Ok(())
}
