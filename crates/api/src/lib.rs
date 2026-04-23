use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use satori_core::{JargonCard, SearchResponse, normalize_query, rank_keyword_matches};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 50;
const MAX_QUERY_CHARS: usize = 200;

#[derive(Debug, Clone)]
pub struct AppState {
    cards: Arc<Vec<JargonCard>>,
}

impl AppState {
    pub fn new(cards: Vec<JargonCard>) -> Self {
        Self {
            cards: Arc::new(cards),
        }
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/search", get(search))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, ApiError> {
    let query = normalize_query(&params.q, MAX_QUERY_CHARS).map_err(ApiError::from_query_error)?;
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let results = rank_keyword_matches(&query, state.cards.iter(), limit);

    Ok(Json(SearchResponse { query, results }))
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    error: &'static str,
}

impl ApiError {
    fn from_query_error(_: satori_core::SearchQueryError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_query",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorResponse { error: self.error })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use satori_core::load_cards_from_reader;
    use tower::ServiceExt;

    fn fixture_cards() -> Vec<JargonCard> {
        load_cards_from_reader(include_str!("../../../tests/fixtures/cards.json").as_bytes())
            .expect("parse cards fixture JSON")
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let response = app(AppState::new(fixture_cards()))
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn search_returns_matching_card() {
        let cards = fixture_cards();
        let query = cards[0].plain.clone();
        let response = app(AppState::new(fixture_cards()))
            .oneshot(
                Request::builder()
                    .uri(format!("/api/search?q={query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn search_rejects_empty_query() {
        let response = app(AppState::new(fixture_cards()))
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=%20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
