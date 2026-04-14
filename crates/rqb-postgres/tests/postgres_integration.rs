use pretty_assertions::assert_eq;
use rqb_core::{
    Dataset, DbEnum, ElemType, EnumType, Field, FieldType, IntoSqlExpr, JsonPathPolicy,
    SearchRequest, SelectRepr, TypeFamily, TypeSpec, Value, ValueRepr, all, case_when, cast,
    coalesce, count, cte, delete, excluded, exists, field, insert, lower, not_exists, raw,
    raw_query, select, set_default, set_expr, sum, union, union_all, update, upper,
};
use rqb_postgres::{
    BuildPostgres, BuiltQuery, Error as PgError, ExecutePostgres, ExecuteRawPostgres,
    ExecuteWritePostgres, PgExecutor, ResultExt, StatementCache,
};
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Row, types::ToSql};

mod order_search {
    use super::*;

    pub const ORDER_STATUS: EnumType = EnumType::new(
        Some("public"),
        "order_status",
        &["draft", "paid", "cancelled", "refunded"],
    );

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum OrderStatus {
        #[serde(rename = "draft")]
        Draft,
        #[serde(rename = "paid")]
        Paid,
        #[serde(rename = "cancelled")]
        Cancelled,
        #[serde(rename = "refunded")]
        Refunded,
    }

    impl OrderStatus {
        pub const fn as_db_str(self) -> &'static str {
            match self {
                Self::Draft => "draft",
                Self::Paid => "paid",
                Self::Cancelled => "cancelled",
                Self::Refunded => "refunded",
            }
        }
    }

    impl DbEnum for OrderStatus {
        const TYPE: EnumType = ORDER_STATUS;

        fn as_db_str(self) -> &'static str {
            Self::as_db_str(self)
        }
    }

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const EMAIL: Field = Field::new("email", FieldType::Text).text_search("simple");
    pub const STATUS: Field = Field::new("status", FieldType::Enum(ORDER_STATUS));
    pub const STATUS_HISTORY: Field = Field::mapped(
        "statusHistory",
        "status_history",
        FieldType::Array(ElemType::Enum(ORDER_STATUS)),
    )
    .sortable(false);
    pub const CHANNEL: Field = Field::new("channel", FieldType::Text);
    pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
    pub const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const ITEMS_COUNT: Field = Field::mapped("itemsCount", "items_count", FieldType::BigInt);
    pub const TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);

    pub fn fields() -> [Field; 10] {
        [
            ID,
            EMAIL,
            STATUS,
            STATUS_HISTORY,
            CHANNEL,
            TAGS,
            METADATA,
            CREATED_AT,
            ITEMS_COUNT,
            TOTAL_CENTS,
        ]
    }

    pub fn dataset() -> Dataset {
        Dataset::view("order_search_view").fields(fields())
    }
}

mod orders_table {
    use super::*;

    pub const ORDER_STATUS: EnumType = EnumType::new(
        Some("public"),
        "order_status",
        &["draft", "paid", "cancelled", "refunded"],
    );

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
    pub const STATUS: Field = Field::new("status", FieldType::Enum(ORDER_STATUS));
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

    pub fn dataset() -> Dataset {
        Dataset::table("orders").fields([ID, USER_ID, STATUS, CREATED_AT])
    }
}

mod users_table {
    use super::*;

    pub const USER_STATUS: EnumType =
        EnumType::new(Some("public"), "user_status", &["active", "disabled"]);

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const EMAIL: Field = Field::new("email", FieldType::Text);
    pub const STATUS: Field = Field::new("status", FieldType::Enum(USER_STATUS));

    pub fn dataset() -> Dataset {
        Dataset::table("app_users").fields([ID, EMAIL, STATUS])
    }
}

mod events_table {
    use super::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const ORDER_ID: Field = Field::mapped("orderId", "order_id", FieldType::Uuid);
    pub const EVENT_TYPE: Field = Field::mapped("eventType", "event_type", FieldType::Text);
    pub const PAYLOAD: Field = Field::new("payload", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

    pub fn dataset() -> Dataset {
        Dataset::table("events").fields([ID, ORDER_ID, EVENT_TYPE, PAYLOAD, CREATED_AT])
    }
}

mod organizations_table {
    use super::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const SLUG: Field = Field::new("slug", FieldType::Text);
    pub const NAME: Field = Field::new("name", FieldType::Text);

    pub fn dataset() -> Dataset {
        Dataset::table("organizations").fields([ID, SLUG, NAME])
    }
}

mod withdrawals_table {
    use super::*;

    pub const UINT_256: TypeSpec = TypeSpec::domain(Some("public"), "uint_256")
        .base(TypeFamily::Numeric)
        .value_repr(ValueRepr::DecimalString)
        .select_repr(SelectRepr::Text);

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
    pub const AMOUNT: Field = Field::new("amount", FieldType::Custom(&UINT_256));
    pub const AMOUNT_HISTORY: Field = Field::mapped(
        "amountHistory",
        "amount_history",
        FieldType::Array(ElemType::Custom(&UINT_256)),
    );
    pub const WALLET_ADDRESS: Field =
        Field::mapped("walletAddress", "wallet_address", FieldType::Text);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

    pub fn dataset() -> Dataset {
        Dataset::table("withdrawals").fields([
            ID,
            USER_ID,
            AMOUNT,
            AMOUNT_HISTORY,
            WALLET_ADDRESS,
            CREATED_AT,
        ])
    }
}

mod pg_type_examples {
    use super::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const DISPLAY_NAME: Field = Field::mapped("displayName", "display_name", FieldType::Citext);
    pub const PAYLOAD: Field = Field::new("payload", FieldType::Bytea);
    pub const IP_ADDR: Field = Field::mapped("ipAddr", "ip_addr", FieldType::Inet);
    pub const NETWORK: Field = Field::new("network", FieldType::Cidr);
    pub const ACTIVE_WINDOW: Field = Field::mapped(
        "activeWindow",
        "active_window",
        FieldType::Range(ElemType::Timestamptz),
    );
    pub const LOCAL_WINDOW: Field = Field::mapped(
        "localWindow",
        "local_window",
        FieldType::Range(ElemType::Timestamp),
    );
    pub const BILLING_DATES: Field = Field::mapped(
        "billingDates",
        "billing_dates",
        FieldType::Range(ElemType::Date),
    );
    pub const CREATED_LOCAL: Field =
        Field::mapped("createdLocal", "created_local", FieldType::Timestamp);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

    pub fn dataset() -> Dataset {
        Dataset::table("pg_type_examples").fields([
            ID,
            DISPLAY_NAME,
            PAYLOAD,
            IP_ADDR,
            NETWORK,
            ACTIVE_WINDOW,
            LOCAL_WINDOW,
            BILLING_DATES,
            CREATED_LOCAL,
            CREATED_AT,
        ])
    }
}

#[tokio::test]
async fn executes_view_query_with_jsonb_arrays_projection_and_count() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let built = select(order_search::dataset())
        .fields([
            order_search::ID,
            order_search::EMAIL,
            order_search::TOTAL_CENTS,
            order_search::CREATED_AT,
        ])
        .filter(all([
            order_search::STATUS.eq(order_search::OrderStatus::Paid),
            order_search::STATUS_HISTORY.has(order_search::OrderStatus::Paid),
            order_search::TAGS.contains_any(["vip", "gift"]),
            order_search::METADATA.path("score").gte(80),
            order_search::CREATED_AT.gte("2026-01-01T00:00:00Z"),
        ]))
        .order_by(order_search::CREATED_AT.desc())
        .limit(10)
        .build_pg()?;

    let rows = query(&client, &built.rows).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>("email"), "ada@example.com");
    assert_eq!(rows[0].get::<_, i64>("totalCents"), 15_900);
    assert_count(&client, &built.count, 1).await?;
    Ok(())
}

#[tokio::test]
async fn round_trips_custom_numeric_domain_without_losing_precision() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Withdrawal {
        id: String,
        amount: String,
        amount_history: Vec<String>,
    }

    let high_value = "900719925474099312345678901234567890";
    let rows: Vec<Withdrawal> = select(withdrawals_table::dataset())
        .fields([
            withdrawals_table::ID,
            withdrawals_table::AMOUNT,
            withdrawals_table::AMOUNT_HISTORY,
        ])
        .filter(all([
            withdrawals_table::AMOUNT.gte(high_value),
            withdrawals_table::AMOUNT_HISTORY.has(high_value),
        ]))
        .fetch_all_as(&client)
        .await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "60000000-0000-0000-0000-000000000001");
    assert_eq!(rows[0].amount, high_value);
    assert_eq!(rows[0].amount_history, vec!["1", high_value]);

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NewWithdrawal<'a> {
        id: &'a str,
        user_id: &'a str,
        amount: &'a str,
        amount_history: Vec<&'a str>,
        wallet_address: &'a str,
    }

    let inserted = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
    insert(withdrawals_table::dataset())
        .value(&NewWithdrawal {
            id: "60000000-0000-0000-0000-000000000003",
            user_id: "10000000-0000-0000-0000-000000000001",
            amount: inserted,
            amount_history: vec!["42", inserted],
            wallet_address: "0xfeed",
        })
        .execute(&client)
        .await?;

    let row: Withdrawal = select(withdrawals_table::dataset())
        .fields([
            withdrawals_table::ID,
            withdrawals_table::AMOUNT,
            withdrawals_table::AMOUNT_HISTORY,
        ])
        .filter(all([
            withdrawals_table::ID.eq("60000000-0000-0000-0000-000000000003"),
            withdrawals_table::AMOUNT_HISTORY.contains_all(["42", inserted]),
        ]))
        .fetch_one_as(&client)
        .await?;
    assert_eq!(row.amount, inserted);
    assert_eq!(row.amount_history, vec!["42", inserted]);

    Ok(())
}

#[tokio::test]
async fn executes_native_postgres_type_filters_and_mapping() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PgTypeExample {
        display_name: String,
        payload: Vec<u8>,
        ip_addr: String,
        network: String,
        active_window: String,
        local_window: String,
        billing_dates: String,
        created_local: String,
        created_at: String,
    }

    let rows: Vec<PgTypeExample> = select(pg_type_examples::dataset())
        .fields([
            pg_type_examples::DISPLAY_NAME,
            pg_type_examples::PAYLOAD,
            pg_type_examples::IP_ADDR,
            pg_type_examples::NETWORK,
            pg_type_examples::ACTIVE_WINDOW,
            pg_type_examples::LOCAL_WINDOW,
            pg_type_examples::BILLING_DATES,
            pg_type_examples::CREATED_LOCAL,
            pg_type_examples::CREATED_AT,
        ])
        .filter(all([
            pg_type_examples::DISPLAY_NAME.eq("ada"),
            pg_type_examples::PAYLOAD.is_in(vec![
                Value::bytes([0xde, 0xad, 0xbe, 0xef]),
                Value::bytes([0xca, 0xfe]),
            ]),
            pg_type_examples::IP_ADDR.is_in(["10.1.2.3", "10.1.2.4"]),
            pg_type_examples::NETWORK.contains("10.1.2.0/24"),
            pg_type_examples::ACTIVE_WINDOW.is_in([
                "[2026-02-01T00:00:00Z,2026-03-01T00:00:00Z)",
                "[2026-04-01T00:00:00Z,2026-05-01T00:00:00Z)",
            ]),
            pg_type_examples::ACTIVE_WINDOW.overlaps("[2026-02-15T00:00:00Z,2026-02-20T00:00:00Z)"),
            pg_type_examples::BILLING_DATES.contains("[2026-02-10,2026-02-11)"),
        ]))
        .fetch_all_as(&client)
        .await?;

    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.display_name, "Ada");
    assert_eq!(row.payload, vec![0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(row.ip_addr, "10.1.2.3/32");
    assert_eq!(row.network, "10.1.0.0/16");
    assert!(row.active_window.contains("2026-02-01"));
    assert!(row.local_window.contains("2026-02-01"));
    assert_eq!(row.billing_dates, "[2026-02-01,2026-03-01)");
    assert_eq!(row.created_local, "2026-02-01 12:30:00");
    assert!(row.created_at.starts_with("2026-02-01T12:30:00"));

    Ok(())
}

#[tokio::test]
async fn executes_raw_cte_as_search_source() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let recent_paid = cte(
        "recent_paid",
        raw("SELECT * FROM order_search_view \
             WHERE status = ?::text::order_status AND created_at >= ?::text::timestamptz")
        .bind("paid")
        .bind("2026-01-01T00:00:00Z"),
    );

    let built = select(Dataset::cte("recent_paid").fields(order_search::fields()))
        .cte(recent_paid)
        .fields([order_search::EMAIL, order_search::TOTAL_CENTS])
        .filter(order_search::TOTAL_CENTS.gte(10_000))
        .order_by(order_search::TOTAL_CENTS.desc())
        .build_pg()?;

    let rows = query(&client, &built.rows).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>("email"), "ada@example.com");
    assert_count(&client, &built.count, 1).await?;
    Ok(())
}

#[tokio::test]
async fn executes_raw_source_with_safe_outer_filtering() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let source = Dataset::raw(
        "SELECT id, email, status, status_history, channel, tags, metadata, created_at, items_count, total_cents \
         FROM order_search_view WHERE total_cents > 0",
        "order_rollup",
    )
    .fields(order_search::fields());

    let built = select(source)
        .fields([order_search::EMAIL, order_search::STATUS])
        .filter(
            field("email")
                .ends_with("@example.com")
                .and(field("status").eq("paid")),
        )
        .sort_asc("email")
        .limit(10)
        .build_pg()?;

    let rows = query(&client, &built.rows).await?;
    let emails = rows
        .iter()
        .map(|row| row.get::<_, String>("email"))
        .collect::<Vec<_>>();
    assert_eq!(
        emails,
        vec!["ada@example.com", "grace@example.com", "linus@example.com"]
    );
    assert_count(&client, &built.count, 3).await?;
    Ok(())
}

#[tokio::test]
async fn maps_expression_select_items_into_structs() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    struct ExpressionRow {
        email: String,
        label: String,
        campaign: String,
        score_text: String,
        #[serde(rename = "statusLabel")]
        status_label: String,
        #[serde(rename = "totalText")]
        total_text: String,
    }

    let rows = select(order_search::dataset())
        .select([order_search::EMAIL])
        .select_expr(
            coalesce([order_search::CHANNEL.expr(), order_search::EMAIL.expr()]).alias("label"),
        )
        .select_expr(
            order_search::METADATA
                .json_text("campaign")
                .alias("campaign"),
        )
        .select_expr(
            order_search::METADATA
                .json_path_text(["score"])
                .alias("score_text"),
        )
        .select_expr(
            case_when(order_search::STATUS.eq(order_search::OrderStatus::Paid))
                .then("settled")
                .otherwise("open")
                .alias("statusLabel"),
        )
        .select_expr(cast(order_search::TOTAL_CENTS.expr(), FieldType::Text).alias("totalText"))
        .filter(order_search::EMAIL.eq("ada@example.com"))
        .fetch_all_as::<ExpressionRow>(&client)
        .await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].email, "ada@example.com");
    assert_eq!(rows[0].label, "web");
    assert_eq!(rows[0].campaign, "spring");
    assert_eq!(rows[0].score_text, "92");
    assert_eq!(rows[0].status_label, "settled");
    assert_eq!(rows[0].total_text, "15900");
    Ok(())
}

#[tokio::test]
async fn accepts_json_api_request_and_runs_same_validation_pipeline() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let request: SearchRequest = serde_json::from_value(serde_json::json!({
        "fields": ["id", "email", "totalCents"],
        "limit": 5,
        "sort": [{ "field": "totalCents", "dir": "DESC" }],
        "query": {
            "logical": "and",
            "predicates": [
                { "field": "status", "operator": "equals", "value": "paid" },
                { "field": "metadata.campaign", "operator": "equals", "value": "spring" }
            ]
        }
    }))?;

    let built = select(order_search::dataset())
        .request(request)
        .build_pg()?;
    let rows = query(&client, &built.rows).await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<_, i64>("totalCents"), 15_900);
    assert_eq!(rows[1].get::<_, i64>("totalCents"), 10_900);
    assert_count(&client, &built.count, 2).await?;
    Ok(())
}

#[tokio::test]
async fn executes_first_class_join_query() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let built = select(orders_table::dataset().alias("o"))
        .join(
            users_table::dataset().alias("u"),
            orders_table::USER_ID
                .on("o")
                .eq_col(users_table::ID.on("u")),
        )
        .fields([
            orders_table::ID.on("o"),
            users_table::EMAIL.on("u"),
            orders_table::STATUS.on("o"),
        ])
        .filter(all([
            users_table::EMAIL.on("u").eq("ada@example.com"),
            orders_table::STATUS
                .on("o")
                .eq(order_search::OrderStatus::Paid),
        ]))
        .order_by(orders_table::CREATED_AT.on("o").desc())
        .limit(10)
        .build_pg()?;

    let rows = query(&client, &built.rows).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>("u_email"), "ada@example.com");
    assert_eq!(rows[0].get::<_, String>("status"), "paid");
    assert_count(&client, &built.count, 1).await?;
    Ok(())
}

#[tokio::test]
async fn executes_correlated_exists_and_in_subquery() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let paid_event_orders = select(orders_table::dataset().alias("o"))
        .fields([orders_table::ID.on("o")])
        .filter(exists(
            select(events_table::dataset().alias("e")).filter(all([
                events_table::ORDER_ID
                    .on("e")
                    .eq_col(orders_table::ID.on("o")),
                events_table::EVENT_TYPE.on("e").eq("paid"),
            ])),
        ))
        .order_by(orders_table::CREATED_AT.on("o").asc())
        .build_pg()?;

    let rows = query(&client, &paid_event_orders.rows).await?;
    assert_eq!(rows.len(), 2);
    assert_count(&client, &paid_event_orders.count, 2).await?;

    let users_with_paid_orders = select(users_table::dataset())
        .fields([users_table::EMAIL])
        .filter(
            users_table::ID.in_subquery(
                select(orders_table::dataset().alias("o"))
                    .fields([orders_table::USER_ID.on("o")])
                    .filter(
                        orders_table::STATUS
                            .on("o")
                            .eq(order_search::OrderStatus::Paid),
                    ),
            ),
        )
        .order_by(users_table::EMAIL.asc())
        .build_pg()?;

    let rows = query(&client, &users_with_paid_orders.rows).await?;
    let emails = rows
        .iter()
        .map(|row| row.get::<_, String>("email"))
        .collect::<Vec<_>>();
    assert_eq!(
        emails,
        vec!["ada@example.com", "grace@example.com", "linus@example.com"]
    );

    let orders_without_events = select(orders_table::dataset().alias("o"))
        .fields([orders_table::ID.on("o")])
        .filter(not_exists(
            select(events_table::dataset().alias("e")).filter(
                events_table::ORDER_ID
                    .on("e")
                    .eq_col(orders_table::ID.on("o")),
            ),
        ))
        .build_pg()?;
    assert_count(&client, &orders_without_events.count, 1).await?;
    Ok(())
}

#[tokio::test]
async fn executes_set_queries_and_set_subqueries() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    struct EmailRow {
        email: String,
    }

    let active = select(users_table::dataset())
        .fields([users_table::EMAIL])
        .filter(users_table::STATUS.eq("active"));
    let disabled = select(users_table::dataset())
        .fields([users_table::EMAIL])
        .filter(users_table::STATUS.eq("disabled"));

    let emails = union(active, disabled)
        .order_by(field("email").asc())
        .fetch_all_as::<EmailRow>(&client)
        .await?
        .into_iter()
        .map(|row| row.email)
        .collect::<Vec<_>>();
    assert_eq!(
        emails,
        vec!["ada@example.com", "grace@example.com", "linus@example.com"]
    );

    let paid_or_draft_user_ids = union_all(
        select(orders_table::dataset().alias("paid"))
            .fields([orders_table::USER_ID.on("paid")])
            .filter(orders_table::STATUS.on("paid").eq("paid")),
        select(orders_table::dataset().alias("draft"))
            .fields([orders_table::USER_ID.on("draft")])
            .filter(orders_table::STATUS.on("draft").eq("draft")),
    );

    let mut rows = select(users_table::dataset())
        .fields([users_table::EMAIL])
        .filter(users_table::ID.in_subquery(paid_or_draft_user_ids))
        .fetch_all_as::<EmailRow>(&client)
        .await?
        .into_iter()
        .map(|row| row.email)
        .collect::<Vec<_>>();
    rows.sort();
    assert_eq!(
        rows,
        vec!["ada@example.com", "grace@example.com", "linus@example.com"]
    );
    Ok(())
}

#[tokio::test]
async fn executes_subquery_sources_and_lateral_joins() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let paid_orders = select(orders_table::dataset().alias("o"))
        .fields([orders_table::ID.on("o"), orders_table::USER_ID.on("o")])
        .filter(orders_table::STATUS.on("o").eq("paid"))
        .into_source("paid_orders")
        .fields([orders_table::ID, orders_table::USER_ID]);

    let paid_rows = select(paid_orders)
        .fields([orders_table::USER_ID])
        .fetch_all(&client)
        .await?;
    assert_eq!(paid_rows.len(), 3);

    let latest_order = select(orders_table::dataset().alias("o"))
        .fields([orders_table::STATUS.on("o")])
        .filter(
            orders_table::USER_ID
                .on("o")
                .eq_col(users_table::ID.on("u")),
        )
        .order_by(orders_table::CREATED_AT.on("o").desc())
        .limit(1)
        .into_source("latest_order")
        .fields([orders_table::STATUS]);

    let latest = select(users_table::dataset().alias("u"))
        .fields([
            users_table::EMAIL.on("u"),
            orders_table::STATUS
                .on("latest_order")
                .alias("latestStatus"),
        ])
        .left_join_lateral(latest_order, raw("TRUE"))
        .filter(users_table::EMAIL.on("u").eq("ada@example.com"))
        .fetch_one(&client)
        .await?;

    assert_eq!(latest.get::<_, String>("email"), "ada@example.com");
    assert_eq!(latest.get::<_, String>("latestStatus"), "draft");
    Ok(())
}

#[tokio::test]
async fn executor_api_runs_rows_optional_and_count() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let rows = select(order_search::dataset())
        .fields([order_search::EMAIL, order_search::TOTAL_CENTS])
        .filter(order_search::STATUS.eq(order_search::OrderStatus::Paid))
        .order_by(order_search::TOTAL_CENTS.desc())
        .limit(2)
        .fetch_all(&client)
        .await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<_, String>("email"), "ada@example.com");

    let one = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(order_search::EMAIL.eq("ada@example.com"))
        .fetch_one(&client)
        .await?;
    assert_eq!(one.get::<_, String>("email"), "ada@example.com");

    let history = select(order_search::dataset())
        .fields([order_search::STATUS_HISTORY])
        .filter(order_search::EMAIL.eq("ada@example.com"))
        .fetch_one(&client)
        .await?;
    assert_eq!(
        history.get::<_, Vec<String>>("statusHistory"),
        vec!["draft".to_owned(), "paid".to_owned()]
    );

    let none = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(order_search::EMAIL.eq("nobody@example.com"))
        .fetch_optional(&client)
        .await?;
    assert!(none.is_none());

    let total = select(order_search::dataset())
        .filter(order_search::STATUS.eq(order_search::OrderStatus::Paid))
        .count(&client)
        .await?;
    assert_eq!(total, 3);
    Ok(())
}

#[tokio::test]
async fn raw_query_executes_maps_rows_and_validates_binds() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RawOrder {
        id: String,
        email: String,
        total_cents: i64,
        created_at: String,
    }

    let rows = raw_query(
        "SELECT id, email, total_cents AS \"totalCents\", created_at AS \"createdAt\" \
         FROM order_search_view \
         WHERE status = ?::text::order_status AND total_cents > ?::bigint \
         ORDER BY total_cents DESC",
    )
    .bind("paid")
    .bind(10_000)
    .fetch_all_as::<RawOrder>(&client)
    .await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].email, "ada@example.com");
    assert_eq!(rows[0].total_cents, 15_900);
    assert!(rows[0].created_at.starts_with("2026-02-01T10:00:00"));
    assert_eq!(rows[1].id, "30000000-0000-0000-0000-000000000004");

    let version: String = raw_query("SELECT 'rqb'::text")
        .fetch_one_scalar(&client)
        .await?;
    assert_eq!(version, "rqb");

    let missing: Option<RawOrder> = raw_query(
        "SELECT id, email, total_cents AS \"totalCents\", created_at AS \"createdAt\" \
         FROM order_search_view WHERE email = ?",
    )
    .bind("nobody@example.com")
    .fetch_optional_as(&client)
    .await?;
    assert!(missing.is_none());

    #[derive(Debug, Deserialize)]
    struct Escaped {
        literal: String,
        value: String,
    }

    let escaped: Escaped = raw_query("SELECT '??' AS literal, ?::text AS value")
        .bind("bound")
        .fetch_one_as(&client)
        .await?;
    assert_eq!(escaped.literal, "?");
    assert_eq!(escaped.value, "bound");

    let updated = raw_query("UPDATE app_users SET profile = profile || ?::jsonb WHERE email = ?")
        .bind(serde_json::json!({ "rawQuery": true }))
        .bind("ada@example.com")
        .execute(&client)
        .await?;
    assert_eq!(updated, 1);

    let raw_flag: bool =
        raw_query("SELECT (profile->>'rawQuery')::bool FROM app_users WHERE email = ?")
            .bind("ada@example.com")
            .fetch_one_scalar(&client)
            .await?;
    assert!(raw_flag);

    let err = raw_query("SELECT ?").build_pg().unwrap_err();
    assert!(matches!(
        err,
        PgError::Core(rqb_core::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        })
    ));

    let unsupported = raw_query("SELECT 1.23::numeric AS amount")
        .fetch_one_as::<serde_json::Value>(&client)
        .await
        .unwrap_err();
    assert!(matches!(
        unsupported,
        PgError::Deserialize(message)
            if message.contains("unsupported Postgres type `numeric`")
    ));

    Ok(())
}

#[cfg(all(feature = "with-uuid", feature = "with-chrono"))]
#[tokio::test]
async fn uuid_chrono_and_page_helpers_are_ergonomic() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TypedOrder {
        id: uuid::Uuid,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let id = uuid::Uuid::parse_str("30000000-0000-0000-0000-000000000001")?;
    let page = select(order_search::dataset())
        .fields([order_search::ID, order_search::CREATED_AT])
        .filter(order_search::ID.eq(id))
        .limit(10)
        .page_as::<TypedOrder>(&client)
        .await?;

    assert_eq!(page.total, 1);
    assert_eq!(page.limit, 10);
    assert_eq!(page.offset, 0);
    assert_eq!(page.items[0].id, id);
    assert_eq!(
        page.items[0].created_at.to_rfc3339(),
        "2026-02-01T10:00:00+00:00"
    );
    Ok(())
}

#[tokio::test]
async fn executes_insert_update_delete_and_upsert() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NewEvent<'a> {
        id: &'a str,
        order_id: &'a str,
        event_type: &'a str,
        payload: serde_json::Value,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EventRow {
        id: String,
        event_type: String,
        payload: serde_json::Value,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EventExpressionRow {
        event_type: String,
        event_type_lower: String,
        payload: serde_json::Value,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EventOrderRow {
        id: String,
        order_id: String,
    }

    let event_id = "50000000-0000-0000-0000-000000009901";
    let default_returning_event_id = "50000000-0000-0000-0000-000000009906";
    let order_id = "30000000-0000-0000-0000-000000000001";
    let _ = delete(events_table::dataset())
        .filter(events_table::ID.is_in([event_id, default_returning_event_id]))
        .execute(&client)
        .await;

    let inserted: EventRow = insert(events_table::dataset())
        .value(&NewEvent {
            id: event_id,
            order_id,
            event_type: "rqb-write",
            payload: serde_json::json!({ "step": 1 }),
        })
        .returning([
            events_table::ID,
            events_table::EVENT_TYPE,
            events_table::PAYLOAD,
        ])
        .fetch_one_as(&client)
        .await?;
    assert_eq!(inserted.id, event_id);
    assert_eq!(inserted.event_type, "rqb-write");
    assert_eq!(inserted.payload["step"], 1);

    let default_inserted: EventRow = insert(events_table::dataset())
        .value(&NewEvent {
            id: default_returning_event_id,
            order_id,
            event_type: "rqb-default-returning",
            payload: serde_json::json!({ "defaultReturning": true }),
        })
        .fetch_one_as(&client)
        .await?;
    assert_eq!(default_inserted.id, default_returning_event_id);
    assert_eq!(default_inserted.event_type, "rqb-default-returning");
    assert_eq!(default_inserted.payload["defaultReturning"], true);

    let upserted: EventRow = insert(events_table::dataset())
        .set(events_table::ID, event_id)
        .set(events_table::ORDER_ID, order_id)
        .set(events_table::EVENT_TYPE, "rqb-upsert")
        .set(events_table::PAYLOAD, serde_json::json!({ "step": 2 }))
        .on_conflict(events_table::ID)
        .do_update([events_table::EVENT_TYPE, events_table::PAYLOAD])
        .returning([
            events_table::ID,
            events_table::EVENT_TYPE,
            events_table::PAYLOAD,
        ])
        .fetch_one_as(&client)
        .await?;
    assert_eq!(upserted.event_type, "rqb-upsert");
    assert_eq!(upserted.payload["step"], 2);

    let updated: EventRow = update(events_table::dataset())
        .set(events_table::EVENT_TYPE, "rqb-updated")
        .set_raw(
            events_table::PAYLOAD,
            raw("payload || ?::jsonb").bind(serde_json::json!({ "updated": true })),
        )
        .filter(events_table::ID.eq(event_id))
        .returning([
            events_table::ID,
            events_table::EVENT_TYPE,
            events_table::PAYLOAD,
        ])
        .fetch_one_as(&client)
        .await?;
    assert_eq!(updated.event_type, "rqb-updated");
    assert_eq!(updated.payload["updated"], true);

    let expression_updated: EventExpressionRow = update(events_table::dataset())
        .set_expr(events_table::EVENT_TYPE, upper(events_table::EVENT_TYPE))
        .set_default(events_table::PAYLOAD)
        .filter(events_table::ID.eq(event_id))
        .returning([events_table::EVENT_TYPE, events_table::PAYLOAD])
        .returning_expr(lower(events_table::EVENT_TYPE).alias("eventTypeLower"))
        .fetch_one_as(&client)
        .await?;
    assert_eq!(expression_updated.event_type, "RQB-UPDATED");
    assert_eq!(expression_updated.event_type_lower, "rqb-updated");
    assert_eq!(expression_updated.payload, serde_json::json!({}));

    let custom_upserted: EventRow = insert(events_table::dataset())
        .set(events_table::ID, event_id)
        .set(events_table::ORDER_ID, order_id)
        .set(events_table::EVENT_TYPE, "rqb-custom-upsert")
        .set(
            events_table::PAYLOAD,
            serde_json::json!({ "ignored": true }),
        )
        .on_conflict(events_table::ID)
        .do_update_set([
            set_expr(events_table::EVENT_TYPE, excluded(events_table::EVENT_TYPE)),
            set_default(events_table::PAYLOAD),
        ])
        .returning([
            events_table::ID,
            events_table::EVENT_TYPE,
            events_table::PAYLOAD,
        ])
        .fetch_one_as(&client)
        .await?;
    assert_eq!(custom_upserted.event_type, "rqb-custom-upsert");
    assert_eq!(custom_upserted.payload, serde_json::json!({}));

    let update_from: EventOrderRow = update(events_table::dataset().alias("e"))
        .from(orders_table::dataset().alias("o"))
        .set_col(events_table::ORDER_ID, orders_table::ID.on("o"))
        .filter(events_table::ID.on("e").eq(event_id))
        .filter(
            events_table::ORDER_ID
                .on("e")
                .eq_col(orders_table::ID.on("o")),
        )
        .returning([
            events_table::ID.on("e").alias("id"),
            events_table::ORDER_ID.on("e").alias("orderId"),
        ])
        .fetch_one_as(&client)
        .await?;
    assert_eq!(update_from.id, event_id);
    assert_eq!(update_from.order_id, order_id);

    let deleted: EventRow = delete(events_table::dataset().alias("e"))
        .using(orders_table::dataset().alias("o"))
        .filter(events_table::ID.on("e").eq(event_id))
        .filter(
            events_table::ORDER_ID
                .on("e")
                .eq_col(orders_table::ID.on("o")),
        )
        .returning([
            events_table::ID.on("e").alias("id"),
            events_table::EVENT_TYPE.on("e").alias("eventType"),
            events_table::PAYLOAD.on("e").alias("payload"),
        ])
        .fetch_one_as(&client)
        .await?;
    assert_eq!(deleted.id, event_id);
    delete(events_table::dataset())
        .filter(events_table::ID.eq(default_returning_event_id))
        .execute(&client)
        .await?;
    Ok(())
}

#[cfg(feature = "pool")]
#[tokio::test]
async fn db_pool_executes_queries_and_transactions() -> TestResult {
    let Some(url) = database_url() else {
        eprintln!("skipping Postgres integration test; set RQB_TEST_DATABASE_URL");
        return Ok(());
    };
    let db = rqb_postgres::connect(&url).await?;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EventRow {
        id: String,
        event_type: String,
    }

    let committed_id = "50000000-0000-0000-0000-000000009904";
    let rolled_back_id = "50000000-0000-0000-0000-000000009905";
    let savepoint_rolled_back_id = "50000000-0000-0000-0000-000000009906";
    let savepoint_released_id = "50000000-0000-0000-0000-000000009907";
    let order_id = "30000000-0000-0000-0000-000000000001";

    let cache_client = db.get().await?;
    assert_eq!(cache_client.statement_cache.size(), 0);
    select(events_table::dataset())
        .fields([events_table::ID])
        .filter(events_table::EVENT_TYPE.eq("rqb-cache-probe"))
        .fetch_all(&cache_client)
        .await?;
    assert_eq!(cache_client.statement_cache.size(), 1);

    select(events_table::dataset())
        .fields([events_table::ID])
        .filter(events_table::EVENT_TYPE.is_in(["paid", "created"]))
        .fetch_all(&cache_client)
        .await?;
    let cached_after_in = cache_client.statement_cache.size();
    assert_eq!(cached_after_in, 2);

    select(events_table::dataset())
        .fields([events_table::ID])
        .filter(events_table::EVENT_TYPE.is_in(["paid", "created", "rqb-cache-probe"]))
        .fetch_all(&cache_client)
        .await?;
    assert_eq!(cache_client.statement_cache.size(), cached_after_in);

    let raw_version: String = raw_query("SELECT ?::text")
        .bind("raw")
        .fetch_one_scalar(&cache_client)
        .await?;
    assert_eq!(raw_version, "raw");
    assert_eq!(cache_client.statement_cache.size(), cached_after_in);

    let dynamic_request = SearchRequest {
        query: Some(events_table::PAYLOAD.path("cacheProbe").eq(true)),
        ..SearchRequest::new()
    };
    select(events_table::dataset())
        .fields([events_table::ID])
        .request(dynamic_request)
        .fetch_all(&cache_client)
        .await?;
    assert_eq!(cache_client.statement_cache.size(), cached_after_in);
    drop(cache_client);

    let _ = delete(events_table::dataset())
        .filter(events_table::ID.is_in([
            committed_id,
            rolled_back_id,
            savepoint_rolled_back_id,
            savepoint_released_id,
        ]))
        .execute(&db)
        .await;

    let read_only_tx = db.begin().serializable().read_only().deferrable().await?;
    let isolation = read_only_tx
        .query_one("SHOW transaction_isolation", &[], StatementCache::Bypass)
        .await?
        .get::<_, String>(0);
    let read_only = read_only_tx
        .query_one("SHOW transaction_read_only", &[], StatementCache::Bypass)
        .await?
        .get::<_, String>(0);
    let deferrable = read_only_tx
        .query_one("SHOW transaction_deferrable", &[], StatementCache::Bypass)
        .await?
        .get::<_, String>(0);
    assert_eq!(isolation, "serializable");
    assert_eq!(read_only, "on");
    assert_eq!(deferrable, "on");
    read_only_tx.rollback().await?;

    let tx = db.begin().serializable().await?;
    insert(events_table::dataset())
        .set(events_table::ID, committed_id)
        .set(events_table::ORDER_ID, order_id)
        .set(events_table::EVENT_TYPE, "rqb-pool-commit")
        .set(events_table::PAYLOAD, serde_json::json!({ "pool": true }))
        .execute(&tx)
        .await?;

    let before_commit: Option<EventRow> = select(events_table::dataset())
        .fields([events_table::ID, events_table::EVENT_TYPE])
        .filter(events_table::ID.eq(committed_id))
        .fetch_optional_as(&db)
        .await?;
    assert!(before_commit.is_none());

    tx.commit().await?;

    let committed: EventRow = select(events_table::dataset())
        .fields([events_table::ID, events_table::EVENT_TYPE])
        .filter(events_table::ID.eq(committed_id))
        .fetch_one_as(&db)
        .await?;
    assert_eq!(committed.id, committed_id);
    assert_eq!(committed.event_type, "rqb-pool-commit");

    let result = db
        .transaction(rqb_postgres::txn!(|tx| {
            insert(events_table::dataset())
                .set(events_table::ID, rolled_back_id)
                .set(events_table::ORDER_ID, order_id)
                .set(events_table::EVENT_TYPE, "rqb-pool-rollback")
                .set(events_table::PAYLOAD, serde_json::json!({ "pool": true }))
                .execute(tx)
                .await?;
            Err::<(), rqb_postgres::Error>(rqb_postgres::Error::Connection(
                "force rollback".to_owned(),
            ))
        }))
        .await;
    assert!(result.is_err());

    let rolled_back: Option<EventRow> = select(events_table::dataset())
        .fields([events_table::ID, events_table::EVENT_TYPE])
        .filter(events_table::ID.eq(rolled_back_id))
        .fetch_optional_as(&db)
        .await?;
    assert!(rolled_back.is_none());

    let tx = db.begin().await?;
    let savepoint = tx.savepoint("rollback savepoint").await?;
    assert_eq!(savepoint.name(), "rollback savepoint");
    insert(events_table::dataset())
        .set(events_table::ID, savepoint_rolled_back_id)
        .set(events_table::ORDER_ID, order_id)
        .set(events_table::EVENT_TYPE, "rqb-savepoint-rollback")
        .set(
            events_table::PAYLOAD,
            serde_json::json!({ "savepoint": "rollback" }),
        )
        .execute(&savepoint)
        .await?;
    savepoint.rollback().await?;
    let savepoint_rolled_back: Option<EventRow> = select(events_table::dataset())
        .fields([events_table::ID, events_table::EVENT_TYPE])
        .filter(events_table::ID.eq(savepoint_rolled_back_id))
        .fetch_optional_as(&tx)
        .await?;
    assert!(savepoint_rolled_back.is_none());

    let savepoint = tx.savepoint("release_savepoint").await?;
    insert(events_table::dataset())
        .set(events_table::ID, savepoint_released_id)
        .set(events_table::ORDER_ID, order_id)
        .set(events_table::EVENT_TYPE, "rqb-savepoint-release")
        .set(
            events_table::PAYLOAD,
            serde_json::json!({ "savepoint": "release" }),
        )
        .execute(&savepoint)
        .await?;
    savepoint.release().await?;
    tx.commit().await?;

    let savepoint_released: EventRow = select(events_table::dataset())
        .fields([events_table::ID, events_table::EVENT_TYPE])
        .filter(events_table::ID.eq(savepoint_released_id))
        .fetch_one_as(&db)
        .await?;
    assert_eq!(savepoint_released.id, savepoint_released_id);
    assert_eq!(savepoint_released.event_type, "rqb-savepoint-release");

    delete(events_table::dataset())
        .filter(events_table::ID.is_in([committed_id, savepoint_released_id]))
        .execute(&db)
        .await?;
    Ok(())
}

#[tokio::test]
async fn maps_postgres_execution_errors_and_result_ext() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    client.batch_execute("SAVEPOINT duplicate_org").await?;
    let duplicate = insert(organizations_table::dataset())
        .set(
            organizations_table::ID,
            "00000000-0000-0000-0000-000000009901",
        )
        .set(organizations_table::SLUG, "acme")
        .set(organizations_table::NAME, "Duplicate Acme")
        .execute(&client)
        .await
        .unwrap_err();
    assert!(matches!(duplicate, PgError::UniqueViolation { .. }));
    assert_eq!(duplicate.constraint_name(), Some("organizations_slug_key"));
    client
        .batch_execute("ROLLBACK TO SAVEPOINT duplicate_org")
        .await?;

    client.batch_execute("SAVEPOINT bad_fk").await?;
    let fk = insert(events_table::dataset())
        .set(events_table::ID, "50000000-0000-0000-0000-000000009902")
        .set(
            events_table::ORDER_ID,
            "30000000-0000-0000-0000-999999999999",
        )
        .set(events_table::EVENT_TYPE, "bad-fk")
        .set(events_table::PAYLOAD, serde_json::json!({}))
        .execute(&client)
        .await
        .unwrap_err();
    assert!(matches!(fk, PgError::ForeignKeyViolation { .. }));
    client.batch_execute("ROLLBACK TO SAVEPOINT bad_fk").await?;

    client
        .batch_execute("SAVEPOINT query_timeout; SET LOCAL statement_timeout = '1ms'")
        .await?;
    let canceled = raw_query("SELECT pg_sleep(0.05)")
        .fetch_one(&client)
        .await
        .unwrap_err();
    assert!(matches!(canceled, PgError::QueryCanceled { .. }));
    assert_eq!(canceled.code(), Some("57014"));
    assert!(!canceled.is_retryable());
    client
        .batch_execute("ROLLBACK TO SAVEPOINT query_timeout")
        .await?;

    let not_found = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(order_search::EMAIL.eq("nobody@example.com"))
        .fetch_one(&client)
        .await
        .unwrap_err();
    assert!(matches!(not_found, PgError::NotFound));

    let maybe = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(order_search::EMAIL.eq("nobody@example.com"))
        .fetch_one(&client)
        .await
        .optional()?;
    assert!(maybe.is_none());

    #[derive(Debug)]
    enum AppError {
        EmailTaken,
        Db,
    }

    impl From<PgError> for AppError {
        fn from(_: PgError) -> Self {
            Self::Db
        }
    }

    client.batch_execute("SAVEPOINT mapped_duplicate").await?;
    let mapped = insert(organizations_table::dataset())
        .set(
            organizations_table::ID,
            "00000000-0000-0000-0000-000000009903",
        )
        .set(organizations_table::SLUG, "acme")
        .set(organizations_table::NAME, "Duplicate Acme")
        .execute(&client)
        .await
        .on_constraint("organizations_slug_key", |_| AppError::EmailTaken)
        .unwrap_err();
    assert!(matches!(mapped, AppError::EmailTaken));
    client
        .batch_execute("ROLLBACK TO SAVEPOINT mapped_duplicate")
        .await?;

    client.batch_execute("SAVEPOINT mapped_conflict").await?;
    let mapped = insert(organizations_table::dataset())
        .set(
            organizations_table::ID,
            "00000000-0000-0000-0000-000000009904",
        )
        .set(organizations_table::SLUG, "acme")
        .set(organizations_table::NAME, "Duplicate Acme")
        .execute(&client)
        .await
        .on_conflict(|_| AppError::EmailTaken)
        .unwrap_err();
    assert!(matches!(mapped, AppError::EmailTaken));
    client
        .batch_execute("ROLLBACK TO SAVEPOINT mapped_conflict")
        .await?;

    Ok(())
}

#[tokio::test]
async fn fetch_all_as_deserializes_fields_json_arrays_and_aggregates() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    #[derive(Debug, Deserialize)]
    struct Metadata {
        score: i64,
        gift: bool,
        campaign: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OrderRow {
        id: String,
        email: String,
        status: order_search::OrderStatus,
        status_history: Vec<order_search::OrderStatus>,
        tags: Vec<String>,
        metadata: Metadata,
        created_at: String,
        total_cents: i64,
    }

    let order: OrderRow = select(order_search::dataset())
        .fields([
            order_search::ID,
            order_search::EMAIL,
            order_search::STATUS,
            order_search::STATUS_HISTORY,
            order_search::TAGS,
            order_search::METADATA,
            order_search::CREATED_AT,
            order_search::TOTAL_CENTS,
        ])
        .filter(order_search::ID.eq("30000000-0000-0000-0000-000000000001"))
        .fetch_one_as(&client)
        .await?;
    assert_eq!(order.email, "ada@example.com");
    assert_eq!(order.id, "30000000-0000-0000-0000-000000000001");
    assert_eq!(order.status, order_search::OrderStatus::Paid);
    assert_eq!(
        order.status_history,
        vec![
            order_search::OrderStatus::Draft,
            order_search::OrderStatus::Paid
        ]
    );
    assert_eq!(order.tags, vec!["vip", "gift"]);
    assert_eq!(order.metadata.score, 92);
    assert!(order.metadata.gift);
    assert_eq!(order.metadata.campaign, "spring");
    assert!(order.created_at.starts_with("2026-02-01"));
    assert_eq!(order.total_cents, 15_900);

    #[derive(Debug, Deserialize)]
    struct StatusRollup {
        status: order_search::OrderStatus,
        count: i64,
        total: f64,
    }

    let rollups: Vec<StatusRollup> = select(order_search::dataset())
        .fields([order_search::STATUS])
        .agg(count("count"))
        .agg(sum(order_search::TOTAL_CENTS, "total"))
        .group_by([order_search::STATUS])
        .order_by(order_search::STATUS.asc())
        .fetch_all_as(&client)
        .await?;
    let paid = rollups
        .iter()
        .find(|rollup| rollup.status == order_search::OrderStatus::Paid)
        .expect("paid rollup should exist");
    assert_eq!(paid.count, 3);
    assert_eq!(paid.total, 33_800.0);

    #[derive(Debug, Deserialize)]
    struct UserOrders {
        email: String,
        orders: Vec<NestedOrder>,
    }

    #[derive(Debug, Deserialize)]
    struct NestedOrder {
        id: String,
        status: order_search::OrderStatus,
    }

    let nested: Vec<UserOrders> = select(users_table::dataset().alias("u"))
        .join(
            orders_table::dataset().alias("o"),
            orders_table::USER_ID
                .on("o")
                .eq_col(users_table::ID.on("u")),
        )
        .fields([users_table::EMAIL.on("u")])
        .json_agg(
            "orders",
            [orders_table::ID.on("o"), orders_table::STATUS.on("o")],
        )
        .order_within("orders", orders_table::CREATED_AT.on("o").asc())
        .filter(users_table::EMAIL.on("u").eq("ada@example.com"))
        .group_by([users_table::EMAIL.on("u")])
        .fetch_all_as(&client)
        .await?;
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].email, "ada@example.com");
    assert_eq!(nested[0].orders.len(), 2);
    assert_eq!(
        nested[0].orders[0].id,
        "30000000-0000-0000-0000-000000000001"
    );
    assert_eq!(nested[0].orders[0].status, order_search::OrderStatus::Paid);

    let none: Option<OrderRow> = select(order_search::dataset())
        .fields([order_search::ID, order_search::EMAIL])
        .filter(order_search::EMAIL.eq("nobody@example.com"))
        .fetch_optional_as(&client)
        .await?;
    assert!(none.is_none());

    Ok(())
}

#[tokio::test]
async fn executes_extended_operators_against_postgres() -> TestResult {
    let Some(client) = begin_test_transaction().await? else {
        return Ok(());
    };

    let built = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(all([
            order_search::ID.contains("30000000"),
            order_search::TAGS.has("vip"),
            order_search::TAGS.contains_all(["vip", "gift"]),
            order_search::TAGS.elem_match("vip"),
            order_search::TAGS.is_not_empty(),
            order_search::METADATA.key_exists("campaign"),
            order_search::METADATA.keys_exist_any(["score", "missing"]),
            order_search::METADATA.keys_exist_all(["campaign", "score"]),
            order_search::METADATA
                .path("score")
                .is_not_distinct_from(92),
            order_search::EMAIL.regex("^a"),
            order_search::EMAIL.search("ada@example.com"),
            order_search::STATUS.not_in([
                order_search::OrderStatus::Draft,
                order_search::OrderStatus::Cancelled,
            ]),
            order_search::STATUS_HISTORY.contains_any([
                order_search::OrderStatus::Paid,
                order_search::OrderStatus::Refunded,
            ]),
            order_search::TOTAL_CENTS.not_between(1, 10_000),
        ]))
        .order_by(order_search::CREATED_AT.desc().nulls_last())
        .build_pg()?;

    let rows = query(&client, &built.rows).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>("email"), "ada@example.com");
    assert_count(&client, &built.count, 1).await?;
    Ok(())
}

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type TestSetupResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn begin_test_transaction() -> TestSetupResult<Option<TestDb>> {
    let Some(url) = database_url() else {
        eprintln!("skipping Postgres integration test; set RQB_TEST_DATABASE_URL");
        return Ok(None);
    };

    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("postgres connection error: {error}");
        }
    });
    client.batch_execute("BEGIN").await?;
    Ok(Some(TestDb {
        client: Some(client),
    }))
}

struct TestDb {
    client: Option<Client>,
}

impl TestDb {
    fn client(&self) -> &Client {
        self.client
            .as_ref()
            .expect("test database client should stay open until drop")
    }

    async fn batch_execute(&self, sql: &str) -> Result<(), tokio_postgres::Error> {
        self.client().batch_execute(sql).await
    }
}

impl PgExecutor for TestDb {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> rqb_postgres::Result<Vec<Row>> {
        self.client()
            .query(sql, params)
            .await
            .map_err(PgError::from)
    }

    async fn query_one(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> rqb_postgres::Result<Row> {
        self.client()
            .query_opt(sql, params)
            .await
            .map_err(PgError::from)?
            .ok_or(PgError::NotFound)
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> rqb_postgres::Result<Option<Row>> {
        self.client()
            .query_opt(sql, params)
            .await
            .map_err(PgError::from)
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> rqb_postgres::Result<u64> {
        self.client()
            .execute(sql, params)
            .await
            .map_err(PgError::from)
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let Some(client) = self.client.take() else {
            return;
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        handle.spawn(async move {
            let _ = client.batch_execute("ROLLBACK").await;
        });
    }
}

fn database_url() -> Option<String> {
    std::env::var("RQB_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

async fn query(client: &TestDb, built: &BuiltQuery) -> Result<Vec<Row>, tokio_postgres::Error> {
    let params = built.params();
    let refs = params.as_refs();
    client.client().query(&built.sql, &refs).await
}

async fn assert_count(
    client: &TestDb,
    built: &BuiltQuery,
    expected: i64,
) -> Result<(), tokio_postgres::Error> {
    let rows = query(client, built).await?;
    assert_eq!(rows[0].get::<_, i64>(0), expected);
    Ok(())
}
