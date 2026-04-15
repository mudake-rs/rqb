use chrono::{DateTime, Utc};
use rqb::prelude::*;
use rqb_sample_base::{
    ACME_ORG_ID, OrderStatus, UserStatus,
    schema::{app_users, events, order_search_view as order_search},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
struct AdvancedOrderRow {
    id: Uuid,
    email: String,
    user_status: UserStatus,
    status: OrderStatus,
    status_label: String,
    channel_label: String,
    email_lower: String,
    email_len: i32,
    search_label: String,
    score_text: String,
    total_text: String,
    created_day: DateTime<Utc>,
    email_rank: i64,
    previous_total: i64,
    latest_event_type: String,
    latest_event_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserSummaryRow {
    email: String,
    order_count: i64,
    paid_count: i64,
    total_cents: String,
    orders: Vec<OrderSummary>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OrderSummary {
    id: Uuid,
    status: OrderStatus,
    total_cents: i64,
}

fn eligible_orders() -> Dataset {
    Dataset::cte("eligible_orders").fields([
        order_search::ID,
        order_search::EMAIL,
        order_search::TOTAL_CENTS,
    ])
}

fn advanced_order_rows_query() -> SelectBuilder {
    let os = order_search::view().alias("os");
    let user = app_users::table().alias("u");
    let event = events::table().alias("e");
    let cte_event = events::table().alias("cte_e");

    let eligible = cte(
        "eligible_orders",
        select(order_search::dataset())
            .fields([
                order_search::ID.into(),
                order_search::EMAIL.into(),
                order_search::TOTAL_CENTS.alias("total_cents"),
            ])
            .filter(all([
                order_search::ORGANIZATION_ID.eq(ACME_ORG_ID),
                any([
                    order_search::STATUS.eq(OrderStatus::Paid),
                    order_search::TOTAL_CENTS.gte(10_000_i64),
                ]),
            ]))
            .build(),
    );

    let latest_event = select(&event)
        .fields([
            event.event_type().alias("event_type"),
            event.created_at().alias("created_at"),
        ])
        .filter(event.order_id().eq_col(os.id()))
        .order_by(event.created_at().desc())
        .limit(1)
        .into_source("latest_event")
        .fields([events::EVENT_TYPE, events::CREATED_AT]);

    /*
    SQL shape:

    WITH eligible_orders AS (
      SELECT id, email, total_cents
      FROM order_search_view
      WHERE organization_id = $1
        AND (status = $2 OR total_cents >= $3)
    )
    SELECT os fields, CASE status labels, function expressions, window values,
           and latest_event fields
    FROM order_search_view AS os
    LEFT JOIN app_users AS u ON os.email = u.email
    LEFT JOIN LATERAL (
      SELECT event_type, created_at
      FROM events AS e
      WHERE e.order_id = os.id
      ORDER BY e.created_at DESC
      LIMIT 1
    ) AS latest_event ON TRUE
    WHERE os.id IN (SELECT id FROM eligible_orders)
      AND EXISTS (SELECT id FROM events AS cte_e WHERE cte_e.order_id = os.id)
      AND JSON/tag/text/status predicates all pass
    ORDER BY os.created_at DESC NULLS LAST, os.email ASC
    LIMIT 20 OFFSET 0
    */
    select(&os)
        .cte(eligible)
        .left_join(&user, os.email().eq_col(user.email()))
        .left_join_lateral(latest_event, raw("TRUE"))
        .fields([
            os.id().alias("id"),
            os.email().alias("email"),
            user.status().alias("user_status"),
            os.status().alias("status"),
            events::EVENT_TYPE
                .on("latest_event")
                .alias("latest_event_type"),
            events::CREATED_AT
                .on("latest_event")
                .alias("latest_event_at"),
        ])
        .select_expr(
            case_when(os.status().eq(OrderStatus::Paid))
                .then("settled")
                .when(os.status().eq(OrderStatus::Draft))
                .then("open")
                .otherwise("other")
                .alias("status_label"),
        )
        .select_expr(
            coalesce([os.channel().expr(), "unknown".into_sql_expr()]).alias("channel_label"),
        )
        .select_expr(lower(os.email().expr()).alias("email_lower"))
        .select_expr(length(os.email().expr()).alias("email_len"))
        .select_expr(
            func(
                "concat_ws",
                [
                    " / ".into_sql_expr(),
                    os.email().expr(),
                    os.channel().expr(),
                ],
            )
            .returns(FieldType::Text)
            .alias("search_label"),
        )
        .select_expr(os.metadata().json_path_text(["score"]).alias("score_text"))
        .select_expr(cast(os.total_cents().expr(), FieldType::Text).alias("total_text"))
        .select_expr(date_trunc("day", os.created_at().expr()).alias("created_day"))
        .select_expr(
            row_number()
                .over(partition_by(os.email()).order_by(os.created_at().desc()))
                .alias("email_rank"),
        )
        .select_expr(
            lag(os.total_cents())
                .offset(1)
                .default(0_i64)
                .over(partition_by(os.email()).order_by(os.created_at().asc()))
                .alias("previous_total"),
        )
        .filter(os.id().in_subquery(
            select(eligible_orders())
                .fields([order_search::ID.on("eligible_orders")])
                .filter(order_search::TOTAL_CENTS.on("eligible_orders").gt(0_i64))
                .build(),
        ))
        .filter(exists(
            select(&cte_event)
                .fields([cte_event.id()])
                .filter(cte_event.order_id().eq_col(os.id()))
                .build(),
        ))
        .filter(all([
            os.organization_id().eq(ACME_ORG_ID),
            os.metadata().path("score").gte(40_i64),
            os.metadata().key_exists("campaign"),
            os.tags().contains_any(["vip", "standard"]),
            not(os.status().eq(OrderStatus::Refunded)),
        ]))
        .filter_option(Some("web"), |channel| os.channel().eq(channel))
        .order_by(Sort::desc(os.created_at()).nulls_last())
        .order_by(os.email().asc())
        .limit(20)
        .offset(0)
}

fn user_summary_query() -> SelectBuilder {
    /*
    SQL shape:

    SELECT email,
           count(*) FILTER (...),
           sum(total_cents),
           jsonb_agg(jsonb_build_object(... ) ORDER BY created_at DESC)
    FROM order_search_view
    WHERE organization_id = $1
    GROUP BY email
    HAVING count(*) > $2
    ORDER BY email ASC
    */
    select(order_search::dataset())
        .fields([order_search::EMAIL])
        .agg(count("order_count"))
        .agg(count("paid_count").filter(order_search::STATUS.eq(OrderStatus::Paid)))
        .agg(sum(order_search::TOTAL_CENTS, "total_cents"))
        .json_agg(
            "orders",
            [
                order_search::ID.into(),
                order_search::STATUS.into(),
                order_search::TOTAL_CENTS.alias("total_cents"),
            ],
        )
        .order_within("orders", order_search::CREATED_AT.desc())
        .filter_agg("orders", order_search::STATUS.ne(OrderStatus::Cancelled))
        .filter(order_search::ORGANIZATION_ID.eq(ACME_ORG_ID))
        .group_by([order_search::EMAIL])
        .having(raw("COUNT(*) > ?").bind(0_i64))
        .order_by(order_search::EMAIL.asc())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let rows = advanced_order_rows_query()
        .fetch_all_as::<AdvancedOrderRow>(&db)
        .await?;
    println!("advanced rows: {}", serde_json::to_string_pretty(&rows)?);

    let summary = user_summary_query()
        .fetch_all_as::<UserSummaryRow>(&db)
        .await?;
    println!("summary rows: {}", serde_json::to_string_pretty(&summary)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const EXPECTED_ADVANCED_ROWS_SQL: &str = "WITH \"eligible_orders\" AS (SELECT \"id\", \"email\", \"total_cents\" FROM \"public\".\"order_search_view\" WHERE (\"organization_id\" = $1::text::uuid AND (\"status\" = $2::text::\"public\".\"order_status\" OR \"total_cents\" >= $3::bigint))) SELECT \"os\".\"id\" AS \"id\", \"os\".\"email\" AS \"email\", \"u\".\"status\"::text AS \"user_status\", \"os\".\"status\"::text AS \"status\", \"latest_event\".\"event_type\" AS \"latest_event_type\", \"latest_event\".\"created_at\" AS \"latest_event_at\", CASE WHEN \"os\".\"status\" = $4::text::\"public\".\"order_status\" THEN $5 WHEN \"os\".\"status\" = $6::text::\"public\".\"order_status\" THEN $7 ELSE $8 END AS \"status_label\", COALESCE(\"os\".\"channel\", $9) AS \"channel_label\", lower(\"os\".\"email\") AS \"email_lower\", length(\"os\".\"email\") AS \"email_len\", \"concat_ws\"($10, \"os\".\"email\", \"os\".\"channel\") AS \"search_label\", (\"os\".\"metadata\" #>> ARRAY[$11]::text[]) AS \"score_text\", CAST(\"os\".\"total_cents\" AS text) AS \"total_text\", date_trunc($12, \"os\".\"created_at\") AS \"created_day\", row_number() OVER (PARTITION BY \"os\".\"email\" ORDER BY \"os\".\"created_at\" DESC) AS \"email_rank\", lag(\"os\".\"total_cents\", $13::int, $14::bigint) OVER (PARTITION BY \"os\".\"email\" ORDER BY \"os\".\"created_at\" ASC) AS \"previous_total\" FROM \"public\".\"order_search_view\" AS \"os\" LEFT JOIN \"public\".\"app_users\" AS \"u\" ON \"os\".\"email\" = \"u\".\"email\" LEFT JOIN LATERAL (SELECT \"e\".\"event_type\", \"e\".\"created_at\" FROM \"public\".\"events\" AS \"e\" WHERE \"e\".\"order_id\" = \"os\".\"id\" ORDER BY \"e\".\"created_at\" DESC LIMIT 1) AS \"latest_event\" ON TRUE WHERE (((\"os\".\"id\" IN (SELECT \"eligible_orders\".\"id\" FROM \"eligible_orders\" WHERE \"eligible_orders\".\"total_cents\" > $15::bigint) AND EXISTS (SELECT 1 FROM \"public\".\"events\" AS \"cte_e\" WHERE \"cte_e\".\"order_id\" = \"os\".\"id\")) AND (\"os\".\"organization_id\" = $16::text::uuid AND (\"os\".\"metadata\" #>> ARRAY[$17]::text[])::numeric >= $18::text::numeric AND \"os\".\"metadata\" ? $19 AND \"os\".\"tags\" && $20::text[] AND NOT (\"os\".\"status\" = $21::text::\"public\".\"order_status\"))) AND \"os\".\"channel\" = $22) ORDER BY \"os\".\"created_at\" DESC NULLS LAST, \"os\".\"email\" ASC LIMIT 20 OFFSET 0";
    const EXPECTED_USER_SUMMARY_SQL: &str = "SELECT \"email\", COUNT(*) AS \"order_count\", COUNT(*) FILTER (WHERE \"status\" = $1::text::\"public\".\"order_status\") AS \"paid_count\", SUM(\"total_cents\")::text AS \"total_cents\", COALESCE(jsonb_agg(jsonb_build_object('id', \"id\", 'status', \"status\"::text, 'total_cents', \"total_cents\") ORDER BY \"created_at\" DESC) FILTER (WHERE \"status\" <> $2::text::\"public\".\"order_status\"), '[]'::jsonb) AS \"orders\" FROM \"public\".\"order_search_view\" WHERE \"organization_id\" = $3::text::uuid GROUP BY \"email\" HAVING COUNT(*) > $4 ORDER BY \"email\" ASC LIMIT 100 OFFSET 0";

    #[test]
    fn advanced_order_rows_sql_matches_documented_shape() -> TestResult {
        let built = advanced_order_rows_query().build_pg()?;

        assert_eq!(built.rows.sql, EXPECTED_ADVANCED_ROWS_SQL);
        assert!(!built.rows.cacheable);
        assert_eq!(built.rows.params.len(), 22);
        Ok(())
    }

    #[test]
    fn user_summary_sql_matches_documented_shape() -> TestResult {
        let built = user_summary_query().build_pg()?;

        assert_eq!(built.rows.sql, EXPECTED_USER_SUMMARY_SQL);
        assert!(!built.rows.cacheable);
        assert_eq!(built.rows.params.len(), 4);
        Ok(())
    }
}
