use async_stream::try_stream;
use futures_util::{Stream, TryStreamExt};
use rqb::dsl::{coalesce, count_all, literal, max, row};
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
const MAX_PAGE_LIMIT: u32 = 100;

type StreamResult<T> = std::result::Result<T, rqb::Error>;

pub async fn checkout(pool: &PgPool, input: CreateOrder) -> rqb::Result<CheckoutResponse> {
    tx!(pool, |conn| {
        let user = users::find_query(input.user_id)
            .fetch_one_as(&mut *conn)
            .await?;
        let order = create(&mut *conn, input).await?;
        Ok(CheckoutResponse { user, order })
    })
    .await
}

async fn create(db: impl PgExecutor<'_>, input: CreateOrder) -> rqb::Result<OrderRow> {
    insert(orders::table())
        .set_many((orders::ID.set(Uuid::new_v4()), orders::STATUS.set("open")))
        .values(&input)
        .returning_all()
        .fetch_one_as::<OrderRow>(db)
        .await
}

pub async fn list_after(
    db: &PgPool,
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

pub async fn filter(
    db: &PgPool,
    user_id: Uuid,
    status: Option<String>,
    min_total: Option<i64>,
    from_date: Option<chrono::DateTime<chrono::Utc>>,
    limit: u32,
) -> rqb::Result<Vec<OrderRow>> {
    select(orders::table())
        .filter(orders::USER_ID.eq(user_id))
        .filter_option(status, |status| orders::STATUS.eq(status))
        .filter_option(min_total, |total| orders::TOTAL_CENTS.gte(total))
        .filter_option(from_date, |from| orders::CREATED_AT.gte(from))
        .order_desc(orders::CREATED_AT)
        .limit(limit.clamp(1, 100))
        .fetch_all_as::<OrderRow>(db)
        .await
}

pub async fn cancel_open_for_user(db: impl PgExecutor<'_>, user_id: Uuid) -> rqb::Result<()> {
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
    let mut request = request;
    let limit = request
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let offset = request.offset.unwrap_or(0);
    let has_client_sort = !request.sort.is_empty();
    request.limit = Some(limit);

    // Pagination is application policy, not a hidden rqb executor behavior. The
    // same trusted query shape is used for the page items and the count.
    let query = select(order_search_view::view())
        .filter(order_search_view::STATUS.ne("canceled"))
        .apply_search(request)?;
    let query = if has_client_sort {
        query
    } else {
        query.order_desc(order_search_view::CREATED_AT)
    };

    let query = query.order_desc(order_search_view::ID);
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
    let mut rows = select(orders::table())
        // The CSV contract is intentionally narrower than the full order row.
        .columns((
            orders::ID,
            orders::USER_ID,
            orders::STATUS,
            orders::TOTAL_CENTS,
            orders::CREATED_AT,
        ))
        .filter(orders::USER_ID.eq(user_id))
        .order_asc(orders::CREATED_AT)
        .fetch_stream_pool_as::<OrderExportRow>(pool)?;

    Ok(try_stream! {
        yield String::from("id,user_id,status,total_cents,created_at\n");

        // The rqb stream owns the pool handle and built query, so axum can keep
        // pulling chunks after the handler returns response headers. This sample
        // yields one CSV line per row; production exports may batch lines into
        // larger chunks to reduce response overhead.
        while let Some(row) = rows.try_next().await? {
            yield format!(
                "{},{},{},{},{}\n",
                row.id, row.user_id, row.status, row.total_cents, row.created_at
            );
        }
    })
}

pub async fn summary(db: &PgPool) -> rqb::Result<Vec<UserOrderSummaryRow>> {
    const U: &str = "u";

    let u = user_fields::alias(U);
    let o = orders::alias("o");
    let last_event_at =
        rqb::field!("last_event_at": timestamptz => chrono::DateTime<chrono::Utc>, ordered);
    let orders_json = coalesce([
        jsonb_agg_object![o.id(), o.status(), o.total_cents()]
            .aggregate_order_desc(o.created_at())
            .aggregate_filter(o.id().is_not_null()),
        literal("[]").cast("jsonb"),
    ]);

    // The CTE exposes app_users fields, so the generated alias handle can
    // qualify those same field metas against the CTE alias.
    let active_users = select(user_fields::table())
        .column(user_fields::ID)
        .column(user_fields::EMAIL)
        .filter(user_fields::ACTIVE.eq(true))
        .infer_cte("active_users")?;
    let order_events = select(events::table())
        .column(events::ORDER_ID)
        .expr_as(max(events::CREATED_AT), "last_event_at")
        .filter(events::ORDER_ID.is_not_null())
        .group_by(events::ORDER_ID)
        .into_cte("order_events", (events::ORDER_ID, last_event_at));
    let order_events_source = order_events.source().alias("oe");

    select(active_users.source().alias(U))
        .with(active_users)
        .with(order_events)
        .left_join(&o, u.id().eq_field(o.user_id()))
        .left_join(
            order_events_source,
            events::ORDER_ID.at("oe").eq_field(o.id()),
        )
        .expr_as(u.email(), "email")
        .expr_as(
            count_all().aggregate_filter(o.id().is_not_null()),
            "order_count",
        )
        .expr_as(orders_json, "orders")
        .expr_as(max(last_event_at.at("oe")), "last_event_at")
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
