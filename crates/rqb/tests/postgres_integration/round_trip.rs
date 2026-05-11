use crate::common::{self, products};
use rqb::prelude::*;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    sku: String,
    name: String,
    price_cents: i64,
}

#[derive(rqb::Insertable)]
#[rqb(table = crate::common::products)]
struct NewProduct {
    id: Uuid,
    sku: String,
    name: String,
    price_cents: i64,
}

#[derive(rqb::Changeset)]
#[rqb(table = crate::common::products)]
struct ProductPatch {
    name: Option<String>,
    price_cents: Option<i64>,
}

static AMOUNT_META: Meta = Meta::col("amount", "int8").ops(OpSet::ordered());
const AMOUNT: Field<i64> = Field::new(&AMOUNT_META);

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn select_returns_decoded_rows() {
    let pool = common::pool().await;

    let rows = rqb::select(products::table())
        .columns((
            products::ID,
            products::SKU,
            products::NAME,
            products::PRICE_CENTS,
        ))
        .order_asc(products::SKU)
        .limit(3)
        .fetch_all_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].sku, "BAG-001");
    assert_eq!(rows[1].sku, "CAM-001");
    assert_eq!(rows[2].sku, "MIC-001");
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn insert_with_returning_decodes() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let product = NewProduct {
        id,
        sku: common::unique_text("insert-returning"),
        name: "Inserted product".to_owned(),
        price_cents: 321,
    };

    let row = rqb::insert(products::table())
        .values(&product)
        .returning_all()
        .fetch_one_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.sku, product.sku);
    assert_eq!(row.name, "Inserted product");
    assert_eq!(row.price_cents, 321);

    common::delete_product(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn update_with_returning_decodes_changeset() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("update-returning");
    common::insert_product(&pool, id, &sku, "Before patch", 100).await;

    let patch = ProductPatch {
        name: Some("After patch".to_owned()),
        price_cents: None,
    };
    let row = rqb::update(products::table())
        .patch(&patch)
        .filter(products::ID.eq(id))
        .returning_all()
        .fetch_one_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.sku, sku);
    assert_eq!(row.name, "After patch");
    assert_eq!(row.price_cents, 100);

    common::delete_product(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn delete_with_returning_decodes() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("delete-returning");
    common::insert_product(&pool, id, &sku, "Delete me", 444).await;

    let row = rqb::delete_from(products::table())
        .filter(products::ID.eq(id))
        .returning_all()
        .fetch_one_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.sku, sku);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn on_conflict_do_update_excluded_round_trip() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("upsert");
    common::insert_product(&pool, id, &sku, "Original", 100).await;

    let row = rqb::insert(products::table())
        .set_many((
            products::ID.set(Uuid::new_v4()),
            products::SKU.set(sku.clone()),
            products::NAME.set("Updated".to_owned()),
            products::PRICE_CENTS.set(250_i64),
        ))
        .on_conflict(products::SKU)
        .do_update_excluded((products::NAME, products::PRICE_CENTS))
        .returning_all()
        .fetch_one_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(row.id, id);
    assert_eq!(row.sku, sku);
    assert_eq!(row.name, "Updated");
    assert_eq!(row.price_cents, 250);

    common::delete_product(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn cte_on_update_round_trip() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("cte-update");
    common::insert_product(&pool, id, &sku, "CTE update", 100).await;

    let bump = rqb::cte(
        "bump",
        rqb::raw("SELECT ?::bigint AS amount").bind(25_i64),
        AMOUNT,
    );
    let amount = rqb::scalar_subquery(rqb::select(&bump).column(AMOUNT));

    let row = rqb::update(products::table())
        .with(bump)
        .set(products::PRICE_CENTS.set_expr(products::PRICE_CENTS.expr().op("+", amount)))
        .filter(products::ID.eq(id))
        .returning_all()
        .fetch_one_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(row.price_cents, 125);

    common::delete_product(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn merge_when_matched_and_not_matched_round_trip() {
    let pool = common::pool().await;
    let existing_id = Uuid::new_v4();
    let existing_sku = common::unique_text("merge-existing");
    let inserted_id = Uuid::new_v4();
    let inserted_sku = common::unique_text("merge-inserted");
    common::insert_product(&pool, existing_id, &existing_sku, "Before merge", 100).await;

    let incoming = rqb::raw_source(
        "VALUES (?, ?, ?, ?), (?, ?, ?, ?)",
        "incoming",
        vec![
            Param::typed(existing_id),
            Param::typed(existing_sku.clone()),
            Param::typed("After merge".to_owned()),
            Param::typed(150_i64),
            Param::typed(inserted_id),
            Param::typed(inserted_sku.clone()),
            Param::typed("Inserted by merge".to_owned()),
            Param::typed(200_i64),
        ],
        (
            products::ID,
            products::SKU,
            products::NAME,
            products::PRICE_CENTS,
        ),
    );

    rqb::merge_into(
        products::table().alias("target"),
        incoming,
        products::SKU
            .at("target")
            .eq_field(products::SKU.at("incoming")),
    )
    .when_matched()
    .update((
        products::NAME.set_expr(products::NAME.at("incoming")),
        products::PRICE_CENTS.set_expr(products::PRICE_CENTS.at("incoming")),
    ))
    .when_not_matched()
    .insert((
        products::ID.set_expr(products::ID.at("incoming")),
        products::SKU.set_expr(products::SKU.at("incoming")),
        products::NAME.set_expr(products::NAME.at("incoming")),
        products::PRICE_CENTS.set_expr(products::PRICE_CENTS.at("incoming")),
    ))
    .execute(&pool)
    .await
    .unwrap();

    let rows = rqb::select(products::table())
        .columns((
            products::ID,
            products::SKU,
            products::NAME,
            products::PRICE_CENTS,
        ))
        .filter(products::SKU.in_list([existing_sku.clone(), inserted_sku.clone()]))
        .order_asc(products::SKU)
        .fetch_all_as::<ProductRow>(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter().any(|row| row.id == existing_id
            && row.name == "After merge"
            && row.price_cents == 150)
    );
    assert!(rows.iter().any(|row| row.id == inserted_id
        && row.name == "Inserted by merge"
        && row.price_cents == 200));

    common::delete_product(&pool, existing_id).await;
    common::delete_product(&pool, inserted_id).await;
}
