use rqb_core::{
    Dataset, DbEnum, ElemType, EnumType, Field, FieldType, JsonPathPolicy, SearchRequest, all,
    count, cte, delete, exists, field, insert, not_exists, raw, select, sum, update,
};
use rqb_postgres::{
    BuildPostgres, BuiltQuery, Error as PgError, ExecutePostgres, ExecuteWritePostgres, ResultExt,
};
use serde::{Deserialize, Serialize};

mod order_search {
    use super::*;

    pub const ORDER_STATUS: EnumType = EnumType::new(
        Some("public"),
        "order_status",
        &["draft", "paid", "cancelled", "refunded"],
    );

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OrderStatus {
        Draft,
        Paid,
        Cancelled,
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
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamp);
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
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamp);

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
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamp);

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

#[tokio::test]
async fn executes_view_query_with_jsonb_arrays_projection_and_count() -> TestResult {
    let Some(client) = connect().await? else {
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
async fn executes_raw_cte_as_search_source() -> TestResult {
    let Some(client) = connect().await? else {
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
    let Some(client) = connect().await? else {
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
async fn accepts_json_api_request_and_runs_same_validation_pipeline() -> TestResult {
    let Some(client) = connect().await? else {
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
    let Some(client) = connect().await? else {
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
    let Some(client) = connect().await? else {
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
async fn executor_api_runs_rows_optional_and_count() -> TestResult {
    let Some(client) = connect().await? else {
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

#[cfg(all(feature = "with-uuid", feature = "with-chrono"))]
#[tokio::test]
async fn uuid_chrono_and_page_helpers_are_ergonomic() -> TestResult {
    let Some(client) = connect().await? else {
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
    let Some(client) = connect().await? else {
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

    let deleted: EventRow = delete(events_table::dataset())
        .filter(events_table::ID.eq(event_id))
        .returning([
            events_table::ID,
            events_table::EVENT_TYPE,
            events_table::PAYLOAD,
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
    let order_id = "30000000-0000-0000-0000-000000000001";

    let _ = delete(events_table::dataset())
        .filter(events_table::ID.is_in([committed_id, rolled_back_id]))
        .execute(&db)
        .await;

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

    delete(events_table::dataset())
        .filter(events_table::ID.eq(committed_id))
        .execute(&db)
        .await?;
    Ok(())
}

#[tokio::test]
async fn maps_postgres_execution_errors_and_result_ext() -> TestResult {
    let Some(client) = connect().await? else {
        return Ok(());
    };

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
    assert!(duplicate.is_unique_violation());
    assert!(duplicate.is_constraint("organizations_slug_key"));

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
    assert!(fk.is_foreign_key_violation());

    let not_found = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(order_search::EMAIL.eq("nobody@example.com"))
        .fetch_one(&client)
        .await
        .unwrap_err();
    assert!(not_found.is_not_found());

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

    Ok(())
}

#[tokio::test]
async fn fetch_as_deserializes_fields_json_arrays_and_aggregates() -> TestResult {
    let Some(client) = connect().await? else {
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
        status: String,
        status_history: Vec<String>,
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
    assert_eq!(order.status, "paid");
    assert_eq!(order.status_history, vec!["draft", "paid"]);
    assert_eq!(order.tags, vec!["vip", "gift"]);
    assert_eq!(order.metadata.score, 92);
    assert!(order.metadata.gift);
    assert_eq!(order.metadata.campaign, "spring");
    assert!(order.created_at.starts_with("2026-02-01"));
    assert_eq!(order.total_cents, 15_900);

    #[derive(Debug, Deserialize)]
    struct StatusRollup {
        status: String,
        count: i64,
        total: f64,
    }

    let rollups: Vec<StatusRollup> = select(order_search::dataset())
        .fields([order_search::STATUS])
        .agg(count("count"))
        .agg(sum(order_search::TOTAL_CENTS, "total"))
        .group_by([order_search::STATUS])
        .order_by(order_search::STATUS.asc())
        .fetch_as(&client)
        .await?;
    let paid = rollups
        .iter()
        .find(|rollup| rollup.status == "paid")
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
        status: String,
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
        .fetch_as(&client)
        .await?;
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].email, "ada@example.com");
    assert_eq!(nested[0].orders.len(), 2);
    assert_eq!(
        nested[0].orders[0].id,
        "30000000-0000-0000-0000-000000000001"
    );
    assert_eq!(nested[0].orders[0].status, "paid");

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
    let Some(client) = connect().await? else {
        return Ok(());
    };

    let built = select(order_search::dataset())
        .fields([order_search::EMAIL])
        .filter(all([
            order_search::TAGS.has("vip"),
            order_search::TAGS.is_not_empty(),
            order_search::METADATA.key_exists("campaign"),
            order_search::METADATA.keys_exist_any(["score", "missing"]),
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

async fn connect()
-> Result<Option<tokio_postgres::Client>, Box<dyn std::error::Error + Send + Sync>> {
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
    Ok(Some(client))
}

fn database_url() -> Option<String> {
    std::env::var("RQB_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

async fn query(
    client: &tokio_postgres::Client,
    built: &BuiltQuery,
) -> Result<Vec<tokio_postgres::Row>, tokio_postgres::Error> {
    let params = built.params();
    let refs = params.as_refs();
    client.query(&built.sql, &refs).await
}

async fn assert_count(
    client: &tokio_postgres::Client,
    built: &BuiltQuery,
    expected: i64,
) -> Result<(), tokio_postgres::Error> {
    let rows = query(client, built).await?;
    assert_eq!(rows[0].get::<_, i64>(0), expected);
    Ok(())
}
