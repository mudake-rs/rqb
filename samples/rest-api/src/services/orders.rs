use rqb::dsl::{count_all, max};
use rqb::prelude::*;
use rqb_sample_schema::{app_users as user_fields, events, order_search_view, orders};
use uuid::Uuid;

use crate::services::users;
use crate::types::{
    CheckoutResponse, CreateOrder, OrderRow, OrderSearchRow, Page, UserOrderSummaryRow,
};

const DEFAULT_PAGE_LIMIT: u32 = 100;

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
        .set(orders::ID.set(Uuid::new_v4()))
        .values(&input)
        .set(orders::STATUS.set("open"))
        .returning_all()
        .fetch_one_as::<OrderRow>(db)
        .await
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
