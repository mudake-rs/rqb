use crate::common::products;
use futures_util::TryStreamExt;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    sku: String,
    name: String,
    price_cents: i64,
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn fetch_stream_as_yields_rows() {
    let pool = crate::common::pool().await;
    let built = rqb::select(products::table())
        .columns((
            products::ID,
            products::SKU,
            products::NAME,
            products::PRICE_CENTS,
        ))
        .order_asc(products::SKU)
        .limit(3)
        .build()
        .unwrap();

    let rows = built
        .fetch_stream_as::<ProductRow>(&pool)
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    assert_eq!(rows.len(), 3);
    assert!(rows.windows(2).all(|pair| pair[0].sku <= pair[1].sku));
    assert!(rows.iter().all(|row| row.price_cents > 0));
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn fetch_stream_scalar_yields_values() {
    let pool = crate::common::pool().await;
    let built = rqb::select(products::table())
        .column(products::SKU)
        .order_asc(products::SKU)
        .limit(3)
        .build()
        .unwrap();

    let values = built
        .fetch_stream_scalar::<String>(&pool)
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    assert_eq!(values, vec!["BAG-001", "CAM-001", "MIC-001"]);
}
