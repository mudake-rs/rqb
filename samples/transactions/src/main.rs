use rqb::dsl::{advisory_xact_lock_named, try_advisory_xact_lock, try_advisory_xact_lock_named};
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::orders;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

const ORDER_WORKER_LOCK: i32 = 10;

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

    let worker_future = run_exclusive_user_job(&pool, user_id);
    drop(worker_future);

    // The rendered statement matches the second helper in the transaction body.
    let cancel_sql = update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .build()?;

    let lock_sql = select(orders::table())
        .filter(orders::ID.eq(Uuid::nil()))
        .for_no_key_update()
        .nowait()
        .build()?;

    let advisory_sql = advisory_xact_lock_named("sample:user-job").build()?;
    let try_advisory_sql = try_advisory_xact_lock((ORDER_WORKER_LOCK, 42_i32)).build()?;

    assert_eq!(
        cancel_sql.sql,
        "UPDATE \"sample\".\"orders\" SET \"status\" = $1 WHERE (\"user_id\" = $2 AND \"status\" = $3)"
    );
    assert_eq!(cancel_sql.params.len(), 3);
    assert_eq!(
        lock_sql.sql,
        "SELECT \"id\", \"user_id\", \"status\", \"total_cents\", \"metadata\", \"tags\", \"created_at\" FROM \"sample\".\"orders\" WHERE \"id\" = $1 FOR NO KEY UPDATE NOWAIT"
    );
    assert_eq!(advisory_sql.sql, "SELECT pg_advisory_xact_lock($1)");
    assert_eq!(
        try_advisory_sql.sql,
        "SELECT pg_try_advisory_xact_lock($1, $2)"
    );

    println!("{}", cancel_sql.sql);
    println!("{}", lock_sql.sql);
    println!("{}", advisory_sql.sql);
    println!("{}", try_advisory_sql.sql);
    Ok(())
}

// `PgExecutor<'_>` keeps service helpers reusable: callers can pass a pool,
// connection, or the connection borrowed from a transaction.
async fn deactivate_user(db: impl PgExecutor<'_>, user_id: Uuid) -> rqb::Result<u64> {
    update(users::table())
        .set(users::ACTIVE.set(false))
        .filter(users::ID.eq(user_id))
        .execute(db)
        .await
}

async fn cancel_open_orders(db: impl PgExecutor<'_>, user_id: Uuid) -> rqb::Result<u64> {
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

async fn run_exclusive_user_job(pool: &PgPool, user_id: Uuid) -> rqb::Result<bool> {
    tx!(pool, |conn| {
        let acquired = try_advisory_xact_lock_named(format!("sample:user-job:{user_id}"))
            .fetch_one_scalar::<bool>(&mut *conn)
            .await?;
        if !acquired {
            return Ok(false);
        }

        deactivate_user(&mut *conn, user_id).await?;
        cancel_open_orders(conn, user_id).await?;
        Ok(true)
    })
    .await
}
