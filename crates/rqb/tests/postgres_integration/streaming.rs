use crate::common::products;
use futures_util::TryStreamExt;
use rqb::prelude::*;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    sku: String,
    name: String,
    price_cents: i64,
}

static N_META: Meta = Meta::col("n", "int4").ops(OpSet::ordered());
const N: Field<i32> = Field::new(&N_META);

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

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn fetch_stream_pool_scalar_owns_query_and_yields_many_rows() {
    let pool = crate::common::pool().await;

    let values = rqb::select(rqb::generate_series_source(1_i32, 150_i32, "g", N))
        .fetch_stream_pool_scalar::<i32>(pool.clone())
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    assert_eq!(values.len(), 150);
    assert_eq!(values[0], 1);
    assert_eq!(values[149], 150);
}
