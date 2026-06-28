use rqb::prelude::*;
use rqb_sample_schema::{app_users as users, orders};
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

trait ExecutorSource {
    type Executor<'a>: PgExecutor<'a>
    where
        Self: 'a;

    fn exec(&mut self) -> Self::Executor<'_>;
}

impl ExecutorSource for &PgPool {
    type Executor<'a>
        = &'a PgPool
    where
        Self: 'a;

    fn exec(&mut self) -> Self::Executor<'_> {
        *self
    }
}

impl ExecutorSource for &mut PgConnection {
    type Executor<'a>
        = &'a mut PgConnection
    where
        Self: 'a;

    fn exec(&mut self) -> Self::Executor<'_> {
        &mut **self
    }
}

impl ExecutorSource for &mut PoolConnection<Postgres> {
    type Executor<'a>
        = &'a mut PgConnection
    where
        Self: 'a;

    fn exec(&mut self) -> Self::Executor<'_> {
        &mut **self
    }
}

impl ExecutorSource for &mut Transaction<'_, Postgres> {
    type Executor<'a>
        = &'a mut PgConnection
    where
        Self: 'a;

    fn exec(&mut self) -> Self::Executor<'_> {
        &mut **self
    }
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserRow {
    id: Uuid,
    email: String,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new().connect_lazy("postgres://rqb:rqb@localhost/rqb")?;
    let user_id = Uuid::nil();

    // The futures below compile-check the service shapes but are never polled,
    // so this sample remains database-free when run.
    let pool_future = pool_flow(&pool, user_id);
    drop(pool_future);

    let connection_future = acquired_connection_flow(&pool, user_id);
    drop(connection_future);

    let transaction_future = explicit_transaction_flow(&pool, user_id);
    drop(transaction_future);

    let tx_macro_future = tx_macro_flow(&pool, user_id);
    drop(tx_macro_future);

    let find_sql = find_user_query(user_id).build()?;
    assert_eq!(
        find_sql.sql,
        "SELECT \"id\", \"email\" FROM \"sample\".\"app_users\" WHERE \"id\" = $1"
    );
    assert_eq!(find_sql.params.len(), 1);

    println!("{}", find_sql.sql);
    Ok(())
}

fn find_user_query(id: Uuid) -> Select {
    select(users::table())
        .columns((users::ID, users::EMAIL))
        .filter(users::ID.eq(id))
}

async fn find_user(db: impl PgExecutor<'_>, id: Uuid) -> rqb::Result<UserRow> {
    find_user_query(id).fetch_one_as::<UserRow>(db).await
}

async fn deactivate_and_cancel(mut db: impl ExecutorSource, user_id: Uuid) -> rqb::Result<()> {
    // This is the narrow case a GAT wrapper can express and `impl PgExecutor<'_>`
    // cannot: the helper needs to reborrow the executor source twice. Calling it
    // with `&PgPool` would let each statement acquire independently, and using
    // an acquired connection still would not make the pair atomic. Use a
    // transaction when the two writes must commit or roll back together.
    update(users::table())
        .set(users::ACTIVE.set(false))
        .filter(users::ID.eq(user_id))
        .execute(db.exec())
        .await?;

    update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .execute(db.exec())
        .await?;

    Ok(())
}

async fn pool_flow(pool: &PgPool, user_id: Uuid) -> rqb::Result<()> {
    let _user = find_user(pool, user_id).await?;
    Ok(())
}

async fn acquired_connection_flow(pool: &PgPool, user_id: Uuid) -> rqb::Result<()> {
    let mut conn = pool.acquire().await?;

    deactivate_and_cancel(&mut conn, user_id).await
}

async fn explicit_transaction_flow(pool: &PgPool, user_id: Uuid) -> rqb::Result<()> {
    let mut tx = pool.begin().await?;

    let _user = find_user(&mut *tx, user_id).await?;
    deactivate_and_cancel(&mut tx, user_id).await?;

    tx.commit().await.map_err(rqb::Error::from)
}

async fn tx_macro_flow(pool: &PgPool, user_id: Uuid) -> rqb::Result<()> {
    tx!(pool, |conn| {
        let _user = find_user(&mut *conn, user_id).await?;
        deactivate_and_cancel(conn, user_id).await
    })
    .await
}
