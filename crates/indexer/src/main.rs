use anyhow::{Context, bail};
use satori_core::{JargonCard, load_cards_from_reader, validate_cards};
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    path::Path,
};

const DEFAULT_CARDS_PATH: &str = "data/processed/cards.json";
const DEFAULT_SOURCE: &str = "mcsrainbow/chinese-internet-jargon";

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        Some("import-mcsrainbow") => import_mcsrainbow(&args[1..]),
        Some("validate") => validate_command(args.get(1).map(String::as_str)),
        Some(path) if Path::new(path).exists() => validate_command(Some(path)),
        Some(command) => bail!("unrecognized command or missing file: {command}"),
        None => validate_command(None),
    }
}

fn validate_command(path: Option<&str>) -> anyhow::Result<()> {
    let cards_path = path.unwrap_or(DEFAULT_CARDS_PATH);
    let cards_file =
        File::open(&cards_path).with_context(|| format!("failed to open {cards_path}"))?;
    let cards = load_cards_from_reader(cards_file)
        .with_context(|| format!("failed to load jargon cards from {cards_path}"))?;

    validate_cards(&cards)?;

    println!("validated {} card(s) from {cards_path}", cards.len());
    Ok(())
}

fn import_mcsrainbow(args: &[String]) -> anyhow::Result<()> {
    let input_path = args
        .first()
        .map(String::as_str)
        .unwrap_or("data/raw/mcsrainbow/readme.md");
    let output_path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("data/processed/imported/mcsrainbow_cards.json");
    let markdown =
        fs::read_to_string(input_path).with_context(|| format!("failed to read {input_path}"))?;
    let cards = parse_mcsrainbow_markdown(&markdown);

    validate_cards(&cards)?;
    write_cards(output_path, &cards)?;

    println!("imported {} card(s) into {output_path}", cards.len());
    Ok(())
}

fn write_cards(path: &str, cards: &[JargonCard]) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(cards).context("failed to serialize cards")?;
    let temp_path = format!("{path}.tmp");

    fs::write(&temp_path, format!("{json}\n"))
        .with_context(|| format!("failed to write {temp_path}"))?;
    fs::rename(&temp_path, path)
        .with_context(|| format!("failed to move {temp_path} to {path}"))?;

    Ok(())
}

fn parse_mcsrainbow_markdown(markdown: &str) -> Vec<JargonCard> {
    let mut cards = Vec::new();
    let mut seen_terms = HashSet::new();
    let mut in_explanation_section = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            in_explanation_section = trimmed.contains("解释") && !trimmed.contains("翻译");
            continue;
        }

        if !in_explanation_section {
            continue;
        }

        let Some((term, explanation)) = parse_explanation_line(trimmed) else {
            continue;
        };

        if !seen_terms.insert(term.to_owned()) {
            continue;
        }

        cards.push(JargonCard {
            id: imported_card_id(&term),
            term,
            plain: explanation.clone(),
            explanation: explanation.clone(),
            examples: Vec::new(),
            queries: vec![explanation],
            tags: vec!["external".to_owned(), "jargon".to_owned()],
            source: DEFAULT_SOURCE.to_owned(),
            verified: false,
        });
    }

    cards
}

fn parse_explanation_line(line: &str) -> Option<(String, String)> {
    let normalized = line
        .trim_start_matches(|item: char| item == '-' || item == '*' || item.is_whitespace())
        .trim();
    let (term, explanation) = normalized
        .split_once(" - ")
        .or_else(|| normalized.split_once(" — "))
        .or_else(|| normalized.split_once("："))?;
    let term = term.trim();
    let explanation = explanation.trim();

    if term.is_empty() || explanation.is_empty() {
        return None;
    }

    Some((term.to_owned(), explanation.to_owned()))
}

fn imported_card_id(term: &str) -> String {
    format!("jargon_mcsrainbow_{:016x}", stable_hash(term.as_bytes()))
}

// FNV-1a 64-bit keeps imported IDs stable across platforms.
fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mcsrainbow_markdown_imports_explanation_lines() {
        let markdown = r#"
# 二字黑话词汇解释
赋能 - 提供帮助或支持。
闭环 - 把事情从开始做到结束。

# 二字黑话词汇翻译
赋能 - enable
"#;

        let cards = parse_mcsrainbow_markdown(markdown);

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].term, "赋能");
        assert_eq!(cards[0].plain, "提供帮助或支持。");
        assert!(!cards[0].verified);
    }

    #[test]
    fn parse_mcsrainbow_markdown_skips_duplicate_terms() {
        let markdown = r#"
# 词汇解释
赋能 - 提供帮助或支持。
赋能 - 重复内容。
"#;

        let cards = parse_mcsrainbow_markdown(markdown);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].plain, "提供帮助或支持。");
    }

    #[test]
    fn imported_card_id_is_stable() {
        assert_eq!(imported_card_id("赋能"), imported_card_id("赋能"));
        assert_ne!(imported_card_id("赋能"), imported_card_id("闭环"));
    }
}
