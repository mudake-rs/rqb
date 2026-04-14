//! Render grouped stats and nested JSON aggregates.
//!
//! The first query shows `GROUP BY` inference for selected non-aggregate
//! fields. The second query shows a left join with nested `json_agg`.

use rqb::prelude::*;

const USER_ID: Field = Field::new("id", FieldType::Uuid);
const USER_EMAIL: Field = Field::new("email", FieldType::Text);
const ORDER_ID: Field = Field::new("id", FieldType::Uuid);
const ORDER_USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
const ORDER_STATUS: Field = Field::new("status", FieldType::Text);
const ORDER_TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);

fn users() -> Dataset {
    Dataset::table("app_users").fields([USER_ID, USER_EMAIL])
}

fn orders() -> Dataset {
    Dataset::view("order_search_view").fields([
        ORDER_ID,
        ORDER_USER_ID,
        ORDER_STATUS,
        ORDER_TOTAL_CENTS,
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stats = select(orders())
        .fields([ORDER_STATUS])
        .agg(count("orders"))
        .agg(count("paidOrders").filter(ORDER_STATUS.eq("paid")))
        .agg(sum(ORDER_TOTAL_CENTS, "totalCents"))
        .order_by(ORDER_STATUS.asc())
        .build_pg()?;

    println!("-- grouped stats");
    println!("{}", stats.debug_sql());

    let users = users().alias("u");
    let orders = orders().alias("o");
    let nested = select(users)
        .left_join(orders, USER_ID.on("u").eq_col(ORDER_USER_ID.on("o")))
        .fields([USER_ID.on("u"), USER_EMAIL.on("u")])
        .agg(
            json_agg(
                "orders",
                [
                    ORDER_ID.on("o").alias("id"),
                    ORDER_STATUS.on("o").alias("status"),
                ],
            )
            .filter(ORDER_ID.on("o").is_not_null()),
        )
        .filter(USER_EMAIL.on("u").contains("@example.com"))
        .build_pg()?;

    println!("-- users with nested orders");
    println!("{}", nested.debug_sql());
    Ok(())
}
