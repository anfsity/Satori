use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, io::Read};

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

    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut results = cards
        .into_iter()
        .filter_map(|card| {
            let score = keyword_score(query, card);

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

fn keyword_score(query: &str, card: &JargonCard) -> f32 {
    if card.term == query || card.plain == query {
        return 1.0;
    }

    if card.queries.iter().any(|candidate| candidate == query) {
        return 0.95;
    }

    let text = card.searchable_text();

    if text.contains(query) {
        return 0.75;
    }

    if query.chars().any(|item| text.contains(item)) {
        return 0.2;
    }

    0.0
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
}
