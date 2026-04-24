use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use satori_api::{AppState, app};
use satori_core::{JargonCard, load_cards_from_reader};
use serde_json::Value;
use std::{collections::BTreeSet, fs, fs::File, path::PathBuf};
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

#[tokio::test]
async fn search_endpoint_returns_expected_shape() {
    let cards = fixture_cards();
    let query = cards[0].plain.clone();
    let expected_term = cards[0].term.clone();
    let response = app(AppState::new(fixture_cards()))
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/search?q={}&limit=5",
                    urlencoding::encode(&query)
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["query"], query);
    assert!(payload["results"].is_array());
    assert_eq!(payload["results"][0]["term"], expected_term);
    assert!(payload["results"][0]["score"].is_number());
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

#[test]
fn processed_cards_match_fixture_card_ids() {
    let fixture_ids = load_card_ids(
        repo_root()
            .join("tests")
            .join("fixtures")
            .join("cards.json"),
    );
    let processed_ids = load_card_ids(
        repo_root()
            .join("data")
            .join("processed")
            .join("cards.json"),
    );

    let missing_in_processed = fixture_ids
        .difference(&processed_ids)
        .cloned()
        .collect::<Vec<_>>();
    let missing_in_fixtures = processed_ids
        .difference(&fixture_ids)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing_in_processed.is_empty() && missing_in_fixtures.is_empty(),
        "card corpus drift detected: missing in processed = {:?}, missing in fixtures = {:?}",
        missing_in_processed,
        missing_in_fixtures
    );
}

fn load_card_ids(path: PathBuf) -> BTreeSet<String> {
    let contents = fs::read_to_string(path).unwrap();
    let cards: Vec<JargonCard> = serde_json::from_str(&contents).unwrap();

    cards.into_iter().map(|card| card.id).collect()
}
