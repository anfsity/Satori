use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error::Error, fmt, io::Read};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JargonCard {
    pub id: String,
    pub term: String,
    pub plain: String,
    pub explanation: String,
    pub examples: Vec<String>,
    pub queries: Vec<String>,
    pub tags: Vec<String>,
    pub source: String,
    pub verified: bool,
}

impl JargonCard {
    pub fn searchable_text(&self) -> String {
        [
            self.term.as_str(),
            self.plain.as_str(),
            self.explanation.as_str(),
            &self.examples.join(" "),
            &self.queries.join(" "),
            &self.tags.join(" "),
        ]
        .join(" ")
    }

    pub fn index_document(&self) -> IndexDocument {
        IndexDocument::from_card(self)
    }
}

#[derive(Debug, Clone)]
pub struct SearchIndex {
    entries: Vec<SearchEntry>,
}

#[derive(Debug, Clone)]
struct SearchEntry {
    card: JargonCard,
    searchable_text: String,
    searchable_chars: HashSet<char>,
    searchable_bigrams: HashSet<(char, char)>,
}

impl SearchIndex {
    pub fn new(cards: Vec<JargonCard>) -> Result<Self, CardValidationError> {
        validate_cards(&cards)?;

        Ok(Self {
            entries: cards
                .into_iter()
                .map(|card| {
                    let searchable_text = card.searchable_text();
                    let searchable_chars = distinct_non_whitespace_chars(&searchable_text);
                    let searchable_bigrams = distinct_non_whitespace_bigrams(&searchable_text);

                    SearchEntry {
                        card,
                        searchable_text,
                        searchable_chars,
                        searchable_bigrams,
                    }
                })
                .collect(),
        })
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query = query.trim();
        let query_chars = distinct_non_whitespace_chars(query);
        let query_bigrams = distinct_non_whitespace_bigrams(query);

        if query.is_empty() || limit == 0 {
            return Vec::new();
        }

        let mut results = self
            .entries
            .iter()
            .filter_map(|entry| {
                let score = keyword_score_with_text(
                    query,
                    &query_chars,
                    &query_bigrams,
                    &entry.card,
                    &entry.searchable_text,
                    &entry.searchable_chars,
                    &entry.searchable_bigrams,
                );

                (score > 0.0).then(|| SearchResult::from_card(&entry.card, score))
            })
            .collect::<Vec<_>>();

        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.term.cmp(&right.term))
        });
        results.truncate(limit);
        results
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDocument {
    pub id: String,
    pub term: String,
    pub plain: String,
    pub explanation: String,
    pub tags: Vec<String>,
    pub source: String,
    pub verified: bool,
    pub content: String,
}

impl IndexDocument {
    pub fn from_card(card: &JargonCard) -> Self {
        Self {
            id: card.id.clone(),
            term: card.term.clone(),
            plain: card.plain.clone(),
            explanation: card.explanation.clone(),
            tags: card.tags.clone(),
            source: card.source.clone(),
            verified: card.verified,
            content: build_index_content(card),
        }
    }
}

pub fn build_index_documents(cards: &[JargonCard]) -> Vec<IndexDocument> {
    cards.iter().map(JargonCard::index_document).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub term: String,
    pub plain: String,
    pub explanation: String,
    pub examples: Vec<String>,
    pub tags: Vec<String>,
    pub score: f32,
}

impl SearchResult {
    pub fn from_card(card: &JargonCard, score: f32) -> Self {
        Self {
            id: card.id.clone(),
            term: card.term.clone(),
            plain: card.plain.clone(),
            explanation: card.explanation.clone(),
            examples: card.examples.clone(),
            tags: card.tags.clone(),
            score,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchQueryError {
    Empty,
    TooLong { max_chars: usize },
}

#[derive(Debug)]
pub enum CardLoadError {
    Json(serde_json::Error),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardValidationIssue {
    pub card_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardValidationError {
    pub issues: Vec<CardValidationIssue>,
}

impl CardValidationError {
    fn new(issues: Vec<CardValidationIssue>) -> Self {
        Self { issues }
    }
}

impl fmt::Display for CardValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "card validation failed with {} issue(s)",
            self.issues.len()
        )?;

        for issue in &self.issues {
            match &issue.card_id {
                Some(card_id) => writeln!(formatter, "- {card_id}: {}", issue.message)?,
                None => writeln!(formatter, "- {}", issue.message)?,
            }
        }

        Ok(())
    }
}

impl Error for CardValidationError {}

impl fmt::Display for CardLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid card JSON: {error}"),
            Self::Empty => formatter.write_str("card collection is empty"),
        }
    }
}

impl Error for CardLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Empty => None,
        }
    }
}

impl From<serde_json::Error> for CardLoadError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn load_cards_from_reader(reader: impl Read) -> Result<Vec<JargonCard>, CardLoadError> {
    let cards: Vec<JargonCard> = serde_json::from_reader(reader)?;

    if cards.is_empty() {
        return Err(CardLoadError::Empty);
    }

    Ok(cards)
}

pub fn validate_cards(cards: &[JargonCard]) -> Result<(), CardValidationError> {
    let mut issues = Vec::new();
    let mut seen_ids = HashSet::new();

    for card in cards {
        let card_id = card_id_for_issue(card);

        check_required_field(&mut issues, &card_id, "id", &card.id);
        check_required_field(&mut issues, &card_id, "term", &card.term);
        check_required_field(&mut issues, &card_id, "plain", &card.plain);
        check_required_field(&mut issues, &card_id, "explanation", &card.explanation);
        check_required_field(&mut issues, &card_id, "source", &card.source);

        if !card.id.trim().is_empty() && !seen_ids.insert(card.id.trim().to_owned()) {
            issues.push(CardValidationIssue {
                card_id,
                message: "duplicate id".to_owned(),
            });
        }

        if !has_searchable_text(card) {
            issues.push(CardValidationIssue {
                card_id: card_id_for_issue(card),
                message: "card has no searchable text".to_owned(),
            });
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(CardValidationError::new(issues))
    }
}

fn check_required_field(
    issues: &mut Vec<CardValidationIssue>,
    card_id: &Option<String>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        issues.push(CardValidationIssue {
            card_id: card_id.clone(),
            message: format!("{field} is required"),
        });
    }
}

fn card_id_for_issue(card: &JargonCard) -> Option<String> {
    let id = card.id.trim();

    (!id.is_empty()).then(|| id.to_owned())
}

fn has_searchable_text(card: &JargonCard) -> bool {
    [
        card.term.as_str(),
        card.plain.as_str(),
        card.explanation.as_str(),
    ]
    .into_iter()
    .chain(card.examples.iter().map(String::as_str))
    .chain(card.queries.iter().map(String::as_str))
    .chain(card.tags.iter().map(String::as_str))
    .any(|item| !item.trim().is_empty())
}

fn build_index_content(card: &JargonCard) -> String {
    let mut sections = vec![
        format!("term: {}", card.term.trim()),
        format!("plain: {}", card.plain.trim()),
        format!("explanation: {}", card.explanation.trim()),
    ];

    if !card.examples.is_empty() {
        sections.push(format!("examples: {}", join_non_empty(&card.examples)));
    }

    if !card.queries.is_empty() {
        sections.push(format!("queries: {}", join_non_empty(&card.queries)));
    }

    if !card.tags.is_empty() {
        sections.push(format!("tags: {}", join_non_empty(&card.tags)));
    }

    sections.join("\n")
}

fn join_non_empty(items: &[String]) -> String {
    items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn normalize_query(input: &str, max_chars: usize) -> Result<String, SearchQueryError> {
    let query = input.trim();

    if query.is_empty() {
        return Err(SearchQueryError::Empty);
    }

    if query.chars().count() > max_chars {
        return Err(SearchQueryError::TooLong { max_chars });
    }

    Ok(query.to_owned())
}

pub fn rank_keyword_matches<'a>(
    query: &str,
    cards: impl IntoIterator<Item = &'a JargonCard>,
    limit: usize,
) -> Vec<SearchResult> {
    let query = query.trim();
    let query_chars = distinct_non_whitespace_chars(query);
    let query_bigrams = distinct_non_whitespace_bigrams(query);

    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut results = cards
        .into_iter()
        .filter_map(|card| {
            let searchable_text = card.searchable_text();
            let searchable_chars = distinct_non_whitespace_chars(&searchable_text);
            let searchable_bigrams = distinct_non_whitespace_bigrams(&searchable_text);
            let score = keyword_score_with_text(
                query,
                &query_chars,
                &query_bigrams,
                card,
                &searchable_text,
                &searchable_chars,
                &searchable_bigrams,
            );

            (score > 0.0).then(|| SearchResult::from_card(card, score))
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.term.cmp(&right.term))
    });
    results.truncate(limit);
    results
}

fn keyword_score_with_text(
    query: &str,
    query_chars: &HashSet<char>,
    query_bigrams: &HashSet<(char, char)>,
    card: &JargonCard,
    searchable_text: &str,
    searchable_chars: &HashSet<char>,
    searchable_bigrams: &HashSet<(char, char)>,
) -> f32 {
    if card.term == query || card.plain == query {
        return 1.0;
    }

    if card.queries.iter().any(|candidate| candidate == query) {
        return 0.95;
    }

    if searchable_text.contains(query) {
        return 0.75;
    }

    let fallback_score = fallback_score(
        query_chars,
        query_bigrams,
        searchable_chars,
        searchable_bigrams,
    );

    if fallback_score > 0.0 {
        return fallback_score;
    }

    0.0
}

fn distinct_non_whitespace_chars(text: &str) -> HashSet<char> {
    text.chars().filter(|item| !item.is_whitespace()).collect()
}

fn distinct_non_whitespace_bigrams(text: &str) -> HashSet<(char, char)> {
    let chars = text
        .chars()
        .filter(|item| !item.is_whitespace())
        .collect::<Vec<_>>();

    chars
        .windows(2)
        .map(|window| (window[0], window[1]))
        .collect()
}

fn fallback_score(
    query_chars: &HashSet<char>,
    query_bigrams: &HashSet<(char, char)>,
    searchable_chars: &HashSet<char>,
    searchable_bigrams: &HashSet<(char, char)>,
) -> f32 {
    let shared_bigrams = query_bigrams
        .iter()
        .filter(|item| searchable_bigrams.contains(item))
        .count();

    if shared_bigrams == 0 {
        return 0.0;
    }

    let shared_chars = query_chars
        .iter()
        .filter(|item| searchable_chars.contains(item))
        .count();
    let char_coverage = shared_chars as f32 / query_chars.len() as f32;
    let bigram_coverage = shared_bigrams as f32 / query_bigrams.len() as f32;

    0.2 + (char_coverage * 0.05) + (bigram_coverage * 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_cards() -> Vec<JargonCard> {
        load_cards_from_reader(include_str!("../../../tests/fixtures/cards.json").as_bytes())
            .expect("parse cards fixture JSON")
    }

    fn card() -> JargonCard {
        fixture_cards().remove(0)
    }

    #[test]
    fn searchable_text_contains_card_fields() {
        let card = card();
        let text = card.searchable_text();

        assert!(text.contains(&card.term));
        assert!(text.contains(&card.plain));
        assert!(card.tags.iter().any(|tag| text.contains(tag)));
    }

    #[test]
    fn index_document_contains_stable_content_sections() {
        let card = card();
        let document = card.index_document();

        assert_eq!(document.id, card.id);
        assert!(document.content.contains("term: "));
        assert!(document.content.contains("plain: "));
        assert!(document.content.contains("explanation: "));
        assert!(document.content.contains("examples: "));
        assert!(document.content.contains("queries: "));
        assert!(document.content.contains("tags: "));
    }

    #[test]
    fn normalize_query_trims_input() {
        assert_eq!(normalize_query("  query  ", 20), Ok("query".to_owned()));
    }

    #[test]
    fn normalize_query_rejects_empty_input() {
        assert_eq!(normalize_query(" ", 20), Err(SearchQueryError::Empty));
    }

    #[test]
    fn normalize_query_rejects_long_input() {
        assert_eq!(
            normalize_query("一二三四五", 4),
            Err(SearchQueryError::TooLong { max_chars: 4 })
        );
    }

    #[test]
    fn rank_keyword_matches_returns_expected_card() {
        let card = card();
        let results = rank_keyword_matches(&card.plain, [&card], 3);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, card.id);
    }

    #[test]
    fn rank_keyword_matches_orders_same_score_by_term() {
        let mut cards = fixture_cards();
        cards[0].term = "b".to_owned();
        cards[1].term = "a".to_owned();
        cards[0].plain = "共享匹配".to_owned();
        cards[1].plain = "共享匹配".to_owned();
        cards[0].queries.clear();
        cards[1].queries.clear();

        let results = rank_keyword_matches("共享匹配", cards.iter(), 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].term, "a");
        assert_eq!(results[1].term, "b");
    }

    #[test]
    fn search_index_returns_expected_card() {
        let card = card();
        let index = SearchIndex::new(vec![card.clone()]).unwrap();
        let results = index.search(&card.plain, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, card.id);
    }

    #[test]
    fn search_index_returns_query_match_before_character_fallback() {
        let cards = fixture_cards();
        let exact_query = cards[0].queries[1].clone();
        let index = SearchIndex::new(cards).unwrap();
        let results = index.search(&exact_query, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "jargon_lar_tong_dui_qi");
        assert_eq!(results[0].score, 0.95);
    }

    #[test]
    fn search_index_returns_bigram_overlap_match_for_approximate_query() {
        let index = SearchIndex::new(fixture_cards()).unwrap();
        let results = index.search("心态一下崩掉", 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "meme_po_fang_le");
        assert!(results[0].score > 0.2);
        assert!(results[0].score < 0.75);
    }

    #[test]
    fn search_index_returns_empty_results_for_unknown_query() {
        let index = SearchIndex::new(fixture_cards()).unwrap();
        let results = index.search("xyznotfound", 5);

        assert!(results.is_empty());
    }

    #[test]
    fn search_index_rejects_low_signal_single_character_overlap() {
        let index = SearchIndex::new(fixture_cards()).unwrap();
        let results = index.search("心情很好", 5);

        assert!(results.is_empty());
    }

    #[test]
    fn load_cards_from_reader_reads_json_cards() {
        let cards = fixture_cards();
        let first = cards.first().expect("fixture should contain cards");

        assert!(!first.id.is_empty());
        assert!(!first.term.is_empty());
        assert!(!first.plain.is_empty());
    }

    #[test]
    fn load_cards_from_reader_rejects_empty_collection() {
        assert!(matches!(
            load_cards_from_reader("[]".as_bytes()),
            Err(CardLoadError::Empty)
        ));
    }

    #[test]
    fn load_cards_from_reader_rejects_invalid_json() {
        assert!(matches!(
            load_cards_from_reader("not json".as_bytes()),
            Err(CardLoadError::Json(_))
        ));
    }

    #[test]
    fn validate_cards_accepts_fixture_cards() {
        assert_eq!(validate_cards(&fixture_cards()), Ok(()));
    }

    #[test]
    fn validate_cards_rejects_blank_required_fields() {
        let mut card = card();
        card.id = " ".to_owned();
        card.term = " ".to_owned();

        let error = validate_cards(&[card]).unwrap_err();

        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.message == "id is required")
        );
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.message == "term is required")
        );
    }

    #[test]
    fn validate_cards_rejects_duplicate_ids() {
        let card = card();
        let duplicate = card.clone();

        let error = validate_cards(&[card, duplicate]).unwrap_err();

        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.message == "duplicate id")
        );
    }

    #[test]
    fn validate_cards_rejects_cards_without_searchable_text() {
        let mut card = card();
        card.term.clear();
        card.plain.clear();
        card.explanation.clear();
        card.examples.clear();
        card.queries.clear();
        card.tags.clear();

        let error = validate_cards(&[card]).unwrap_err();

        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.message == "card has no searchable text")
        );
    }

    #[test]
    fn search_index_rejects_invalid_cards() {
        let mut cards = fixture_cards();
        cards[1].id = cards[0].id.clone();

        let error = SearchIndex::new(cards).unwrap_err();

        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.message == "duplicate id")
        );
    }
}
