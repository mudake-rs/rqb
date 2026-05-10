use chrono::{DateTime, Utc};
use rqb::dsl::{case, count_all, json_get_text, param, row_number, sum, true_, window};
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
        .column(users::ID)
        .column(users::EMAIL)
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
        .agg(sum(o.total_cents()).alias("gross_cents"))
        .agg(
            count_all()
                .filter(o.status().eq("paid"))
                .alias("paid_count"),
        )
        .agg(
            // Metadata-backed fields become JSON keys automatically; computed
            // values keep an explicit key.
            jsonb_agg_object![
                o.id(),
                o.status(),
                ("source", json_get_text(o.metadata(), "source")),
            ]
            .order_desc(o.created_at())
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
        .filter(o.metadata().key_exists("source"))
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

    println!("{}", built.sql);
    Ok(())
}
