use crate::common::{self, products};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn tx_macro_commits_successful_work() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("tx-commit");

    let tx_pool = pool.clone();
    rqb::tx!(&tx_pool, |conn| {
        rqb::insert(products::table())
            .set_many((
                products::ID.set(id),
                products::SKU.set(sku.clone()),
                products::NAME.set("Committed".to_owned()),
                products::PRICE_CENTS.set(123_i64),
            ))
            .execute(&mut *conn)
            .await?;
        Ok(())
    })
    .await
    .unwrap();

    let found = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(id))
        .fetch_one_scalar::<Uuid>(&pool)
        .await
        .unwrap();

    assert_eq!(found, id);
    common::delete_product(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn tx_macro_rolls_back_when_db_error_escapes() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("tx-rollback");

    let tx_pool = pool.clone();
    let err = rqb::tx!(&tx_pool, |conn| {
        rqb::insert(products::table())
            .set_many((
                products::ID.set(id),
                products::SKU.set(sku.clone()),
                products::NAME.set("Rolled back".to_owned()),
                products::PRICE_CENTS.set(123_i64),
            ))
            .execute(&mut *conn)
            .await?;

        rqb::insert(products::table())
            .set_many((
                products::ID.set(Uuid::new_v4()),
                products::SKU.set("CAM-001".to_owned()),
                products::NAME.set("Duplicate".to_owned()),
                products::PRICE_CENTS.set(1_i64),
            ))
            .execute(conn)
            .await?;

        Ok(())
    })
    .await
    .unwrap_err();

    assert!(matches!(err, rqb::Error::UniqueViolation(_)));

    let found = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(id))
        .fetch_optional_scalar::<Uuid>(&pool)
        .await
        .unwrap();

    assert_eq!(found, None);
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn select_for_update_nowait_fails_on_locked_row() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("nowait");
    common::insert_product(&pool, id, &sku, "Locked", 100).await;

    let mut tx1 = pool.begin().await.unwrap();
    let locked = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(id))
        .for_update()
        .fetch_one_scalar::<Uuid>(&mut *tx1)
        .await
        .unwrap();
    assert_eq!(locked, id);

    let mut tx2 = pool.begin().await.unwrap();
    let err = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(id))
        .for_update()
        .nowait()
        .fetch_one_scalar::<Uuid>(&mut *tx2)
        .await
        .unwrap_err();

    assert_eq!(err.code(), Some("55P03"));

    tx2.rollback().await.unwrap();
    tx1.rollback().await.unwrap();
    common::delete_product(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn select_for_update_skip_locked_skips_locked_row() {
    let pool = common::pool().await;
    let id = Uuid::new_v4();
    let sku = common::unique_text("skip-locked");
    common::insert_product(&pool, id, &sku, "Locked", 100).await;

    let mut tx1 = pool.begin().await.unwrap();
    let locked = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(id))
        .for_update()
        .fetch_one_scalar::<Uuid>(&mut *tx1)
        .await
        .unwrap();
    assert_eq!(locked, id);

    let mut tx2 = pool.begin().await.unwrap();
    let skipped = rqb::select(products::table())
        .column(products::ID)
        .filter(products::ID.eq(id))
        .for_update()
        .skip_locked()
        .fetch_optional_scalar::<Uuid>(&mut *tx2)
        .await
        .unwrap();

    assert_eq!(skipped, None);

    tx2.rollback().await.unwrap();
    tx1.rollback().await.unwrap();
    common::delete_product(&pool, id).await;
}
