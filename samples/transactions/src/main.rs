use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::orders;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new().connect_lazy("postgres://rqb:rqb@localhost/rqb")?;
    let user_id = Uuid::nil();

    // The focused sample renders SQL without connecting to a database. The
    // transaction functions below contain the real `.await?` flow; the futures
    // are dropped here only to keep `cargo run` database-free.
    let closure_future = closure_transaction(&pool, user_id);
    drop(closure_future);

    let explicit_future = explicit_transaction(&pool, user_id);
    drop(explicit_future);

    // The rendered statement matches the second helper in the transaction body.
    let cancel_sql = update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .build()?;

    assert_eq!(
        cancel_sql.sql,
        "UPDATE \"sample\".\"orders\" SET \"status\" = $1 WHERE (\"user_id\" = $2 AND \"status\" = $3)"
    );
    assert_eq!(cancel_sql.params.len(), 3);

    println!("{}", cancel_sql.sql);
    Ok(())
}

// `PgExecutor` keeps service functions reusable: callers can pass a pool, a
// connection, or the connection borrowed from a transaction.
async fn deactivate_user<'e>(db: impl PgExecutor<'e>, user_id: Uuid) -> rqb::Result<u64> {
    update(users::table())
        .set(users::ACTIVE.set(false))
        .filter(users::ID.eq(user_id))
        .execute(db)
        .await
}

async fn cancel_open_orders<'e>(db: impl PgExecutor<'e>, user_id: Uuid) -> rqb::Result<u64> {
    update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .execute(db)
        .await
}

async fn closure_transaction(pool: &PgPool, user_id: Uuid) -> rqb::Result<Uuid> {
    tx!(pool, |conn| {
        deactivate_user(&mut *conn, user_id).await?;
        cancel_open_orders(conn, user_id).await?;
        Ok(user_id)
    })
    .await
}

async fn explicit_transaction(pool: &PgPool, user_id: Uuid) -> rqb::Result<()> {
    let mut tx = pool.begin().await?;

    deactivate_user(&mut *tx, user_id).await?;
    cancel_open_orders(&mut *tx, user_id).await?;

    tx.commit().await.map_err(rqb::Error::from)
}
