use std::str::FromStr;

use crate::common::{self, products};
use serde_json::json;
use sqlx::types::BigDecimal;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn bind_uuid_round_trip() {
    let pool = common::pool().await;
    let value = Uuid::new_v4();

    let decoded = rqb::select(products::table())
        .expr(value)
        .limit(1)
        .fetch_one_scalar::<Uuid>(&pool)
        .await
        .unwrap();

    assert_eq!(decoded, value);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn bind_datetime_round_trip() {
    let pool = common::pool().await;
    let value = common::utc("2026-05-11T12:34:56Z");

    let decoded = rqb::select(products::table())
        .expr(value)
        .limit(1)
        .fetch_one_scalar::<chrono::DateTime<chrono::Utc>>(&pool)
        .await
        .unwrap();

    assert_eq!(decoded, value);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn bind_jsonb_round_trip() {
    let pool = common::pool().await;
    let value = json!({ "a": 1, "b": ["x", "y"] });

    let decoded = rqb::select(products::table())
        .expr(value.clone())
        .limit(1)
        .fetch_one_scalar::<serde_json::Value>(&pool)
        .await
        .unwrap();

    assert_eq!(decoded, value);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn bind_bytea_round_trip() {
    let pool = common::pool().await;
    let value = vec![0xde, 0xad, 0xbe, 0xef];

    let decoded = rqb::select(products::table())
        .expr(value.clone())
        .limit(1)
        .fetch_one_scalar::<Vec<u8>>(&pool)
        .await
        .unwrap();

    assert_eq!(decoded, value);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn bind_bigdecimal_round_trip() {
    let pool = common::pool().await;
    let value = BigDecimal::from_str("1234.5678").unwrap();

    let decoded = rqb::select(products::table())
        .expr(value.clone())
        .limit(1)
        .fetch_one_scalar::<BigDecimal>(&pool)
        .await
        .unwrap();

    assert_eq!(decoded, value);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn bind_text_array_round_trip() {
    let pool = common::pool().await;
    let value = vec!["alpha".to_owned(), "beta".to_owned()];

    let decoded = rqb::raw("SELECT ?::text[]")
        .bind(value.clone())
        .fetch_one_scalar::<Vec<String>>(&pool)
        .await
        .unwrap();

    assert_eq!(decoded, value);
}
