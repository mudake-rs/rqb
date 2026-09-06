use rqb::dsl::{count, scalar_subquery, sum};
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::order_items as items;
use rqb_sample_schema::orders;

fn user_totals() -> Select {
    // Generated alias handles keep the table alias in one place. `u.email()`
    // returns a qualified FieldRef, so join-heavy queries do not repeat
    // `users::EMAIL.at("u")` everywhere.
    let u = users::alias("u");
    let o = orders::alias("o");
    let i = items::alias("i");

    // Aggregate items within each order before summing at user grain. Joining
    // items here would multiply both order amounts and JSON order objects.
    let units_per_order = select(&i)
        .expr(sum(i.quantity()))
        .filter(i.order_id().eq_field(o.id()));
    select(&u)
        .join(&o, u.id().eq_field(o.user_id()))
        .column(u.email())
        .expr_as(count(o.id()), "order_count")
        .expr_as(sum(scalar_subquery(units_per_order)), "units_sold")
        .expr_as(sum(o.total_cents()), "gross_cents")
        .expr_as(
            // Field arguments use their metadata names as JSON object keys.
            // Computed values can be passed as ("key", expr) pairs.
            jsonb_agg_object![o.id(), o.status()]
                .aggregate_order_desc(o.created_at())
                .aggregate_filter(o.status().eq("paid")),
            "paid_orders",
        )
        .group_by(u.email())
        .order_asc(u.email())
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let grouped = user_totals().build()?;

    assert_eq!(
        grouped.sql,
        "SELECT \"u\".\"email\" AS \"u_email\", count(\"o\".\"id\") AS \"order_count\", sum((SELECT sum(\"i\".\"quantity\") FROM \"sample\".\"order_items\" AS \"i\" WHERE \"i\".\"order_id\" = \"o\".\"id\")) AS \"units_sold\", sum(\"o\".\"total_cents\") AS \"gross_cents\", jsonb_agg(jsonb_build_object($1, \"o\".\"id\", $2, \"o\".\"status\") ORDER BY \"o\".\"created_at\" DESC) FILTER (WHERE \"o\".\"status\" = $3) AS \"paid_orders\" FROM \"sample\".\"app_users\" AS \"u\" JOIN \"sample\".\"orders\" AS \"o\" ON \"u\".\"id\" = \"o\".\"user_id\" GROUP BY \"u\".\"email\" ORDER BY \"u\".\"email\" ASC"
    );
    assert_eq!(grouped.params.len(), 3);

    let latest_per_user = select(orders::table())
        .distinct_on(orders::USER_ID)
        .columns((orders::USER_ID, orders::ID, orders::STATUS))
        .order_asc(orders::USER_ID)
        .order_desc(orders::CREATED_AT)
        .build()?;

    assert_eq!(
        latest_per_user.sql,
        "SELECT DISTINCT ON (\"user_id\") \"user_id\", \"id\", \"status\" FROM \"sample\".\"orders\" ORDER BY \"user_id\" ASC, \"created_at\" DESC"
    );

    println!("{}", grouped.sql);
    println!("{}", latest_per_user.sql);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    #[ignore = "requires sample schema and RQB_TEST_DATABASE_URL"]
    async fn item_fanout_does_not_duplicate_order_totals_or_json() {
        #[derive(sqlx::FromRow)]
        struct Totals {
            order_count: i64,
            units_sold: Option<sqlx::types::BigDecimal>,
            gross_cents: sqlx::types::BigDecimal,
            paid_orders: Option<serde_json::Value>,
        }
        let pool = sqlx::PgPool::connect(&std::env::var("RQB_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        let user_id = Uuid::new_v4();
        insert(users::table())
            .set_many((
                users::ID.set(user_id),
                users::EMAIL.set(format!("{user_id}@test")),
                users::STATUS.set("active"),
                users::DISPLAY_NAME.set("Audit"),
            ))
            .execute(&mut *tx)
            .await
            .unwrap();
        assert!(
            user_totals()
                .filter(users::ID.at("u").eq(user_id))
                .fetch_optional_as::<Totals>(&mut *tx)
                .await
                .unwrap()
                .is_none()
        );
        use rqb_sample_schema::products as p;
        for (item_count, status, total) in [(0, "open", 50_i64), (2, "paid", 100), (1, "paid", 100)]
        {
            let order_id = Uuid::new_v4();
            insert(orders::table())
                .set_many((
                    orders::ID.set(order_id),
                    orders::USER_ID.set(user_id),
                    orders::STATUS.set(status),
                    orders::TOTAL_CENTS.set(total),
                ))
                .execute(&mut *tx)
                .await
                .unwrap();
            for _ in 0..item_count {
                let product_id = Uuid::new_v4();
                insert(p::table())
                    .set_many((
                        p::ID.set(product_id),
                        p::SKU.set(product_id.to_string()),
                        p::NAME.set("Audit"),
                        p::PRICE_CENTS.set(50_i64),
                    ))
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                insert(items::table())
                    .set_many((
                        items::ID.set(Uuid::new_v4()),
                        items::ORDER_ID.set(order_id),
                        items::PRODUCT_ID.set(product_id),
                        items::QUANTITY.set(1_i32),
                        items::UNIT_PRICE_CENTS.set(50_i64),
                    ))
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
            if item_count == 0 {
                let totals = user_totals()
                    .filter(users::ID.at("u").eq(user_id))
                    .fetch_one_as::<Totals>(&mut *tx)
                    .await
                    .unwrap();
                assert_eq!(totals.order_count, 1);
                assert_eq!(totals.units_sold, None);
                assert_eq!(totals.gross_cents, sqlx::types::BigDecimal::from(50));
                assert_eq!(totals.paid_orders, None);
            }
        }
        let totals = user_totals()
            .filter(users::ID.at("u").eq(user_id))
            .fetch_one_as::<Totals>(&mut *tx)
            .await
            .unwrap();
        assert_eq!(totals.order_count, 3);
        assert_eq!(totals.units_sold, Some(sqlx::types::BigDecimal::from(3)));
        assert_eq!(totals.gross_cents, sqlx::types::BigDecimal::from(250));
        assert_eq!(totals.paid_orders.unwrap().as_array().unwrap().len(), 2);
        tx.rollback().await.unwrap();
    }
}
