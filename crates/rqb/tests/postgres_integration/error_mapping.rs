use crate::common::{self, order_items, organizations, products};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn postgres_version_is_18() {
    let pool = common::pool().await;
    common::assert_postgres_18(&pool).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn fetch_one_maps_no_rows_to_not_found() {
    let pool = common::pool().await;

    let err = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(Uuid::new_v4()))
        .fetch_one_scalar::<Uuid>(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::NotFound));
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn unique_violation_maps_constraint_name() {
    let pool = common::pool().await;

    let err = rqb::insert(products::table())
        .set_many((
            products::ID.set(Uuid::new_v4()),
            products::SKU.set("CAM-001".to_owned()),
            products::NAME.set("Duplicate Camera".to_owned()),
            products::PRICE_CENTS.set(100_i64),
        ))
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::UniqueViolation { .. }));
    assert_eq!(err.code(), Some("23505"));
    assert_eq!(err.constraint_name(), Some("products_sku_key"));
    assert!(!err.is_retryable());
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn pk_violation_maps_to_unique_violation() {
    let pool = common::pool().await;

    let err = rqb::insert(products::table())
        .set_many((
            products::ID.set(common::uuid("20000000-0000-0000-0000-000000000001")),
            products::SKU.set(common::unique_text("pk")),
            products::NAME.set("Duplicate primary key".to_owned()),
            products::PRICE_CENTS.set(100_i64),
        ))
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::UniqueViolation { .. }));
    assert_eq!(err.constraint_name(), Some("products_pkey"));
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn foreign_key_violation_maps_constraint_name() {
    let pool = common::pool().await;

    let err = rqb::insert(order_items::table())
        .set_many((
            order_items::ID.set(Uuid::new_v4()),
            order_items::ORDER_ID.set(Uuid::new_v4()),
            order_items::PRODUCT_ID.set(common::uuid("20000000-0000-0000-0000-000000000001")),
            order_items::QUANTITY.set(1_i32),
            order_items::UNIT_PRICE_CENTS.set(100_i64),
        ))
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::ForeignKeyViolation { .. }));
    assert_eq!(err.code(), Some("23503"));
    assert_eq!(err.constraint_name(), Some("order_items_order_id_fkey"));
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn not_null_violation_maps_column_name() {
    let pool = common::pool().await;

    let err = rqb::insert(products::table())
        .set_many((
            products::ID.set(Uuid::new_v4()),
            products::SKU.set(common::unique_text("not-null")),
            products::PRICE_CENTS.set(100_i64),
        ))
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::NotNullViolation { .. }));
    assert_eq!(err.code(), Some("23502"));
    assert_eq!(err.column_name(), Some("name"));
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn check_violation_maps_constraint_name() {
    let pool = common::pool().await;

    let err = rqb::insert(products::table())
        .set_many((
            products::ID.set(Uuid::new_v4()),
            products::SKU.set(common::unique_text("check")),
            products::NAME.set("Invalid price".to_owned()),
            products::PRICE_CENTS.set(-1_i64),
        ))
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::CheckViolation { .. }));
    assert_eq!(err.code(), Some("23514"));
    assert_eq!(
        err.constraint_name(),
        Some("products_price_cents_non_negative")
    );
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn referenced_parent_delete_reports_foreign_key_violation() {
    let pool = common::pool().await;

    let err = rqb::delete_from(organizations::table())
        .filter(organizations::ID.eq(common::uuid("00000000-0000-0000-0000-000000000001")))
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(matches!(err, rqb::Error::ForeignKeyViolation { .. }));
    assert_eq!(err.code(), Some("23503"));
    assert_eq!(
        err.constraint_name(),
        Some("app_users_organization_id_fkey")
    );
}
