use rqb::dsl::{count_distinct, sum};
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::order_items as items;
use rqb_sample_schema::orders;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Generated alias handles keep the table alias in one place. `u.email()`
    // returns a qualified FieldRef, so join-heavy queries do not repeat
    // `users::EMAIL.at("u")` everywhere.
    let u = users::alias("u");
    let o = orders::alias("o");
    let i = items::alias("i");

    let grouped = select(&u)
        .left_join(&o, u.id().eq_field(o.user_id()))
        .left_join(&i, o.id().eq_field(i.order_id()))
        .column(u.email())
        .item(count_distinct(o.id()).alias("order_count"))
        .item(sum(i.quantity()).alias("units_sold"))
        .item(sum(o.total_cents()).alias("gross_cents"))
        .item(
            // Field arguments use their metadata names as JSON object keys.
            // Computed values can be passed as ("key", expr) pairs.
            jsonb_agg_object![o.id(), o.status()]
                .aggregate_order_desc(o.created_at())
                .aggregate_filter(o.status().eq("paid"))
                .alias("paid_orders"),
        )
        .group_by(u.email())
        .having(count_distinct(o.id()).gt(0_i64))
        .order_asc(u.email())
        .build()?;

    assert_eq!(
        grouped.sql,
        "SELECT \"u\".\"email\" AS \"u_email\", count(DISTINCT \"o\".\"id\") AS \"order_count\", sum(\"i\".\"quantity\") AS \"units_sold\", sum(\"o\".\"total_cents\") AS \"gross_cents\", jsonb_agg(jsonb_build_object($1, \"o\".\"id\", $2, \"o\".\"status\") ORDER BY \"o\".\"created_at\" DESC) FILTER (WHERE \"o\".\"status\" = $3) AS \"paid_orders\" FROM \"sample\".\"app_users\" AS \"u\" LEFT JOIN \"sample\".\"orders\" AS \"o\" ON \"u\".\"id\" = \"o\".\"user_id\" LEFT JOIN \"sample\".\"order_items\" AS \"i\" ON \"o\".\"id\" = \"i\".\"order_id\" GROUP BY \"u\".\"email\" HAVING count(DISTINCT \"o\".\"id\") > $4 ORDER BY \"u\".\"email\" ASC"
    );
    assert_eq!(grouped.params.len(), 4);

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
