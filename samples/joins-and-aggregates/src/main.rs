use chrono::{DateTime, Utc};
use rqb::prelude::*;
use rqb_sample_base::{
    OrderStatus,
    schema::{app_users, order_search_view as order_search, orders},
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct UserWithOrders {
    id: Uuid,
    email: String,
    orders: Vec<OrderSummary>,
}

#[derive(Debug, Deserialize)]
struct OrderSummary {
    id: Uuid,
    status: OrderStatus,
    channel: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct OrderStats {
    status: OrderStatus,
    orders: i64,
    total_cents: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;
    let user_id = rqb_sample_base::ADA_USER_ID;

    // 1. Start with grouped aggregates over a generated search view.
    let stats = select(order_search::dataset())
        .fields([order_search::STATUS])
        .agg(count("orders"))
        .agg(sum(order_search::TOTAL_CENTS, "total_cents"))
        .order_by(order_search::STATUS.asc())
        .fetch_all_as::<OrderStats>(&db)
        .await?;
    for row in &stats {
        println!(
            "status={:?} orders={} total_cents={}",
            row.status, row.orders, row.total_cents
        );
    }

    // 2. Then join generated table aliases and aggregate child rows into nested JSON.
    let user_table = app_users::table().alias("u");
    let order_table = orders::table().alias("o");
    let user = select(&user_table)
        .left_join(&order_table, user_table.id().eq_col(order_table.user_id()))
        .fields([
            user_table.id().alias("id"),
            user_table.email().alias("email"),
        ])
        .agg(
            json_agg(
                "orders",
                [
                    order_table.id().alias("id"),
                    order_table.status().alias("status"),
                    order_table.channel().alias("channel"),
                    order_table.created_at().alias("created_at"),
                ],
            )
            .filter(order_table.id().is_not_null()),
        )
        .filter(user_table.id().eq(user_id))
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

    Ok(())
}
