use chrono::{DateTime, Utc};
use rqb::prelude::*;
use rqb_sample_base::{
    OrderStatus,
    schema::{app_users, order_search_view as order_search, orders},
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserWithOrders {
    id: Uuid,
    email: String,
    orders: Vec<OrderSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderSummary {
    id: Uuid,
    status: OrderStatus,
    channel: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderStats {
    status: OrderStatus,
    orders: i64,
    total_cents: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;
    let user_id = rqb_sample_base::ADA_USER_ID;

    let user = app_users::table().alias("u");
    let order = orders::table().alias("o");

    let user = select(&user)
        .left_join(&order, user.id().eq_col(order.user_id()))
        .fields([user.id().alias("id"), user.email().alias("email")])
        .agg(
            json_agg(
                "orders",
                [
                    order.id().alias("id"),
                    order.status().alias("status"),
                    order.channel().alias("channel"),
                    order.created_at().alias("createdAt"),
                ],
            )
            .filter(order.id().is_not_null()),
        )
        .filter(user.id().eq(user_id))
        .fetch_one_as::<UserWithOrders>(&db)
        .await?;
    println!(
        "{} {} has {} orders",
        user.id,
        user.email,
        user.orders.len()
    );
    for order in &user.orders {
        println!(
            "  order {} status={:?} channel={} created_at={}",
            order.id, order.status, order.channel, order.created_at
        );
    }

    let stats = select(order_search::dataset())
        .fields([order_search::STATUS])
        .agg(count("orders"))
        .agg(sum(order_search::TOTAL_CENTS, "totalCents"))
        .order_by(order_search::STATUS.asc())
        .fetch_as::<OrderStats>(&db)
        .await?;
    for row in &stats {
        println!(
            "status={:?} orders={} total_cents={}",
            row.status, row.orders, row.total_cents
        );
    }

    Ok(())
}
