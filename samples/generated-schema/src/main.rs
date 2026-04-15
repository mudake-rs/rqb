use rqb::prelude::*;
use rqb_sample_base::{
    OrderStatus,
    schema::{app_users, orders, withdrawals},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
struct PaidOrder {
    id: Uuid,
    status: OrderStatus,
}

#[derive(Debug, Deserialize, Serialize)]
struct JoinedUser {
    id: Uuid,
    email: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WithdrawalAmount {
    id: Uuid,
    amount: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let paid = select(orders::dataset())
        .fields([orders::ID, orders::STATUS])
        .filter(orders::STATUS.eq(OrderStatus::Paid))
        .fetch_all_as::<PaidOrder>(&db)
        .await?;
    println!(
        "paid orders: {}",
        serde_json::to_string_pretty(&paid).unwrap()
    );

    let user = app_users::table().alias("u");
    let order = orders::table().alias("o");
    let joined = select(&user)
        .left_join(&order, user.id().eq_col(order.user_id()))
        .fields([user.id().alias("id"), user.email().alias("email")])
        .filter(order.status().eq(OrderStatus::Paid))
        .fetch_all_as::<JoinedUser>(&db)
        .await?;
    println!(
        "joined users: {}",
        serde_json::to_string_pretty(&joined).unwrap()
    );

    let exact = select(withdrawals::dataset())
        .fields([withdrawals::ID, withdrawals::AMOUNT])
        .filter(withdrawals::AMOUNT.gt("9007199254740993"))
        .fetch_all_as::<WithdrawalAmount>(&db)
        .await?;
    println!(
        "exact domain rows: {}",
        serde_json::to_string_pretty(&exact).unwrap()
    );

    Ok(())
}
