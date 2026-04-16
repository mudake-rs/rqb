use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use uuid::Uuid;

use crate::services::orders;
use crate::types::{CreateUser, UserRow};

pub async fn find<'e>(db: impl PgExecutor<'e>, id: Uuid) -> rqb::Result<UserRow> {
    select(users::table())
        .filter(users::ID.eq(id))
        .fetch_one_as::<UserRow>(db)
        .await
}

pub async fn search<'e>(
    db: impl PgExecutor<'e>,
    request: SearchRequest,
) -> rqb::Result<Vec<UserRow>> {
    select(users::table())
        .filter(users::ACTIVE.eq(true))
        .request(request)?
        .fetch_all_as::<UserRow>(db)
        .await
}

pub async fn create<'e>(db: impl PgExecutor<'e>, input: CreateUser) -> rqb::Result<UserRow> {
    // DTO-derived values and explicit server-owned assignments compose in one
    // insert. Later assignments for the same field intentionally win.
    insert(users::table())
        .set(users::ID.set(Uuid::new_v4()))
        .values(&input)
        .set(users::ACTIVE.set(true))
        .returning_all()
        .fetch_one_as::<UserRow>(db)
        .await
}

pub async fn deactivate(pool: &PgPool, id: Uuid) -> rqb::Result<()> {
    // `tx!` gives the closure a transaction connection; the same service
    // functions accept it through `PgExecutor`.
    tx!(pool, |conn| {
        update(users::table())
            .set(users::ACTIVE.set(false))
            .filter(users::ID.eq(id))
            .execute(&mut *conn)
            .await?;

        orders::cancel_open_for_user(&mut *conn, id).await?;
        Ok(())
    })
    .await
}
