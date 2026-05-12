#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rqb::prelude::*;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

pub async fn pool() -> PgPool {
    let url = std::env::var("RQB_TEST_DATABASE_URL")
        .expect("RQB_TEST_DATABASE_URL must be set for postgres integration tests");
    PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .expect("connect to Postgres integration database")
}

pub async fn assert_postgres_18(pool: &PgPool) {
    let version: String = rqb::raw("SHOW server_version_num")
        .fetch_one_scalar(pool)
        .await
        .expect("SHOW server_version_num");
    let version = version
        .parse::<u32>()
        .expect("server_version_num must be an integer");
    assert!(
        (180000..190000).contains(&version),
        "integration tests must run against Postgres 18, got server_version_num={version}"
    );
}

pub fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid fixture UUID")
}

pub fn unique_text(label: &str) -> String {
    format!("rqb-it-{label}-{}", Uuid::new_v4())
}

pub fn utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("valid RFC3339 timestamp")
        .with_timezone(&Utc)
}

pub async fn insert_product(pool: &PgPool, id: Uuid, sku: &str, name: &str, price_cents: i64) {
    rqb::insert(products::table())
        .set_many((
            products::ID.set(id),
            products::SKU.set(sku.to_owned()),
            products::NAME.set(name.to_owned()),
            products::PRICE_CENTS.set(price_cents),
        ))
        .execute(pool)
        .await
        .expect("insert integration product");
}

pub async fn delete_product(pool: &PgPool, id: Uuid) {
    let _ = rqb::delete_from(products::table())
        .filter(products::ID.eq(id))
        .execute(pool)
        .await;
}

pub mod organizations {
    use super::*;

    pub static ID_META: Meta = Meta::col("id", "uuid").ops(OpSet::equality());
    pub static SLUG_META: Meta = Meta::col("slug", "text").ops(OpSet::text());
    pub static NAME_META: Meta = Meta::col("name", "text").ops(OpSet::text());

    pub const ID: Field<Uuid> = Field::new(&ID_META);
    pub const SLUG: Field<String> = Field::new(&SLUG_META);
    pub const NAME: Field<String> = Field::new(&NAME_META);

    pub static FIELDS: [&Meta; 3] = [&ID_META, &SLUG_META, &NAME_META];

    pub fn table() -> Source {
        rqb::table("public.organizations", &FIELDS)
    }
}

pub mod products {
    use super::*;

    pub static ID_META: Meta = Meta::col("id", "uuid")
        .ops(OpSet::equality())
        .json(JsonKind::Uuid);
    pub static SKU_META: Meta = Meta::col("sku", "text")
        .ops(OpSet::text())
        .json(JsonKind::Text);
    pub static NAME_META: Meta = Meta::col("name", "text")
        .ops(OpSet::text())
        .json(JsonKind::Text);
    pub static PRICE_CENTS_META: Meta = Meta::col("price_cents", "int8")
        .ops(OpSet::ordered())
        .json(JsonKind::BigInt);

    pub const ID: Field<Uuid> = Field::new(&ID_META);
    pub const SKU: Field<String> = Field::new(&SKU_META);
    pub const NAME: Field<String> = Field::new(&NAME_META);
    pub const PRICE_CENTS: Field<i64> = Field::new(&PRICE_CENTS_META);

    pub static FIELDS: [&Meta; 4] = [&ID_META, &SKU_META, &NAME_META, &PRICE_CENTS_META];

    pub fn table() -> Source {
        rqb::table("public.products", &FIELDS)
    }
}

pub mod order_items {
    use super::*;

    pub static ID_META: Meta = Meta::col("id", "uuid").ops(OpSet::equality());
    pub static ORDER_ID_META: Meta = Meta::col("order_id", "uuid").ops(OpSet::equality());
    pub static PRODUCT_ID_META: Meta = Meta::col("product_id", "uuid").ops(OpSet::equality());
    pub static QUANTITY_META: Meta = Meta::col("quantity", "int4").ops(OpSet::ordered());
    pub static UNIT_PRICE_CENTS_META: Meta =
        Meta::col("unit_price_cents", "int8").ops(OpSet::ordered());

    pub const ID: Field<Uuid> = Field::new(&ID_META);
    pub const ORDER_ID: Field<Uuid> = Field::new(&ORDER_ID_META);
    pub const PRODUCT_ID: Field<Uuid> = Field::new(&PRODUCT_ID_META);
    pub const QUANTITY: Field<i32> = Field::new(&QUANTITY_META);
    pub const UNIT_PRICE_CENTS: Field<i64> = Field::new(&UNIT_PRICE_CENTS_META);

    pub static FIELDS: [&Meta; 5] = [
        &ID_META,
        &ORDER_ID_META,
        &PRODUCT_ID_META,
        &QUANTITY_META,
        &UNIT_PRICE_CENTS_META,
    ];

    pub fn table() -> Source {
        rqb::table("public.order_items", &FIELDS)
    }
}

pub mod order_search {
    use super::*;

    pub static ID_META: Meta = Meta::col("id", "uuid")
        .ops(OpSet::equality())
        .json(JsonKind::Uuid);
    pub static EMAIL_META: Meta = Meta::col("email", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    pub static STATUS_META: Meta = Meta::col("status", "order_status").ops(OpSet::ordered());
    pub static CHANNEL_META: Meta = Meta::col("channel", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    pub static TOTAL_CENTS_META: Meta = Meta::col("total_cents", "int8")
        .ops(OpSet::ordered())
        .json(JsonKind::BigInt);
    pub static CREATED_AT_META: Meta = Meta::col("created_at", "timestamptz")
        .ops(OpSet::ordered())
        .json(JsonKind::Timestamptz);

    pub const ID: Field<Uuid> = Field::new(&ID_META);
    pub const EMAIL: Field<String> = Field::new(&EMAIL_META);
    pub const STATUS: Field<String> = Field::new(&STATUS_META);
    pub const CHANNEL: Field<String> = Field::new(&CHANNEL_META);
    pub const TOTAL_CENTS: Field<i64> = Field::new(&TOTAL_CENTS_META);
    pub const CREATED_AT: Field<DateTime<Utc>> = Field::new(&CREATED_AT_META);

    pub static FIELDS: [&Meta; 6] = [
        &ID_META,
        &EMAIL_META,
        &STATUS_META,
        &CHANNEL_META,
        &TOTAL_CENTS_META,
        &CREATED_AT_META,
    ];

    pub fn view() -> Source {
        rqb::view("public.order_search_view", &FIELDS)
    }
}
