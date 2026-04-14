use rqb::prelude::*;
use rqb_sample_base::{
    OrderStatus,
    schema::{app_users, orders, withdrawals},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let paid = select(orders::dataset())
        .filter(orders::STATUS.eq(OrderStatus::Paid))
        .fetch_all_as::<serde_json::Value>(&db)
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
        .fetch_all_as::<serde_json::Value>(&db)
        .await?;
    println!(
        "joined users: {}",
        serde_json::to_string_pretty(&joined).unwrap()
    );

    let exact = select(withdrawals::dataset())
        .fields([withdrawals::ID, withdrawals::AMOUNT])
        .filter(withdrawals::AMOUNT.gt("9007199254740993"))
        .fetch_all_as::<serde_json::Value>(&db)
        .await?;
    println!(
        "exact domain rows: {}",
        serde_json::to_string_pretty(&exact).unwrap()
    );

    Ok(())
}
