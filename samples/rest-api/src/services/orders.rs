use async_stream::try_stream;
use futures_util::{Stream, TryStreamExt};
use rqb::dsl::{count_all, max, row};
use rqb::prelude::*;
use rqb_sample_schema::{app_users as user_fields, events, order_search_view, orders};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::users;
use crate::types::{
    CheckoutResponse, CreateOrder, CursorPage, OrderCursor, OrderExportRow, OrderRow,
    OrderSearchRow, Page, TransitionOrder, UserOrderSummaryRow,
};

const DEFAULT_PAGE_LIMIT: u32 = 100;

// Stream items carry boxed rqb errors so each yielded item stays pointer-sized
// instead of embedding the full structured error enum in the stream state.
type StreamResult<T> = std::result::Result<T, Box<rqb::Error>>;

pub async fn checkout(pool: &PgPool, input: CreateOrder) -> rqb::Result<CheckoutResponse> {
    tx!(pool, |conn| {
        let user = users::find(&mut *conn, input.user_id).await?;
        let order = create(&mut *conn, input).await?;
        Ok(CheckoutResponse { user, order })
    })
    .await
}

async fn create<'e>(db: impl PgExecutor<'e>, input: CreateOrder) -> rqb::Result<OrderRow> {
    insert(orders::table())
        .set_many((orders::ID.set(Uuid::new_v4()), orders::STATUS.set("open")))
        .values(&input)
        .returning_all()
        .fetch_one_as::<OrderRow>(db)
        .await
}

pub async fn list_after<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
    cursor: Option<OrderCursor>,
    limit: u32,
) -> rqb::Result<CursorPage<OrderRow>> {
    let limit = limit.clamp(1, 100);

    // Cursor pagination uses the same ORDER BY columns as the seek predicate.
    // Postgres can compare row values directly, so the usual "created_at OR
    // created_at = ... AND id ..." expansion stays out of application code.
    //
    // Fetching one extra row lets the API tell the client whether another page
    // exists without running a separate count query.
    let mut items = select(orders::table())
        .filter(orders::USER_ID.eq(user_id))
        .filter_option(cursor, |cursor| {
            row((orders::CREATED_AT, orders::ID)).lt((cursor.created_at, cursor.id))
        })
        .order_desc(orders::CREATED_AT)
        .order_desc(orders::ID)
        .limit(limit + 1)
        .fetch_all_as::<OrderRow>(db)
        .await?;

    let next_cursor = if items.len() > limit as usize {
        items.pop();
        items.last().map(|row| OrderCursor {
            created_at: row.created_at,
            id: row.id,
        })
    } else {
        None
    };

    Ok(CursorPage {
        items,
        next_cursor,
        limit,
    })
}

pub async fn cancel_open_for_user<'e>(
    db: impl PgExecutor<'e>,
    user_id: Uuid,
) -> rqb::Result<()> {
    update(orders::table())
        .set(orders::STATUS.set("canceled"))
        .filter(orders::USER_ID.eq(user_id))
        .filter(orders::STATUS.eq("open"))
        .execute(db)
        .await?;
    Ok(())
}

pub async fn transition(
    pool: &PgPool,
    order_id: Uuid,
    input: TransitionOrder,
) -> rqb::Result<Option<OrderRow>> {
    tx!(pool, |conn| {
        // Lock before checking the state machine. The later update and audit
        // insert run in the same transaction, so concurrent transitions cannot
        // observe a stale status.
        let current = select(orders::table())
            .filter(orders::ID.eq(order_id))
            .for_update()
            .fetch_one_as::<OrderRow>(&mut *conn)
            .await?;

        if !is_valid_transition(&current.status, &input.status) {
            return Ok(None);
        }

        let updated = update(orders::table())
            .set(orders::STATUS.set(input.status.clone()))
            .filter(orders::ID.eq(order_id))
            .returning_all()
            .fetch_one_as::<OrderRow>(&mut *conn)
            .await?;

        insert(events::table())
            .set_many((
                events::ID.set(Uuid::new_v4()),
                events::ORDER_ID.set(order_id),
                events::EVENT_TYPE.set(input.status.clone()),
                events::PAYLOAD.set(json!({
                    "actor_id": input.actor_id,
                    "old_status": current.status,
                    "new_status": updated.status,
                })),
            ))
            .execute(&mut *conn)
            .await?;

        Ok(Some(updated))
    })
    .await
}

pub async fn search(db: &PgPool, request: SearchRequest) -> rqb::Result<Page<OrderSearchRow>> {
    let limit = request.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    let offset = request.offset.unwrap_or(0);
    let has_limit = request.limit.is_some();

    // Pagination is application policy, not a hidden rqb executor behavior. The
    // same trusted query shape is used for the page items and the count.
    let mut query = select(order_search_view::view())
        .filter(order_search_view::STATUS.ne("canceled"))
        .request(request)?;

    if !has_limit {
        query = query.limit(limit);
    }

    let total = query.count(db).await?;
    let items = query.fetch_all_as::<OrderSearchRow>(db).await?;

    Ok(Page {
        items,
        total,
        limit,
        offset,
    })
}

pub fn export_csv_stream(
    pool: PgPool,
    user_id: Uuid,
) -> StreamResult<impl Stream<Item = StreamResult<String>> + Send + 'static> {
    let built = select(orders::table())
        .columns((
            orders::ID,
            orders::USER_ID,
            orders::STATUS,
            orders::TOTAL_CENTS,
            orders::CREATED_AT,
        ))
        .filter(orders::USER_ID.eq(user_id))
        .order_asc(orders::CREATED_AT)
        .build()
        .map_err(Box::new)?;

    Ok(try_stream! {
        yield String::from("id,user_id,status,total_cents,created_at\n");

        // The stream owns the pool clone and the built query, so axum can keep
        // pulling chunks after the handler has returned the response headers.
        let mut rows = built
            .fetch_stream_as::<OrderExportRow>(&pool)
            .map_err(Box::new)?;
        while let Some(row) = rows.try_next().await.map_err(Box::new)? {
            yield format!(
                "{},{},{},{},{}\n",
                row.id, row.user_id, row.status, row.total_cents, row.created_at
            );
        }
    })
}

pub async fn summary<'e>(db: impl PgExecutor<'e>) -> rqb::Result<Vec<UserOrderSummaryRow>> {
    let u = user_fields::alias("u");
    let o = orders::alias("o");
    let e = events::alias("e");
    let orders_json = jsonb_agg_object![o.id(), o.status(), o.total_cents()]
        .filter(o.id().is_not_null())
        .alias("orders");

    // The CTE exposes exactly the projected fields. `source().alias("u")`
    // then gives the outer query a normal relation source.
    let active_users = select(user_fields::table())
        .column(user_fields::ID)
        .column(user_fields::EMAIL)
        .filter(user_fields::ACTIVE.eq(true))
        .try_into_cte("active_users")?;

    select(active_users.source().alias("u"))
        .with(active_users)
        .left_join(&o, u.id().eq_field(o.user_id()))
        .left_join(&e, e.order_id().eq_field(o.id()))
        .column(u.email().alias("email"))
        .agg(
            count_all()
                .filter(o.id().is_not_null())
                .alias("order_count"),
        )
        .agg(orders_json)
        .agg(max(e.created_at()).alias("last_event_at"))
        .group_by(u.email())
        .fetch_all_as::<UserOrderSummaryRow>(db)
        .await
}

fn is_valid_transition(current: &str, next: &str) -> bool {
    matches!(
        (current, next),
        ("open", "paid") | ("open", "canceled") | ("paid", "refunded")
    )
}
