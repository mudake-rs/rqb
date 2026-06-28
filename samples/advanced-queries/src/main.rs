use chrono::{DateTime, Utc};
use rqb::dsl::{
    case, count_all, current_user, json_get_text, json_typeof, param, row_number, sum, to_char,
    true_, width_bucket, window,
};
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::events;
use rqb_sample_schema::orders;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let au = users::alias("au");
    let o = orders::alias("o");
    let last_event_at = rqb::field!("last_event_at": timestamptz => DateTime<Utc>, ordered);

    // `try_into_cte` infers exposed fields from explicit field projections, so
    // common CTEs do not repeat explicit field metadata.
    let active_users = select(users::table())
        .columns((users::ID, users::EMAIL))
        .filter(users::ACTIVE.eq(true))
        .try_into_cte("active_users")?
        .not_materialized();
    let active_users_source = active_users.source().alias("au");

    let latest_event = subquery(
        select(events::table())
            .column(events::CREATED_AT)
            .filter(events::ORDER_ID.eq_field(o.id()))
            .order_desc(events::CREATED_AT)
            .limit(1),
        "latest_event",
        last_event_at,
    );

    // CASE is a value expression, so it can be selected, aliased, grouped, or
    // nested like any other expression.
    let order_size = case()
        .when(o.total_cents().gte(10_000), "large")
        .else_("standard");

    let built = select(active_users_source)
        .with(active_users)
        .join(&o, au.id().eq_field(o.user_id()))
        .left_join_lateral(latest_event, true_())
        .column(au.email())
        .item(sum(o.total_cents()).alias("gross_cents"))
        .item(
            count_all()
                .aggregate_filter(o.status().eq("paid"))
                .alias("paid_count"),
        )
        .item(
            // Metadata-backed fields become JSON keys automatically; computed
            // values keep an explicit key.
            jsonb_agg_object![
                o.id(),
                o.status(),
                ("source", json_get_text(o.metadata(), "source")),
            ]
            .aggregate_order_desc(o.created_at())
            .alias("orders"),
        )
        .item(
            row_number()
                .over(
                    window()
                        .partition_by(o.user_id())
                        .order_desc(o.created_at()),
                )
                .alias("order_rank"),
        )
        .item(order_size.alias("order_size"))
        .item(last_event_at.at("latest_event").alias("last_event_at"))
        .filter(o.status().in_list(["paid", "refunded"]))
        .filter(o.metadata().has_key("source"))
        .group_by(au.email())
        .group_by(o.id())
        .group_by(o.user_id())
        .group_by(o.status())
        .group_by(o.total_cents())
        .group_by(o.created_at())
        .group_by(last_event_at.at("latest_event"))
        .having(sum(o.total_cents()).gt(0_i64))
        .order_desc_nulls_last(last_event_at.at("latest_event"))
        .fetch_first_with_ties(param(25_i64))
        .build()?;

    assert_eq!(built.params.len(), 15);
    assert!(built.sql.starts_with("WITH \"active_users\""));
    assert!(built.sql.contains("LEFT JOIN LATERAL"));
    assert!(built.sql.contains("jsonb_build_object"));
    assert!(built.sql.contains("row_number() OVER"));
    assert!(built.sql.contains("CASE WHEN"));
    assert!(built.sql.ends_with("FETCH FIRST $15 ROWS WITH TIES"));

    // A compact helper showcase for common reporting expressions that would
    // otherwise become small raw SQL fragments.
    let helper_showcase = select(orders::table())
        .item(to_char(orders::CREATED_AT, "YYYY-MM").alias("order_month"))
        .item(
            sum(orders::TOTAL_CENTS)
                .over(window().partition_by(orders::USER_ID))
                .alias("user_total_cents"),
        )
        .item(width_bucket(orders::TOTAL_CENTS, 0_i64, 100_000_i64, 10_i32).alias("amount_bucket"))
        .item(json_typeof(orders::METADATA).alias("metadata_kind"))
        .item(current_user().alias("current_user"))
        .limit(1)
        .build()?;

    assert_eq!(
        helper_showcase.sql,
        "SELECT to_char(\"created_at\", $1) AS \"order_month\", sum(\"total_cents\") OVER (PARTITION BY \"user_id\") AS \"user_total_cents\", width_bucket(\"total_cents\", $2, $3, $4) AS \"amount_bucket\", json_typeof(\"metadata\") AS \"metadata_kind\", CURRENT_USER AS \"current_user\" FROM \"sample\".\"orders\" LIMIT $5"
    );
    assert_eq!(helper_showcase.params.len(), 5);

    println!("{}", built.sql);
    println!("{}", helper_showcase.sql);
    Ok(())
}
