use axum::{
    Json, Router,
    extract::{Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use satori_core::{CardValidationError, JargonCard, SearchIndex, SearchResponse, normalize_query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 50;
const MAX_QUERY_CHARS: usize = 200;

#[derive(Debug, Clone)]
pub struct AppState {
    search_index: Arc<SearchIndex>,
}

impl AppState {
    pub fn new(cards: Vec<JargonCard>) -> Result<Self, CardValidationError> {
        Ok(Self {
            search_index: Arc::new(SearchIndex::new(cards)?),
        })
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/search", get(search))
        .layer(cors_layer())
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::OPTIONS])
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

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
    limit: Option<String>,
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, ApiError> {
    let query = normalize_query(params.q.as_deref().unwrap_or_default(), MAX_QUERY_CHARS)
        .map_err(ApiError::from_query_error)?;
    let limit = parse_limit(params.limit.as_deref())?;
    let results = state.search_index.search(&query, limit);

    Ok(Json(SearchResponse { query, results }))
}

fn parse_limit(input: Option<&str>) -> Result<usize, ApiError> {
    match input {
        None => Ok(DEFAULT_LIMIT),
        Some(raw) => raw
            .parse::<i64>()
            .map(|limit| limit.clamp(1, MAX_LIMIT as i64) as usize)
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
            message: "limit must be an integer value",
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
        http::{Request, StatusCode, header},
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
        let response = app(AppState::new(fixture_cards()).unwrap())
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
        let response = app(AppState::new(cards).unwrap())
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
        let response = app(AppState::new(fixture_cards()).unwrap())
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
        let response = app(AppState::new(fixture_cards()).unwrap())
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
        let response = app(AppState::new(fixture_cards()).unwrap())
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

    #[tokio::test]
    async fn search_clamps_negative_limit_to_one() {
        let response = app(AppState::new(fixture_cards()).unwrap())
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=%E5%A4%A7%E5%AE%B6%E5%85%88%E7%BB%9F%E4%B8%80%E6%83%B3%E6%B3%95&limit=-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["results"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn health_allows_cross_origin_get_requests() {
        let response = app(AppState::new(fixture_cards()).unwrap())
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/health")
                    .header(header::ORIGIN, "http://localhost:5173")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "*"
        );
    }

    #[tokio::test]
    async fn health_handles_cors_preflight_requests() {
        let response = app(AppState::new(fixture_cards()).unwrap())
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/health")
                    .header(header::ORIGIN, "http://localhost:5173")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "*"
        );
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_METHODS)
                .unwrap()
                .to_str()
                .unwrap()
                .contains("GET")
        );
    }

    #[test]
    fn app_state_rejects_invalid_cards() {
        let mut cards = fixture_cards();
        cards[1].id = cards[0].id.clone();

        let error = AppState::new(cards).unwrap_err();

        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.message == "duplicate id")
        );
    }
}
