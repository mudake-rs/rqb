use chrono::{DateTime, Utc};
use rqb::prelude::*;
use rqb_sample_base::schema::{app_users, order_search_view as order_search, orders};
use serde::Deserialize;
use uuid::Uuid;

fn recent_orders() -> Dataset {
    Dataset::cte("recent_orders").fields([
        order_search::ID,
        order_search::STATUS,
        order_search::TOTAL_CENTS,
        order_search::CREATED_AT,
    ])
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderRow {
    id: Uuid,
    status: rqb_sample_base::OrderStatus,
    total_cents: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UserRow {
    id: Uuid,
    email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestStatusRow {
    email: String,
    latest_status: rqb_sample_base::OrderStatus,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let recent = cte(
        "recent_orders",
        select(order_search::dataset())
            .fields([
                order_search::ID,
                order_search::STATUS,
                order_search::TOTAL_CENTS,
                order_search::CREATED_AT,
            ])
            .filter(order_search::CREATED_AT.gte("2026-02-01T00:00:00Z"))
            .build(),
    );

    let paid_recent = select(recent_orders())
        .cte(recent)
        .filter(order_search::STATUS.eq(rqb_sample_base::OrderStatus::Paid))
        .order_by(order_search::CREATED_AT.desc())
        .fetch_all_as::<OrderRow>(&db)
        .await?;
    print_orders("paid recent orders", &paid_recent);

    let user = app_users::table().alias("u");
    let order = orders::table().alias("o");
    let users_with_orders = select(&user)
        .fields([user.id().alias("id"), user.email().alias("email")])
        .filter(exists(
            select(&order)
                .filter(order.user_id().eq_col(user.id()))
                .build(),
        ))
        .fetch_all_as::<UserRow>(&db)
        .await?;
    print_users("users with any order", &users_with_orders);

    let paid_order = orders::table().alias("paid_o");
    let users_with_paid_orders = select(app_users::dataset())
        .fields([app_users::ID, app_users::EMAIL])
        .filter(
            app_users::ID.in_subquery(
                select(&paid_order)
                    .fields([paid_order.user_id()])
                    .filter(paid_order.status().eq(rqb_sample_base::OrderStatus::Paid))
                    .build(),
            ),
        )
        .fetch_all_as::<UserRow>(&db)
        .await?;
    print_users("users with paid orders", &users_with_paid_orders);

    let active_users = select(app_users::dataset())
        .fields([app_users::ID, app_users::EMAIL])
        .filter(app_users::STATUS.eq(rqb_sample_base::UserStatus::Active));
    let disabled_users = select(app_users::dataset())
        .fields([app_users::ID, app_users::EMAIL])
        .filter(app_users::STATUS.eq(rqb_sample_base::UserStatus::Disabled));
    let users_from_set_query = union(active_users, disabled_users)
        .order_by(field("email").asc())
        .fetch_all_as::<UserRow>(&db)
        .await?;
    print_users("users from UNION set query", &users_from_set_query);

    let latest_order = select(&order)
        .fields([order.status()])
        .filter(order.user_id().eq_col(user.id()))
        .order_by(order.created_at().desc())
        .limit(1)
        .into_source("latest_order")
        .fields([orders::STATUS]);
    let latest_status = select(&user)
        .fields([
            user.email().alias("email"),
            orders::STATUS.on("latest_order").alias("latestStatus"),
        ])
        .left_join_lateral(latest_order, raw("TRUE"))
        .fetch_all_as::<LatestStatusRow>(&db)
        .await?;
    print_latest_statuses("latest order status per user", &latest_status);

    let raw_source = Dataset::raw(
        "SELECT id, email FROM app_users WHERE status = 'active'",
        "active_users",
    )
    .fields([app_users::ID, app_users::EMAIL]);
    let raw_rows = select(raw_source)
        .fetch_all_as::<UserRow>(&db)
        .await?;
    print_users("raw source rows", &raw_rows);

    Ok(())
}

fn print_orders(label: &str, orders: &[OrderRow]) {
    println!("{label}: {}", orders.len());
    for order in orders {
        println!(
            "  {} status={:?} total_cents={} created_at={}",
            order.id, order.status, order.total_cents, order.created_at
        );
    }
}

fn print_users(label: &str, users: &[UserRow]) {
    println!("{label}: {}", users.len());
    for user in users {
        println!("  {} {}", user.id, user.email);
    }
}

fn print_latest_statuses(label: &str, rows: &[LatestStatusRow]) {
    println!("{label}: {}", rows.len());
    for row in rows {
        println!("  {} latest_status={:?}", row.email, row.latest_status);
    }
}
