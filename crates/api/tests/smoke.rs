use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use satori_api::{AppState, app};
use satori_core::{JargonCard, load_cards_from_reader};
use serde_json::Value;
use std::{fs, fs::File, path::PathBuf};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok_status() {
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

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload, load_expected_json("health.json"));
}

fn fixture_cards() -> Vec<JargonCard> {
    let path = repo_root()
        .join("tests")
        .join("fixtures")
        .join("cards.json");
    let file = File::open(path).unwrap();

    load_cards_from_reader(file).unwrap()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn api_examples_root() -> PathBuf {
    repo_root().join("docs").join("api-examples")
}

fn load_expected_json(name: &str) -> Value {
    let path = api_examples_root().join(name);
    let contents = fs::read_to_string(path).unwrap();

    serde_json::from_str(&contents).unwrap()
}

#[tokio::test]
async fn search_endpoint_returns_expected_shape() {
    let query = "大家先统一想法";
    let response = app(AppState::new(fixture_cards()))
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/search?q={}&limit=1",
                    urlencoding::encode(query)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload, load_expected_json("search-success.json"));
}

#[tokio::test]
async fn search_endpoint_honors_limit_parameter() {
    let response = app(AppState::new(fixture_cards()))
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/search?q={}&limit=1",
                    urlencoding::encode("心态崩了")
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["query"], "心态崩了");
    assert_eq!(payload["results"].as_array().unwrap().len(), 1);
    assert_eq!(payload["results"][0]["id"], "meme_po_fang_le");
}

#[tokio::test]
async fn search_endpoint_matches_invalid_query_example() {
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

    assert_eq!(payload, load_expected_json("error-invalid-query.json"));
}

#[tokio::test]
async fn search_endpoint_matches_invalid_limit_example() {
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

    assert_eq!(payload, load_expected_json("error-invalid-limit.json"));
}
