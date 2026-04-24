use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use satori_api::{AppState, app};
use satori_core::JargonCard;
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::PathBuf};
use tower::ServiceExt;

#[derive(Debug, Deserialize)]
struct RegressionCase {
    query: String,
    expected_id: String,
    max_rank: usize,
}

#[tokio::test]
async fn regression_queries_keep_expected_ids_in_top_results() {
    let cards = load_cards();
    let cases = load_regression_cases();

    for case in cases {
        let response = app(AppState::new(cards.clone()))
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/search?q={}",
                        urlencoding::encode(&case.query)
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let result_ids = payload["results"]
            .as_array()
            .unwrap()
            .iter()
            .take(case.max_rank)
            .filter_map(|item| item["id"].as_str())
            .collect::<Vec<_>>();

        assert!(
            result_ids.contains(&case.expected_id.as_str()),
            "query {:?} did not keep expected id {:?} within top {} results: {:?}",
            case.query,
            case.expected_id,
            case.max_rank,
            result_ids
        );
    }
}

fn load_cards() -> Vec<JargonCard> {
    serde_json::from_str(&read_fixture("cards.json")).unwrap()
}

fn load_regression_cases() -> Vec<RegressionCase> {
    serde_json::from_str(&read_fixture("regression.json")).unwrap()
}

fn read_fixture(name: &str) -> String {
    let path = fixture_root().join(name);
    fs::read_to_string(path).unwrap()
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}
