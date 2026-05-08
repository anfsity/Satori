use anyhow::{Context, ensure};
use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, Float32Array, Float64Array, RecordBatch, StringArray,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::StreamExt;
use lancedb::{
    Table,
    embeddings::{
        EmbeddingFunction,
        sentence_transformers::{
            SentenceTransformersEmbeddings, SentenceTransformersEmbeddingsBuilder,
        },
    },
    query::{ExecutableQuery, QueryBase, Select},
};
use satori_core::{
    CardValidationError, JargonCard, SearchIndex, SearchResponse, SearchResult, normalize_query,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tower_http::cors::{Any, CorsLayer};

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 50;
const MAX_QUERY_CHARS: usize = 200;

#[derive(Debug, Clone)]
pub struct AppState {
    search_index: Arc<SearchIndex>,
    cards_by_id: Arc<HashMap<String, JargonCard>>,
    vector_search: Option<Arc<LanceDbSearch>>,
}

impl AppState {
    pub fn new(cards: Vec<JargonCard>) -> Result<Self, CardValidationError> {
        Self::with_vector_search(cards, None)
    }

    pub fn with_lancedb_search(
        cards: Vec<JargonCard>,
        vector_search: LanceDbSearch,
    ) -> Result<Self, CardValidationError> {
        Self::with_vector_search(cards, Some(Arc::new(vector_search)))
    }

    fn with_vector_search(
        cards: Vec<JargonCard>,
        vector_search: Option<Arc<LanceDbSearch>>,
    ) -> Result<Self, CardValidationError> {
        let cards_by_id = cards
            .iter()
            .map(|card| (card.id.clone(), card.clone()))
            .collect::<HashMap<_, _>>();

        Ok(Self {
            search_index: Arc::new(SearchIndex::new(cards)?),
            cards_by_id: Arc::new(cards_by_id),
            vector_search,
        })
    }

    async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        if let Some(vector_search) = &self.vector_search {
            let matches = vector_search.search(query, limit).await?;
            return Ok(vector_matches_to_results(&matches, &self.cards_by_id));
        }

        Ok(self.search_index.search(query, limit))
    }
}

#[derive(Debug, Clone)]
pub struct LanceDbSearchConfig {
    pub db_path: String,
    pub table_name: String,
    pub model_name: String,
}

#[derive(Debug)]
pub struct LanceDbSearch {
    table: Table,
    embedder: SentenceTransformersEmbeddings,
}

impl LanceDbSearch {
    pub async fn open(config: &LanceDbSearchConfig) -> anyhow::Result<Self> {
        let database = lancedb::connect(&config.db_path)
            .execute()
            .await
            .with_context(|| format!("failed to connect to LanceDB at {}", config.db_path))?;
        let table = database
            .open_table(&config.table_name)
            .execute()
            .await
            .with_context(|| format!("failed to open LanceDB table {}", config.table_name))?;
        let embedder = SentenceTransformersEmbeddingsBuilder::new()
            .model(&config.model_name)
            .build()
            .with_context(|| format!("failed to load embedding model {}", config.model_name))?;

        Ok(Self { table, embedder })
    }

    async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<VectorMatch>> {
        let query_vector = self.embed_query(query)?;
        let mut stream = self
            .table
            .query()
            .nearest_to(query_vector.as_slice())?
            .column("vector")
            .limit(limit)
            .select(Select::columns(&["id", "_distance"]))
            .execute()
            .await
            .context("failed to query LanceDB table")?;
        let mut matches = Vec::new();

        while let Some(batch) = stream.next().await {
            matches.extend(vector_matches_from_batch(&batch?)?);
        }

        Ok(matches)
    }

    fn embed_query(&self, query: &str) -> anyhow::Result<Vec<f32>> {
        let embeddings = self
            .embedder
            .compute_query_embeddings(Arc::new(StringArray::from(vec![query.to_owned()])))
            .context("failed to compute query embedding")?;
        query_embedding_to_vector(&embeddings)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct VectorMatch {
    id: String,
    distance: f32,
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
    let results = state.search(&query, limit).await?;

    Ok(Json(SearchResponse { query, results }))
}

fn vector_matches_from_batch(batch: &RecordBatch) -> anyhow::Result<Vec<VectorMatch>> {
    let ids = batch
        .column_by_name("id")
        .context("LanceDB result is missing id column")?
        .as_any()
        .downcast_ref::<StringArray>()
        .context("LanceDB id column is not Utf8")?;
    let distances = batch
        .column_by_name("_distance")
        .context("LanceDB result is missing _distance column")?;

    ensure!(
        ids.len() == distances.len(),
        "LanceDB id and distance column length mismatch"
    );

    (0..ids.len())
        .map(|row| {
            ensure!(!ids.is_null(row), "LanceDB id at row {row} is null");

            Ok(VectorMatch {
                id: ids.value(row).to_owned(),
                distance: distance_at(distances.as_ref(), row)?,
            })
        })
        .collect()
}

fn query_embedding_to_vector(embeddings: &ArrayRef) -> anyhow::Result<Vec<f32>> {
    if let Some(values) = embeddings.as_any().downcast_ref::<Float32Array>() {
        ensure!(!values.is_empty(), "query embedding is empty");
        return Ok((0..values.len()).map(|index| values.value(index)).collect());
    }

    let list_array = embeddings
        .as_any()
        .downcast_ref::<FixedSizeListArray>()
        .context("expected Float32 or fixed-size-list query embedding array")?;

    ensure!(
        list_array.len() == 1,
        "query embedding count mismatch: expected 1 vector, got {}",
        list_array.len()
    );
    ensure!(!list_array.is_null(0), "query embedding row is null");

    let dimension = list_array.value_length() as usize;
    ensure!(dimension > 0, "query embedding is empty");

    let values = list_array
        .values()
        .as_any()
        .downcast_ref::<Float32Array>()
        .context("expected Float32 query embedding values")?;

    Ok((0..dimension).map(|index| values.value(index)).collect())
}

fn distance_at(column: &dyn Array, row: usize) -> anyhow::Result<f32> {
    if let Some(values) = column.as_any().downcast_ref::<Float32Array>() {
        ensure!(
            !values.is_null(row),
            "LanceDB distance at row {row} is null"
        );
        return Ok(values.value(row));
    }

    if let Some(values) = column.as_any().downcast_ref::<Float64Array>() {
        ensure!(
            !values.is_null(row),
            "LanceDB distance at row {row} is null"
        );
        return Ok(values.value(row) as f32);
    }

    anyhow::bail!("LanceDB _distance column is not Float32 or Float64")
}

fn vector_matches_to_results(
    matches: &[VectorMatch],
    cards_by_id: &HashMap<String, JargonCard>,
) -> Vec<SearchResult> {
    matches
        .iter()
        .filter_map(|vector_match| {
            cards_by_id
                .get(&vector_match.id)
                .map(|card| SearchResult::from_card(card, distance_to_score(vector_match.distance)))
        })
        .collect()
}

fn distance_to_score(distance: f32) -> f32 {
    if !distance.is_finite() || distance < 0.0 {
        return 0.0;
    }

    1.0 / (1.0 + distance)
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

    fn internal_search_error() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "search_failed",
            message: "search backend failed",
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(_: anyhow::Error) -> Self {
        Self::internal_search_error()
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
    use arrow_array::types::Float32Type;
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

    #[test]
    fn query_embedding_to_vector_reads_fixed_size_list_values() {
        let embeddings: ArrayRef =
            Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    vec![Some(vec![Some(0.25), Some(0.5), Some(0.75)].into_iter())],
                    3,
                ),
            );

        let vector = query_embedding_to_vector(&embeddings).unwrap();

        assert_eq!(vector, vec![0.25, 0.5, 0.75]);
    }

    #[test]
    fn query_embedding_to_vector_reads_flat_query_values() {
        let embeddings: ArrayRef = Arc::new(Float32Array::from(vec![0.25, 0.5, 0.75]));

        let vector = query_embedding_to_vector(&embeddings).unwrap();

        assert_eq!(vector, vec![0.25, 0.5, 0.75]);
    }

    #[test]
    fn vector_matches_preserve_lancedb_order_and_response_shape() {
        let cards = fixture_cards();
        let cards_by_id = cards
            .iter()
            .map(|card| (card.id.clone(), card.clone()))
            .collect::<HashMap<_, _>>();
        let matches = vec![
            VectorMatch {
                id: cards[1].id.clone(),
                distance: 0.0,
            },
            VectorMatch {
                id: cards[0].id.clone(),
                distance: 1.0,
            },
        ];

        let results = vector_matches_to_results(&matches, &cards_by_id);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, cards[1].id);
        assert_eq!(results[0].examples, cards[1].examples);
        assert_eq!(results[0].score, 1.0);
        assert_eq!(results[1].id, cards[0].id);
        assert_eq!(results[1].score, 0.5);
    }

    #[test]
    fn vector_matches_skip_rows_missing_from_loaded_cards() {
        let cards = fixture_cards();
        let cards_by_id = cards
            .iter()
            .map(|card| (card.id.clone(), card.clone()))
            .collect::<HashMap<_, _>>();
        let matches = vec![VectorMatch {
            id: "missing_card".to_owned(),
            distance: 0.0,
        }];

        let results = vector_matches_to_results(&matches, &cards_by_id);

        assert!(results.is_empty());
    }
}
