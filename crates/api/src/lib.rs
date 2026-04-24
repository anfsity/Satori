use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use satori_core::{JargonCard, SearchResponse, normalize_query, rank_keyword_matches};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

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

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
    message: &'static str,
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SearchResponse>, ApiError> {
    let query = normalize_query(
        params.get("q").map(String::as_str).unwrap_or_default(),
        MAX_QUERY_CHARS,
    )
    .map_err(ApiError::from_query_error)?;
    let limit = parse_limit(params.get("limit").map(String::as_str))?;
    let results = rank_keyword_matches(&query, state.cards.iter(), limit);

    Ok(Json(SearchResponse { query, results }))
}

fn parse_limit(input: Option<&str>) -> Result<usize, ApiError> {
    match input {
        None => Ok(DEFAULT_LIMIT),
        Some(raw) => raw
            .parse::<usize>()
            .map(|limit| limit.clamp(1, MAX_LIMIT))
            .map_err(|_| ApiError::invalid_limit()),
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    error: &'static str,
    message: &'static str,
}

impl ApiError {
    fn from_query_error(_: satori_core::SearchQueryError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_query",
            message: "q must be present, non-empty, and within the character limit",
        }
    }

    fn invalid_limit() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_limit",
            message: "limit must be an integer between 1 and 50",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.error,
                message: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use satori_core::load_cards_from_reader;
    use serde_json::Value;
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
        let encoded_query = urlencoding::encode(&query);
        let response = app(AppState::new(cards))
            .oneshot(
                Request::builder()
                    .uri(format!("/api/search?q={encoded_query}"))
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

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["error"], "invalid_query");
    }

    #[tokio::test]
    async fn search_rejects_missing_query() {
        let response = app(AppState::new(fixture_cards()))
            .oneshot(
                Request::builder()
                    .uri("/api/search")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["error"], "invalid_query");
    }

    #[tokio::test]
    async fn search_rejects_invalid_limit() {
        let response = app(AppState::new(fixture_cards()))
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=%E6%B5%8B%E8%AF%95&limit=abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["error"], "invalid_limit");
    }
}
