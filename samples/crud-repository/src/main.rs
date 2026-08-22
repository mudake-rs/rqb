use chrono::{DateTime, Utc};
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

macro_rules! crud_repository {
    (
        $repo:ident {
            table: $table:ident,
            id: $id:ident,
            row: $row:ty,
            new: $new:ty,
            patch: $patch:ty $(,)?
        }
    ) => {
        struct $repo;

        impl $repo {
            // This sample pins repository IDs to Uuid to keep the macro focused
            // on rqb query shape rather than becoming a generic ORM framework.
            async fn find(db: impl PgExecutor<'_>, id: Uuid) -> rqb::Result<$row> {
                select($table::table())
                    .filter($table::$id.eq(id))
                    .fetch_one_as::<$row>(db)
                    .await
            }

            async fn list(db: impl PgExecutor<'_>, limit: u32) -> rqb::Result<Vec<$row>> {
                select($table::table())
                    .order_asc($table::$id)
                    .limit(limit)
                    .fetch_all_as::<$row>(db)
                    .await
            }

            async fn create(db: impl PgExecutor<'_>, id: Uuid, input: &$new) -> rqb::Result<$row> {
                insert($table::table())
                    .set($table::$id.set(id))
                    .values(input)
                    .returning_all()
                    .fetch_one_as::<$row>(db)
                    .await
            }

            async fn patch(
                db: impl PgExecutor<'_>,
                id: Uuid,
                patch: &$patch,
            ) -> rqb::Result<Option<$row>> {
                let assignments = patch.changeset_assignments();
                if assignments.is_empty() {
                    return Ok(None);
                }

                update($table::table())
                    .set_many(assignments)
                    .filter($table::$id.eq(id))
                    .returning_all()
                    .fetch_optional_as::<$row>(db)
                    .await
            }

            async fn delete(db: impl PgExecutor<'_>, id: Uuid) -> rqb::Result<Option<Uuid>> {
                delete_from($table::table())
                    .filter($table::$id.eq(id))
                    .returning($table::$id)
                    .fetch_optional_scalar::<Uuid>(db)
                    .await
            }
        }
    };
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserRow {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: String,
    display_name: String,
    active: bool,
    created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[rqb(table = users)]
struct NewUser {
    organization_id: Uuid,
    email: String,
    status: String,
    display_name: String,
    active: bool,
}

#[derive(Changeset)]
#[rqb(table = users)]
struct UserPatch {
    email: Option<String>,
    status: Option<String>,
    display_name: Option<String>,
    active: Option<bool>,
}

crud_repository! {
    UserRepo {
        table: users,
        id: ID,
        row: UserRow,
        new: NewUser,
        patch: UserPatch,
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new().connect_lazy("postgres://rqb:rqb@localhost/rqb")?;
    let user_id = Uuid::nil();
    let new_user = NewUser {
        organization_id: Uuid::nil(),
        email: "ada@example.com".to_owned(),
        status: "active".to_owned(),
        display_name: "Ada".to_owned(),
        active: true,
    };
    let patch = UserPatch {
        email: None,
        status: Some("disabled".to_owned()),
        display_name: Some("Ada Lovelace".to_owned()),
        active: Some(false),
    };

    // These futures compile-check the executor shapes but are never polled, so
    // the sample stays database-free when run.
    let pool_future = pool_flow(&pool, user_id, &new_user, &patch);
    drop(pool_future);

    let tx_future = transaction_flow(&pool, user_id, &new_user, &patch);
    drop(tx_future);

    Ok(())
}

async fn pool_flow(
    pool: &PgPool,
    user_id: Uuid,
    new_user: &NewUser,
    patch: &UserPatch,
) -> rqb::Result<()> {
    let _found = UserRepo::find(pool, user_id).await?;
    let _rows = UserRepo::list(pool, 20).await?;
    let _created = UserRepo::create(pool, user_id, new_user).await?;
    let _patched = UserRepo::patch(pool, user_id, patch).await?;
    let _deleted_id = UserRepo::delete(pool, user_id).await?;
    Ok(())
}

async fn transaction_flow(
    pool: &PgPool,
    user_id: Uuid,
    new_user: &NewUser,
    patch: &UserPatch,
) -> rqb::Result<()> {
    let mut tx = pool.begin().await?;

    let _created = UserRepo::create(&mut *tx, user_id, new_user).await?;
    let _patched = deactivate_and_cancel(&mut tx, user_id, patch).await?;

    tx.commit().await.map_err(rqb::Error::from)
}

async fn deactivate_and_cancel(
    mut db: impl ExecutorSource,
    user_id: Uuid,
    patch: &UserPatch,
) -> rqb::Result<Option<UserRow>> {
    let user = UserRepo::patch(db.exec(), user_id, patch).await?;

    update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .execute(db.exec())
        .await?;

    Ok(user)
}
