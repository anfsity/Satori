use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use satori_api::{AppState, app};
use satori_core::{JargonCard, load_cards_from_reader};
use serde_json::Value;
use std::{fs::File, path::PathBuf};
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

    assert_eq!(payload["status"], "ok");
}

fn fixture_cards() -> Vec<JargonCard> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("cards.json");
    let file = File::open(path).unwrap();

    load_cards_from_reader(file).unwrap()
}

#[tokio::test]
async fn search_endpoint_returns_expected_shape() {
    let response = app(AppState::new(fixture_cards()))
        .oneshot(
            Request::builder()
                .uri("/api/search?q=大家先统一想法&limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["query"], "大家先统一想法");
    assert!(payload["results"].is_array());
    assert_eq!(payload["results"][0]["term"], "拉通对齐");
    assert!(payload["results"][0]["score"].is_number());
}
