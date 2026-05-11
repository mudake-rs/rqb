use crate::common::{order_search, uuid};
use serde_json::json;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct SearchRow {
    id: Uuid,
    email: String,
    channel: String,
    total_cents: i64,
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn search_request_filter_and_sort_round_trip() {
    let pool = crate::common::pool().await;
    let request: rqb::SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "channel", "operator": "equals", "value": "web" },
        "sort": [{ "field": "total_cents", "dir": "desc" }],
        "limit": 2
    }))
    .unwrap();

    let rows = rqb::select(order_search::view())
        .columns((
            order_search::ID,
            order_search::EMAIL,
            order_search::CHANNEL,
            order_search::TOTAL_CENTS,
        ))
        .request(request)
        .unwrap()
        .fetch_all_as::<SearchRow>(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.channel == "web"));
    assert!(rows[0].total_cents >= rows[1].total_cents);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn search_request_multi_key_sort_round_trip() {
    let pool = crate::common::pool().await;
    let request: rqb::SearchRequest = serde_json::from_value(json!({
        "sort": [
            { "field": "channel", "dir": "asc" },
            { "field": "total_cents", "dir": "desc" }
        ],
        "limit": 4
    }))
    .unwrap();

    let rows = rqb::select(order_search::view())
        .columns((
            order_search::ID,
            order_search::EMAIL,
            order_search::CHANNEL,
            order_search::TOTAL_CENTS,
        ))
        .request(request)
        .unwrap()
        .fetch_all_as::<SearchRow>(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 4);
    assert!(rows.windows(2).all(|pair| {
        pair[0].channel < pair[1].channel
            || (pair[0].channel == pair[1].channel && pair[0].total_cents >= pair[1].total_cents)
    }));
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn search_request_limit_offset_round_trip() {
    let pool = crate::common::pool().await;
    let request: rqb::SearchRequest = serde_json::from_value(json!({
        "sort": [{ "field": "created_at", "dir": "asc" }],
        "limit": 2,
        "offset": 1
    }))
    .unwrap();

    let rows = rqb::select(order_search::view())
        .columns((
            order_search::ID,
            order_search::EMAIL,
            order_search::CHANNEL,
            order_search::TOTAL_CENTS,
        ))
        .request(request)
        .unwrap()
        .fetch_all_as::<SearchRow>(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, uuid("30000000-0000-0000-0000-000000000001"));
    assert_eq!(rows[1].id, uuid("30000000-0000-0000-0000-000000000002"));
}
