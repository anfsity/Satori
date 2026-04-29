use anyhow::{Context, bail};
use satori_core::{
    IndexDocument, JargonCard, build_index_documents, load_cards_from_reader, validate_cards,
};
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

const DEFAULT_CARDS_PATH: &str = "data/processed/cards.json";
const DEFAULT_SOURCE: &str = "mcsrainbow/chinese-internet-jargon";

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        Some("import-mcsrainbow") => import_mcsrainbow(&args[1..]),
        Some("export-index-docs") => export_index_docs_command(&args[1..]),
        Some("validate") => validate_command(args.get(1).map(String::as_str)),
        Some(path) if Path::new(path).exists() => validate_command(Some(path)),
        Some(command) => bail!("unrecognized command or missing file: {command}"),
        None => validate_command(None),
    }
}

fn validate_command(path: Option<&str>) -> anyhow::Result<()> {
    let cards_path = path.unwrap_or(DEFAULT_CARDS_PATH);
    let cards_file =
        File::open(cards_path).with_context(|| format!("failed to open {cards_path}"))?;
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

fn export_index_docs_command(args: &[String]) -> anyhow::Result<()> {
    let input_path = args
        .first()
        .map(String::as_str)
        .unwrap_or(DEFAULT_CARDS_PATH);
    let output_path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("data/processed/index_docs.jsonl");
    let cards = load_cards(input_path)?;
    validate_cards(&cards)?;
    let documents = build_index_documents(&cards);

    write_index_documents(output_path, &documents)?;
    println!(
        "exported {} index document(s) into {output_path}",
        documents.len()
    );

    Ok(())
}

fn load_cards(path: &str) -> anyhow::Result<Vec<JargonCard>> {
    let cards_file = File::open(path).with_context(|| format!("failed to open {path}"))?;
    load_cards_from_reader(cards_file)
        .with_context(|| format!("failed to load jargon cards from {path}"))
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

fn write_index_documents(path: &str, documents: &[IndexDocument]) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let temp_path = format!("{path}.tmp");
    let temp_file =
        File::create(&temp_path).with_context(|| format!("failed to create {temp_path}"))?;
    let mut writer = BufWriter::new(temp_file);

    for document in documents {
        serde_json::to_writer(&mut writer, document)
            .context("failed to serialize index document")?;
        writer
            .write_all(b"\n")
            .with_context(|| format!("failed to write {temp_path}"))?;
    }

    writer
        .flush()
        .with_context(|| format!("failed to flush {temp_path}"))?;
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

        let normalized_text = normalize_imported_text(&explanation);

        cards.push(JargonCard {
            id: imported_card_id(&term),
            term,
            plain: normalized_text.plain,
            explanation: normalized_text.explanation,
            examples: Vec::new(),
            queries: normalized_text.queries,
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

/// Normalized explanation payload derived from one imported external corpus entry.
struct NormalizedImportedText {
    plain: String,
    explanation: String,
    queries: Vec<String>,
}

/// Cleans imported explanation text and derives imported searchable fields.
fn normalize_imported_text(raw: &str) -> NormalizedImportedText {
    let segments = split_imported_segments(raw);
    let plain = segments
        .first()
        .cloned()
        .unwrap_or_else(|| raw.trim().to_owned());
    let explanation = segments.join("；");

    NormalizedImportedText {
        plain,
        explanation,
        queries: segments,
    }
}

/// Splits an imported explanation into stable searchable segments.
fn split_imported_segments(raw: &str) -> Vec<String> {
    let normalized = raw.replace('／', " / ");
    let mut segments = Vec::new();
    let mut seen = HashSet::new();

    for segment in normalized.split(" / ") {
        let cleaned = normalize_imported_segment(segment);

        if cleaned.is_empty() || !seen.insert(cleaned.clone()) {
            continue;
        }

        segments.push(cleaned);
    }

    if segments.is_empty() {
        vec![normalize_imported_segment(raw)]
    } else {
        segments
    }
}

/// Cleans one imported explanation segment without changing its meaning.
fn normalize_imported_segment(raw: &str) -> String {
    collapse_whitespace(strip_leading_pronunciation(raw))
}

/// Removes a wrapped pronunciation prefix from the start of an imported segment.
fn strip_leading_pronunciation(raw: &str) -> &str {
    strip_wrapped_pronunciation(raw, '(', ')')
        .or_else(|| strip_wrapped_pronunciation(raw, '（', '）'))
        .unwrap_or(raw)
}

/// Returns the remaining text when a wrapped pronunciation prefix is detected.
fn strip_wrapped_pronunciation(raw: &str, open: char, close: char) -> Option<&str> {
    let trimmed = raw.trim();
    let rest = trimmed.strip_prefix(open)?;
    let end = rest.find(close)?;
    let pronunciation = &rest[..end];

    if !looks_like_pronunciation(pronunciation) {
        return None;
    }

    Some(rest[end + close.len_utf8()..].trim())
}

/// Checks whether a wrapped prefix looks like pronunciation rather than content.
fn looks_like_pronunciation(raw: &str) -> bool {
    !raw.is_empty()
        && raw.chars().all(|item| {
            item.is_alphabetic() || item.is_whitespace() || matches!(item, '-' | '\'' | '·')
        })
}

/// Collapses repeated whitespace so imported text stays deterministic.
fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
        assert_eq!(cards[0].queries, vec!["提供帮助或支持。"]);
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
    fn parse_mcsrainbow_markdown_strips_pronunciation_prefixes() {
        let markdown = r#"
# 词汇解释
阈值 - (yù zhí)触发某种状态变化的临界点
"#;

        let cards = parse_mcsrainbow_markdown(markdown);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].plain, "触发某种状态变化的临界点");
        assert_eq!(cards[0].explanation, "触发某种状态变化的临界点");
        assert_eq!(cards[0].queries, vec!["触发某种状态变化的临界点"]);
    }

    #[test]
    fn parse_mcsrainbow_markdown_splits_multi_meaning_explanations() {
        let markdown = r#"
# 词汇解释
矩阵 - 多渠道规模化的产品或服务组合 / 有m行n列二维数组元素的矩形阵列
"#;

        let cards = parse_mcsrainbow_markdown(markdown);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].plain, "多渠道规模化的产品或服务组合");
        assert_eq!(
            cards[0].explanation,
            "多渠道规模化的产品或服务组合；有m行n列二维数组元素的矩形阵列"
        );
        assert_eq!(
            cards[0].queries,
            vec![
                "多渠道规模化的产品或服务组合",
                "有m行n列二维数组元素的矩形阵列"
            ]
        );
    }

    #[test]
    fn imported_card_id_is_stable() {
        assert_eq!(imported_card_id("赋能"), imported_card_id("赋能"));
        assert_ne!(imported_card_id("赋能"), imported_card_id("闭环"));
    }

    #[test]
    fn write_index_documents_writes_jsonl_rows() {
        let cards =
            load_cards_from_reader(include_str!("../../../tests/fixtures/cards.json").as_bytes())
                .unwrap();
        let documents = build_index_documents(&cards);
        let temp_path = unique_temp_path("index-docs.jsonl");

        write_index_documents(temp_path.to_str().unwrap(), &documents).unwrap();

        let contents = fs::read_to_string(&temp_path).unwrap();
        let lines = contents.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), documents.len());

        let first: IndexDocument = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first.id, "jargon_lar_tong_dui_qi");
        assert!(first.content.contains("term: 拉通对齐"));

        fs::remove_file(temp_path).unwrap();
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        env::temp_dir().join(format!("satori-{nanos}-{name}"))
    }
}
