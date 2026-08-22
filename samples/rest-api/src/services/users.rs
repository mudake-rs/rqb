use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use uuid::Uuid;

use crate::services::orders;
use crate::types::{CreateUser, PatchUser, UserRow};

const DEFAULT_SEARCH_LIMIT: u32 = 100;
const MAX_SEARCH_LIMIT: u32 = 100;

pub fn find_query(id: Uuid) -> Select {
    select(users::table()).filter(users::ID.eq(id))
}

pub async fn find(db: &PgPool, id: Uuid) -> rqb::Result<UserRow> {
    find_query(id).fetch_one_as::<UserRow>(db).await
}

pub async fn search(db: &PgPool, request: SearchRequest) -> rqb::Result<Vec<UserRow>> {
    let mut request = request;
    let has_client_sort = !request.sort.is_empty();
    request.limit = Some(
        request
            .limit
            .unwrap_or(DEFAULT_SEARCH_LIMIT)
            .clamp(1, MAX_SEARCH_LIMIT),
    );

    let query = select(users::table())
        .filter(users::ACTIVE.eq(true))
        .apply_search(request)?;
    let query = if has_client_sort {
        query
    } else {
        query.order_asc(users::EMAIL).order_asc(users::ID)
    };

    query.fetch_all_as::<UserRow>(db).await
}

pub async fn create(db: &PgPool, input: CreateUser) -> rqb::Result<UserRow> {
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

pub async fn patch(db: &PgPool, id: Uuid, input: PatchUser) -> rqb::Result<UserRow> {
    // `Changeset` is the REST PATCH bridge: only `Some(...)` fields become
    // assignments, so omitted JSON fields do not accidentally overwrite data.
    update(users::table())
        .filter(users::ID.eq(id))
        .patch(&input)
        .returning_all()
        .fetch_one_as::<UserRow>(db)
        .await
}

pub async fn deactivate(pool: &PgPool, id: Uuid) -> rqb::Result<()> {
    // `tx!` gives the closure a transaction connection; transaction-reused
    // helpers accept it through `PgExecutor<'_>`.
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
