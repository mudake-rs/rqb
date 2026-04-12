use super::*;
use pretty_assertions::assert_eq;
use rqb_core::{
    Dataset, ElemType, EnumType, Field, FieldType, JsonPathPolicy, SearchRequest, SelectColumn,
    Sort, Value, all, array_agg, avg, count, count_distinct, delete, exists, field, insert, max,
    min, not_exists, raw, select, string_agg, sum, update,
};
use serde::Serialize;

fn orders() -> Dataset {
    Dataset::view("order_search_view")
        .fields([
            Field::new("id", FieldType::Uuid),
            Field::new("email", FieldType::Text),
            Field::new("status", FieldType::Text),
            Field::new("name", FieldType::Text).text_search("english"),
            Field::mapped("createdAt", "created_at", FieldType::Timestamp),
            Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false),
            Field::mapped("totalCents", "total_cents", FieldType::BigInt),
            Field::new("metadata", FieldType::Jsonb)
                .sortable(false)
                .json_paths(JsonPathPolicy::Dynamic),
        ])
        .max_limit(500)
}

fn orders_table() -> Dataset {
    Dataset::table("orders").fields([
        Field::new("id", FieldType::Uuid),
        Field::mapped("userId", "user_id", FieldType::Uuid),
        Field::new("status", FieldType::Text),
        Field::mapped("createdAt", "created_at", FieldType::Timestamp),
    ])
}

fn users_table() -> Dataset {
    Dataset::table("app_users").fields([
        Field::new("id", FieldType::Uuid),
        Field::new("email", FieldType::Text),
        Field::new("status", FieldType::Text),
    ])
}

fn writable_orders() -> Dataset {
    Dataset::table("orders").fields([
        Field::new("id", FieldType::Uuid),
        Field::mapped("userId", "user_id", FieldType::Uuid),
        Field::new("status", FieldType::Enum(ORDER_STATUS)),
        Field::mapped("totalCents", "total_cents", FieldType::BigInt),
        Field::mapped("createdAt", "created_at", FieldType::Timestamp),
    ])
}

fn typed_values() -> Dataset {
    Dataset::table("typed_values").fields([
        Field::new("id", FieldType::Uuid),
        Field::new("a", FieldType::Integer),
        Field::new("b", FieldType::BigInt),
        Field::new("amount", FieldType::Numeric),
        Field::new("ratio", FieldType::Float),
        Field::new("active", FieldType::Bool),
        Field::mapped("happenedOn", "happened_on", FieldType::Date),
        Field::mapped("createdAt", "created_at", FieldType::Timestamp),
        Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false),
        Field::new("scores", FieldType::Array(ElemType::Int)).sortable(false),
        Field::new("ids", FieldType::Array(ElemType::Uuid)).sortable(false),
        Field::mapped(
            "createdAtList",
            "created_at_list",
            FieldType::Array(ElemType::Timestamp),
        )
        .sortable(false),
    ])
}

const ORDER_STATUS: EnumType = EnumType::new(
    Some("public"),
    "order_status",
    &["draft", "paid", "cancelled", "refunded"],
);

#[cfg(feature = "with-uuid")]
fn uuid_projection(expr: &str, alias: &str, force_alias: bool) -> String {
    if force_alias {
        format!("{expr} AS \"{alias}\"")
    } else {
        expr.to_owned()
    }
}

#[cfg(not(feature = "with-uuid"))]
fn uuid_projection(expr: &str, alias: &str, _force_alias: bool) -> String {
    format!("{expr}::text AS \"{alias}\"")
}

#[cfg(feature = "with-uuid")]
fn uuid_value(expr: &str) -> String {
    expr.to_owned()
}

#[cfg(not(feature = "with-uuid"))]
fn uuid_value(expr: &str) -> String {
    format!("{expr}::text")
}

#[cfg(feature = "with-chrono")]
fn timestamp_projection(expr: &str, alias: &str) -> String {
    format!("{expr} AS \"{alias}\"")
}

#[cfg(not(feature = "with-chrono"))]
fn timestamp_projection(expr: &str, alias: &str) -> String {
    format!("{expr}::text AS \"{alias}\"")
}

#[derive(Clone, Copy)]
enum OrderStatus {
    Draft,
    Paid,
    Cancelled,
    Refunded,
}

impl OrderStatus {
    const fn as_db_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Paid => "paid",
            Self::Cancelled => "cancelled",
            Self::Refunded => "refunded",
        }
    }
}

impl rqb_core::DbEnum for OrderStatus {
    const TYPE: EnumType = ORDER_STATUS;

    fn as_db_str(self) -> &'static str {
        Self::as_db_str(self)
    }
}

fn enum_orders() -> Dataset {
    Dataset::view("order_search_view").fields([
        Field::new("status", FieldType::Enum(ORDER_STATUS)),
        Field::mapped(
            "statusHistory",
            "status_history",
            FieldType::Array(ElemType::Enum(ORDER_STATUS)),
        )
        .sortable(false),
    ])
}

#[test]
fn renders_ergonomic_query() {
    let built = select(orders())
        .fields(["id", "email", "createdAt"])
        .filter(all([
            field("status").eq("paid"),
            field("metadata.score").gte(70),
            field("tags").contains_any(["vip", "gift"]),
        ]))
        .order_by(Sort::desc("createdAt"))
        .limit(20)
        .build_rows_pg()
        .unwrap();

    let expected = format!(
        "SELECT {}, \"email\", {} FROM \"order_search_view\" \
         WHERE (\"status\" = $1 AND (\"metadata\" #>> ARRAY[$2]::text[])::numeric >= $3::text::numeric AND \"tags\" && $4::text[]) \
         ORDER BY \"created_at\" DESC LIMIT 20 OFFSET 0",
        uuid_projection("\"id\"", "id", false),
        timestamp_projection("\"created_at\"", "createdAt")
    );
    assert_eq!(built.sql, expected);
    assert_eq!(built.params.len(), 4);
}

#[test]
fn json_path_numeric_comparison_keeps_i64_precision() {
    let exact = 9_007_199_254_740_993_i64;
    let built = select(orders())
        .filter(field("metadata.score").gte(exact))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("(\"metadata\" #>> ARRAY[$1]::text[])::numeric >= $2::text::numeric"),
        "{}",
        built.sql
    );
    assert_eq!(
        built.params,
        vec![
            Value::String("score".to_owned()),
            Value::String(exact.to_string())
        ]
    );
}

#[test]
fn renders_known_fields_on_unfielded_dataset_without_descriptor() {
    let id = Field::new("id", FieldType::Uuid);
    let total = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
    let status = Field::new("status", FieldType::Text);

    let built = select(Dataset::table("orders_archive"))
        .fields([id, total])
        .filter(status.eq("paid"))
        .order_by(total.desc())
        .limit(5)
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.starts_with("SELECT \"id\""));
    assert!(built.sql.contains("\"total_cents\" AS \"totalCents\""));
    assert!(built.sql.contains("FROM \"orders_archive\""));
    assert!(built.sql.contains("WHERE \"status\" = $1"));
    assert!(built.sql.contains("ORDER BY \"total_cents\" DESC"));
    assert_eq!(built.params, vec!["paid".into()]);
}

#[test]
fn renders_numeric_float_special_values_as_postgres_literals() {
    let amounts = Dataset::table("amounts").fields([Field::new("amount", FieldType::Numeric)]);
    let built = select(amounts)
        .filter(all([
            field("amount").gte(f64::INFINITY),
            field("amount").lte(f64::NEG_INFINITY),
            field("amount").ne(f64::NAN),
        ]))
        .build_rows_pg()
        .unwrap();

    assert_eq!(
        built.params,
        vec![
            Value::String("Infinity".to_owned()),
            Value::String("-Infinity".to_owned()),
            Value::String("NaN".to_owned())
        ]
    );
    assert!(built.sql.contains("$1::text::numeric"), "{}", built.sql);
    assert!(built.sql.contains("$2::text::numeric"), "{}", built.sql);
    assert!(built.sql.contains("$3::text::numeric"), "{}", built.sql);
}

#[test]
fn renders_scalar_range_and_text_negation_operators() {
    let built = select(orders())
        .filter(all([
            field("totalCents").between(1_000, 2_000),
            field("totalCents").lt(5_000),
            field("email").not_contains("@spam.test"),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("\"total_cents\" BETWEEN $1::bigint AND $2::bigint")
    );
    assert!(built.sql.contains("\"total_cents\" < $3::bigint"));
    assert!(built.sql.contains("\"email\" NOT ILIKE $4 ESCAPE '\\'"));
    assert_eq!(
        built.params,
        vec![
            Value::I64(1_000),
            Value::I64(2_000),
            Value::I64(5_000),
            Value::String("%@spam.test%".to_owned())
        ]
    );
}

#[test]
fn renders_array_contains_all_and_elem_match_with_array_casts() {
    let built = select(orders())
        .filter(all([
            field("tags").contains_all(["vip", "gift"]),
            field("tags").elem_match("vip"),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"tags\" @> $1::text[]"));
    assert!(built.sql.contains("\"tags\" @> $2::text[]"));
    assert_eq!(
        built.params,
        vec![
            Value::Array(vec!["vip".into(), "gift".into()]),
            Value::Array(vec!["vip".into()])
        ]
    );
}

#[test]
fn renders_json_elem_match_and_keys_exist_all() {
    let built = select(orders())
        .filter(all([
            field("metadata").elem_match(serde_json::json!({"gift": true})),
            field("metadata").keys_exist_all(["campaign", "score"]),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"metadata\" @> $1::jsonb"));
    assert!(built.sql.contains("\"metadata\" ?& $2::text[]"));
}

#[test]
fn renders_jsonb_and_json_path_null_safe_comparisons_as_jsonb() {
    let built = select(orders())
        .filter(all([
            field("metadata").eq(serde_json::json!({"gift": true})),
            field("metadata.score").is_not_distinct_from(92),
            field("metadata.missing").is_distinct_from(()),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"metadata\" = $1::jsonb"));
    assert!(
        built
            .sql
            .contains("\"metadata\" #> ARRAY[$2]::text[] IS NOT DISTINCT FROM $3::jsonb")
    );
    assert!(
        built
            .sql
            .contains("\"metadata\" #>> ARRAY[$4]::text[] IS DISTINCT FROM $5")
    );
    assert_eq!(built.params[0], serde_json::json!({"gift": true}).into());
    assert_eq!(built.params[2], serde_json::json!(92).into());
    assert_eq!(built.params[4], Value::Null);
}

#[test]
fn renders_jsonb_array_comparisons_as_jsonb_params() {
    let built = select(orders())
        .filter(all([
            field("metadata").eq([1, 2, 3]),
            field("metadata").is_not_distinct_from(["vip", "gift"]),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"metadata\" = $1::jsonb"));
    assert!(
        built
            .sql
            .contains("\"metadata\" IS NOT DISTINCT FROM $2::jsonb")
    );
    assert_eq!(built.params[0], serde_json::json!([1, 2, 3]).into());
    assert_eq!(built.params[1], serde_json::json!(["vip", "gift"]).into());
}

#[test]
fn renders_column_predicate_operator_matrix() {
    let built = select(typed_values())
        .filter(all([
            field("a").ne_col(field("b")),
            field("a").gt_col(field("b")),
            field("a").gte_col(field("b")),
            field("a").lt_col(field("b")),
            field("a").lte_col(field("b")),
        ]))
        .build_rows_pg()
        .unwrap();

    for fragment in [
        "\"a\" <> \"b\"",
        "\"a\" > \"b\"",
        "\"a\" >= \"b\"",
        "\"a\" < \"b\"",
        "\"a\" <= \"b\"",
    ] {
        assert!(
            built.sql.contains(fragment),
            "{fragment} missing from {}",
            built.sql
        );
    }
}

#[test]
fn renders_postgres_type_cast_matrix() {
    let built = select(typed_values())
        .fields(["id", "happenedOn", "active", "ratio"])
        .filter(all([
            field("id").eq("10000000-0000-0000-0000-000000000001"),
            field("a").eq(42),
            field("b").eq(42_i64),
            field("amount").eq("9007199254740993"),
            field("ratio").eq(1.5),
            field("active").eq(true),
            field("happenedOn").eq("2026-04-12"),
            field("createdAt").eq("2026-04-12T00:00:00Z"),
            field("scores").contains_any([1, 2]),
            field("ids").contains_any(["10000000-0000-0000-0000-000000000001"]),
            field("createdAtList").contains_any(["2026-04-12T00:00:00Z"]),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("$1::text::uuid"));
    assert!(built.sql.contains("$2::bigint::int"));
    assert!(built.sql.contains("$3::bigint"));
    assert!(built.sql.contains("$4::text::numeric"));
    assert!(built.sql.contains("$5::double precision"));
    assert!(built.sql.contains("$7::text::date"));
    assert!(built.sql.contains("$8::text::timestamptz"));
    assert!(built.sql.contains("$9::bigint[]::int[]"));
    assert!(built.sql.contains("$10::text[]::uuid[]"));
    assert!(built.sql.contains("$11::text[]::timestamptz[]"));
}

#[test]
fn renders_uuid_text_operators_with_text_casts() {
    let built = select(typed_values())
        .filter(all([
            field("id").contains("10000000"),
            field("id").starts_with("1000"),
            field("id").ends_with("0001"),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"id\"::text ILIKE $1 ESCAPE '\\'"));
    assert!(built.sql.contains("\"id\"::text ILIKE $2 ESCAPE '\\'"));
    assert!(built.sql.contains("\"id\"::text ILIKE $3 ESCAPE '\\'"));
}

#[test]
fn renders_empty_array_params_as_typed_literals() {
    let built = select(typed_values())
        .filter(all([
            field("scores").contains_any(Vec::<i32>::new()),
            field("ids").contains_any(Vec::<String>::new()),
            field("createdAtList").contains_any(Vec::<String>::new()),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"scores\" && ARRAY[]::bigint[]::int[]"));
    assert!(built.sql.contains("\"ids\" && ARRAY[]::text[]::uuid[]"));
    assert!(
        built
            .sql
            .contains("\"created_at_list\" && ARRAY[]::text[]::timestamptz[]")
    );
    assert!(built.params.is_empty());

    let enum_built = select(enum_orders())
        .filter(field("statusHistory").contains_any(Vec::<OrderStatus>::new()))
        .build_rows_pg()
        .unwrap();

    assert!(
        enum_built
            .sql
            .contains("\"status_history\" && ARRAY[]::text[]::\"public\".\"order_status\"[]")
    );
    assert!(enum_built.params.is_empty());
}

#[test]
fn select_defaults_to_all_selectable_root_fields() {
    let built = select(orders())
        .filter(field("status").eq("paid"))
        .build_rows_pg()
        .unwrap();

    let aliases = built
        .columns
        .iter()
        .map(SelectColumn::alias)
        .collect::<Vec<_>>();
    assert_eq!(
        aliases,
        vec![
            "id",
            "email",
            "status",
            "name",
            "createdAt",
            "tags",
            "totalCents",
            "metadata",
        ]
    );
    assert!(built.sql.starts_with("SELECT "));
    assert!(!built.sql.contains("SELECT *"));
    assert!(built.sql.contains(" FROM \"order_search_view\" "));
}

#[test]
fn renders_schema_qualified_sources_and_explicit_offset() {
    let dataset = Dataset::from(rqb_core::Source::table("orders").schema("archive"))
        .fields([Field::new("id", FieldType::Uuid)]);
    let built = select(dataset)
        .fields(["id"])
        .offset(40)
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("FROM \"archive\".\"orders\""));
    assert!(built.sql.ends_with("LIMIT 100 OFFSET 40"));
}

#[test]
fn renders_cte_and_raw_predicate() {
    let cte = rqb_core::cte(
        "recent_orders",
        raw("SELECT * FROM order_search_view WHERE created_at >= ?").bind("2026-01-01T00:00:00Z"),
    );
    let built = select(Dataset::cte("recent_orders").fields(orders().fields))
        .cte(cte)
        .filter(raw("total_cents > ?").bind(10_000))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.starts_with(
        "WITH \"recent_orders\" AS (SELECT * FROM order_search_view WHERE created_at >= $1) SELECT"
    ));
    assert!(built.sql.contains("WHERE total_cents > $2"));
    assert_eq!(built.params.len(), 2);
}

#[test]
fn renders_select_based_and_recursive_ctes() {
    let paid_orders = rqb_core::cte(
        "paid_orders",
        select(orders())
            .fields(["id", "status"])
            .filter(field("status").eq("paid"))
            .build(),
    );
    let built = select(Dataset::cte("paid_orders").fields([
        Field::new("id", FieldType::Uuid),
        Field::new("status", FieldType::Text),
    ]))
    .cte(paid_orders)
    .build_rows_pg()
    .unwrap();

    assert!(built.sql.starts_with("WITH \"paid_orders\" AS (SELECT"));
    assert!(built.sql.contains("FROM \"order_search_view\""));
    assert!(built.sql.contains("WHERE \"status\" = $1"));

    let numbers = rqb_core::cte("numbers", raw("SELECT ?::bigint AS n").bind(1))
        .columns(["n"])
        .recursive();
    let recursive = select(Dataset::cte("numbers").fields([Field::new("n", FieldType::BigInt)]))
        .cte(numbers)
        .build_rows_pg()
        .unwrap();

    assert!(
        recursive
            .sql
            .starts_with("WITH RECURSIVE \"numbers\" (\"n\") AS (SELECT $1::bigint AS n)")
    );
}

#[test]
fn renders_raw_sql_escaped_question_marks_and_bind_errors() {
    let escaped = rqb_core::cte("escaped", raw("SELECT ?? AS literal, ? AS n").bind(1));
    let built = select(Dataset::cte("escaped").fields([Field::new("n", FieldType::Integer)]))
        .cte(escaped)
        .build_rows_pg()
        .unwrap();
    assert!(built.sql.contains("SELECT ? AS literal, $1 AS n"));
    assert_eq!(built.params, vec![Value::I64(1)]);

    let too_few = select(Dataset::cte("bad").fields([Field::new("n", FieldType::Integer)]))
        .cte(rqb_core::cte("bad", raw("SELECT ? AS n")))
        .build_rows_pg()
        .unwrap_err();
    assert!(matches!(too_few, Error::TooFewRawBinds));

    let unused = select(Dataset::cte("bad").fields([Field::new("n", FieldType::Integer)]))
        .cte(rqb_core::cte("bad", raw("SELECT 1 AS n").bind(1)))
        .build_rows_pg()
        .unwrap_err();
    assert!(matches!(unused, Error::UnusedRawBinds));
}

#[test]
fn renders_insert_values_returning_and_upsert() {
    let built = insert(writable_orders())
        .set("id", "30000000-0000-0000-0000-000000009999")
        .set("userId", "10000000-0000-0000-0000-000000000001")
        .set("status", OrderStatus::Paid)
        .set("totalCents", 15_900)
        .on_conflict("id")
        .do_update(["status", "totalCents"])
        .returning(["id", "status"])
        .build_pg()
        .unwrap();

    assert!(built.sql.starts_with(
        "INSERT INTO \"orders\" (\"id\", \"user_id\", \"status\", \"total_cents\") VALUES "
    ));
    assert!(built.sql.contains("$1::text::uuid"));
    assert!(built.sql.contains("$2::text::uuid"));
    assert!(built.sql.contains("$3::text::\"public\".\"order_status\""));
    assert!(built.sql.contains("ON CONFLICT (\"id\") DO UPDATE SET \"status\" = EXCLUDED.\"status\", \"total_cents\" = EXCLUDED.\"total_cents\""));
    let expected_returning = format!(
        "RETURNING {}, \"status\"::text AS \"status\"",
        uuid_projection("\"id\"", "id", false)
    );
    assert!(built.sql.ends_with(&expected_returning));
    assert_eq!(built.params.len(), 4);
    assert_eq!(built.columns.len(), 2);
}

#[test]
fn renders_insert_batch_values() {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NewOrder {
        id: &'static str,
        user_id: &'static str,
        status: &'static str,
        total_cents: i64,
    }

    let rows = [
        NewOrder {
            id: "30000000-0000-0000-0000-000000009901",
            user_id: "10000000-0000-0000-0000-000000000001",
            status: "draft",
            total_cents: 1_000,
        },
        NewOrder {
            id: "30000000-0000-0000-0000-000000009902",
            user_id: "10000000-0000-0000-0000-000000000002",
            status: "paid",
            total_cents: 2_000,
        },
    ];

    let built = insert(writable_orders()).values(&rows).build_pg().unwrap();

    assert!(built.sql.starts_with("INSERT INTO \"orders\""));
    assert!(built.sql.contains(" VALUES ("));
    assert!(built.sql.contains("), ("));
    assert_eq!(built.params.len(), 8);
}

#[test]
fn renders_insert_from_select() {
    let source = select(writable_orders())
        .fields(["id", "userId", "status", "totalCents"])
        .filter(field("status").eq(OrderStatus::Paid))
        .build();

    let built = insert(writable_orders())
        .from_select(source)
        .on_conflict("id")
        .do_nothing()
        .build_pg()
        .unwrap();

    assert!(built.sql.starts_with(
        "INSERT INTO \"orders\" (\"id\", \"user_id\", \"status\", \"total_cents\") SELECT"
    ));
    assert!(built.sql.contains("FROM \"orders\""));
    assert!(
        built
            .sql
            .contains("WHERE \"status\" = $1::text::\"public\".\"order_status\"")
    );
    assert!(built.sql.ends_with("ON CONFLICT (\"id\") DO NOTHING"));
}

#[test]
fn renders_conflict_targets_actions_and_filters() {
    let do_nothing = insert(writable_orders())
        .set("id", "30000000-0000-0000-0000-000000009999")
        .on_conflict("id")
        .do_nothing()
        .build_pg()
        .unwrap();
    assert!(do_nothing.sql.contains("ON CONFLICT (\"id\") DO NOTHING"));

    let constraint = insert(writable_orders())
        .set("id", "30000000-0000-0000-0000-000000009999")
        .on_conflict_constraint("orders_pkey")
        .do_nothing()
        .build_pg()
        .unwrap();
    assert!(
        constraint
            .sql
            .contains("ON CONFLICT ON CONSTRAINT \"orders_pkey\" DO NOTHING")
    );

    let composite = insert(writable_orders())
        .set("id", "30000000-0000-0000-0000-000000009999")
        .set("userId", "10000000-0000-0000-0000-000000000001")
        .set("status", OrderStatus::Paid)
        .on_conflict(["id", "userId"])
        .do_update(["status"])
        .filter(field("status").ne(OrderStatus::Cancelled))
        .build_pg()
        .unwrap();
    assert!(composite
        .sql
        .contains("ON CONFLICT (\"id\", \"user_id\") DO UPDATE SET \"status\" = EXCLUDED.\"status\" WHERE \"status\" <> $4::text::\"public\".\"order_status\""));
}

#[test]
fn renders_update_raw_column_and_delete() {
    let update = update(writable_orders())
        .set("status", OrderStatus::Refunded)
        .set_raw("totalCents", raw("total_cents + ?").bind(100))
        .set_col("createdAt", "createdAt")
        .filter(field("id").eq("30000000-0000-0000-0000-000000009999"))
        .returning(["id", "status", "totalCents"])
        .build_pg()
        .unwrap();

    assert!(update.sql.starts_with(
            "UPDATE \"orders\" SET \"status\" = $1::text::\"public\".\"order_status\", \"total_cents\" = total_cents + $2, \"created_at\" = \"created_at\""
        ));
    assert!(
        update
            .sql
            .contains("WHERE \"id\" = $3::text::uuid RETURNING")
    );

    let delete = delete(writable_orders())
        .filter(field("id").eq("30000000-0000-0000-0000-000000009999"))
        .returning(["id"])
        .build_pg()
        .unwrap();

    let expected_delete = format!(
        "DELETE FROM \"orders\" WHERE \"id\" = $1::text::uuid RETURNING {}",
        uuid_projection("\"id\"", "id", false)
    );
    assert_eq!(delete.sql, expected_delete);
}

#[test]
fn renders_update_set_from_serde_record() {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct OrderPatch {
        status: &'static str,
        total_cents: i64,
    }

    let built = update(writable_orders())
        .set_from(&OrderPatch {
            status: "paid",
            total_cents: 15_900,
        })
        .filter(field("id").eq("30000000-0000-0000-0000-000000009999"))
        .build_pg()
        .unwrap();

    assert!(built.sql.starts_with("UPDATE \"orders\" SET "));
    assert!(
        built
            .sql
            .contains("\"status\" = $1::text::\"public\".\"order_status\"")
    );
    assert!(built.sql.contains("\"total_cents\" = $2::bigint"));
    assert!(built.sql.contains("WHERE \"id\" = $3::text::uuid"));
}

#[test]
fn renders_jsonb_write_arrays_as_jsonb_params() {
    let dataset = Dataset::table("events").fields([Field::new("payload", FieldType::Jsonb)]);

    let built = update(dataset)
        .set("payload", [1, 2, 3])
        .build_pg()
        .unwrap();

    assert_eq!(built.sql, "UPDATE \"events\" SET \"payload\" = $1::jsonb");
    assert_eq!(built.params, vec![serde_json::json!([1, 2, 3]).into()]);
}

#[test]
fn renders_jsonb_write_null_as_sql_null() {
    let dataset = Dataset::table("events").fields([Field::new("payload", FieldType::Jsonb)]);

    let built = update(dataset).set_null("payload").build_pg().unwrap();

    assert_eq!(built.sql, "UPDATE \"events\" SET \"payload\" = $1::jsonb");
    assert_eq!(built.params, vec![Value::Null]);
}

#[test]
fn renders_default_returning_all_for_write_fetches() {
    let built = insert(writable_orders())
        .set("id", "30000000-0000-0000-0000-000000009999")
        .set("userId", "10000000-0000-0000-0000-000000000001")
        .set("status", OrderStatus::Paid)
        .set("totalCents", 15_900)
        .returning_all_if_empty()
        .build_pg()
        .unwrap();

    let expected_returning = format!(
        "RETURNING {}, {}, \"status\"::text AS \"status\", \"total_cents\" AS \"totalCents\", {}",
        uuid_projection("\"id\"", "id", false),
        uuid_projection("\"user_id\"", "userId", true),
        timestamp_projection("\"created_at\"", "createdAt"),
    );
    assert!(built.sql.ends_with(&expected_returning));
    assert_eq!(
        built
            .columns
            .iter()
            .map(SelectColumn::alias)
            .collect::<Vec<_>>(),
        vec!["id", "userId", "status", "totalCents", "createdAt"]
    );

    let explicit = update(writable_orders())
        .set("status", OrderStatus::Refunded)
        .returning(["id"])
        .returning_all_if_empty()
        .build_pg()
        .unwrap();
    assert_eq!(explicit.columns.len(), 1);

    let update_all = update(writable_orders())
        .set("status", OrderStatus::Paid)
        .filter(field("id").eq("30000000-0000-0000-0000-000000009999"))
        .returning_all_if_empty()
        .build_pg()
        .unwrap();
    assert_eq!(update_all.columns.len(), 5);

    let delete_all = delete(writable_orders())
        .filter(field("id").eq("30000000-0000-0000-0000-000000009999"))
        .returning_all_if_empty()
        .build_pg()
        .unwrap();
    assert_eq!(delete_all.columns.len(), 5);
}

#[test]
fn renders_set_null_shortcut() {
    let built = update(writable_orders())
        .set_null("createdAt")
        .filter(field("id").eq("30000000-0000-0000-0000-000000009999"))
        .returning(["id"])
        .build_pg()
        .unwrap();

    assert!(built.sql.contains("\"created_at\" = $1::text::timestamptz"));
    assert_eq!(
        built.params,
        vec![Value::Null, "30000000-0000-0000-0000-000000009999".into()]
    );
}

#[test]
fn renders_group_by_and_aggregates() {
    let built = select(orders())
        .fields(["status"])
        .agg(count("count"))
        .agg(sum("totalCents", "total"))
        .agg(avg("totalCents", "average"))
        .agg(count_distinct("email", "uniqueEmails"))
        .agg(string_agg("email", ",", "emails"))
        .group_by(["status"])
        .having(raw("COUNT(*) > ?").bind(1))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.starts_with("SELECT \"status\", COUNT(*) AS \"count\", SUM(\"total_cents\")::double precision AS \"total\", AVG(\"total_cents\")::double precision AS \"average\", COUNT(DISTINCT \"email\") AS \"uniqueEmails\", string_agg(\"email\", ',') AS \"emails\" FROM \"order_search_view\""));
    assert!(
        built
            .sql
            .contains(" GROUP BY \"status\" HAVING COUNT(*) > $1 ")
    );
    assert_eq!(built.columns.len(), 6);
}

#[test]
fn renders_filter_for_non_json_aggregates() {
    let built = select(orders())
        .agg(count("paidCount").filter(field("status").eq("paid")))
        .agg(sum("totalCents", "paidTotal"))
        .filter_agg("paidTotal", field("status").eq("paid"))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("COUNT(*) FILTER (WHERE \"status\" = $1) AS \"paidCount\""),
        "{}",
        built.sql
    );
    assert!(
        built.sql.contains("(SUM(\"total_cents\") FILTER (WHERE \"status\" = $2))::double precision AS \"paidTotal\""),
        "{}",
        built.sql
    );
    assert_eq!(built.params, vec!["paid".into(), "paid".into()]);
}

#[test]
fn renders_min_max_array_and_string_aggregate_options() {
    let built = select(orders())
        .fields(["status"])
        .agg(min("createdAt", "firstSeen").filter(field("status").eq("paid")))
        .agg(max("createdAt", "lastSeen"))
        .agg(
            array_agg("email", "emails")
                .distinct()
                .order_by(field("email").asc().nulls_last())
                .filter(field("status").eq("paid")),
        )
        .agg(
            string_agg("email", ";", "emailList")
                .order_by(field("createdAt").desc().nulls_first())
                .filter(field("status").eq("paid")),
        )
        .group_by(["status"])
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("MIN(\"created_at\") FILTER (WHERE \"status\" = $1)")
    );
    assert!(built.sql.contains("MAX(\"created_at\")"));
    assert!(built.sql.contains(" AS \"lastSeen\""));
    assert!(built.sql.contains(
        "to_jsonb(array_agg(DISTINCT \"email\" ORDER BY \"email\" ASC NULLS LAST) FILTER (WHERE \"status\" = $2)) AS \"emails\""
    ));
    assert!(built.sql.contains(
        "string_agg(\"email\", ';' ORDER BY \"created_at\" DESC NULLS FIRST) FILTER (WHERE \"status\" = $3) AS \"emailList\""
    ));
}

#[test]
fn renders_distinct_select() {
    let built = select(orders())
        .fields(["email"])
        .distinct()
        .build_pg()
        .unwrap();

    assert!(built.rows.sql.starts_with("SELECT DISTINCT \"email\""));
    assert!(
        built.count.sql.starts_with(
            "SELECT count(*) FROM (SELECT DISTINCT \"email\" FROM \"order_search_view\""
        )
    );
}

#[test]
fn renders_distinct_on() {
    let built = select(orders())
        .fields(["email", "createdAt"])
        .distinct_on(["email"])
        .order_by(field("email").asc())
        .order_by(field("createdAt").desc())
        .build_pg()
        .unwrap();

    let expected_start = format!(
        "SELECT DISTINCT ON (\"email\") \"email\", {} FROM \"order_search_view\"",
        timestamp_projection("\"created_at\"", "createdAt")
    );
    assert!(
        built.rows.sql.starts_with(&expected_start),
        "{}",
        built.rows.sql
    );
    assert!(
        built
            .rows
            .sql
            .contains("ORDER BY \"email\" ASC, \"created_at\" DESC LIMIT 100 OFFSET 0")
    );
    let expected_count_start = format!(
        "SELECT count(*) FROM (SELECT DISTINCT ON (\"email\") \"email\", {} FROM \"order_search_view\"",
        timestamp_projection("\"created_at\"", "createdAt")
    );
    assert!(built.count.sql.starts_with(&expected_count_start));
}

#[test]
fn renders_row_locking() {
    let built = select(orders_table())
        .fields(["id"])
        .filter(field("status").eq("draft"))
        .order_by(field("createdAt").asc())
        .limit(10)
        .for_update()
        .skip_locked()
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.ends_with(
        "WHERE \"status\" = $1 ORDER BY \"created_at\" ASC LIMIT 10 OFFSET 0 FOR UPDATE SKIP LOCKED"
    ));
}

#[test]
fn renders_for_share_nowait() {
    let built = select(orders_table())
        .fields(["id"])
        .for_share()
        .nowait()
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.ends_with("LIMIT 100 OFFSET 0 FOR SHARE NOWAIT"));
}

#[test]
fn renders_other_lock_modes_and_nulls_first() {
    let no_key_update = select(orders_table())
        .fields(["id"])
        .for_no_key_update()
        .skip_locked()
        .build_rows_pg()
        .unwrap();
    assert!(no_key_update.sql.ends_with("FOR NO KEY UPDATE SKIP LOCKED"));

    let key_share = select(orders_table())
        .fields(["id"])
        .for_key_share()
        .build_rows_pg()
        .unwrap();
    assert!(key_share.sql.ends_with("FOR KEY SHARE"));

    let nulls_first = select(orders())
        .order_by(field("createdAt").asc().nulls_first())
        .build_rows_pg()
        .unwrap();
    assert!(
        nulls_first
            .sql
            .contains("ORDER BY \"created_at\" ASC NULLS FIRST")
    );
}

#[test]
fn request_merges_with_existing_filter() {
    let request = SearchRequest {
        query: Some(field("status").eq("paid")),
        limit: Some(5),
        ..SearchRequest::new()
    };

    let built = select(orders())
        .filter(field("totalCents").gt(1_000))
        .request(request)
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("WHERE (\"total_cents\" > $1::bigint AND \"status\" = $2) LIMIT 5 OFFSET 0"),
        "{}",
        built.sql
    );
    assert_eq!(built.params, vec![1_000.into(), "paid".into()]);
}

#[test]
fn filter_chains_with_and() {
    let built = select(orders())
        .filter(field("status").eq("paid"))
        .filter(field("totalCents").gt(1_000))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("WHERE (\"status\" = $1 AND \"total_cents\" > $2::bigint)"),
        "{}",
        built.sql
    );
    assert_eq!(built.params, vec!["paid".into(), 1_000.into()]);
}

#[test]
fn renders_optional_conditional_and_apply_filters() {
    let built = select(orders())
        .filter_if(false, field("status").eq("draft"))
        .filter_if(true, field("status").eq("paid"))
        .filter_option(Some(1_000), |min| field("totalCents").gte(min))
        .filter_option(None::<String>, |status| field("status").eq(status))
        .apply(|query| query.or_where(field("email").ends_with("@example.com")))
        .build_rows_pg()
        .unwrap();

    assert!(
        built.sql.contains(
            "WHERE ((\"status\" = $1 AND \"total_cents\" >= $2::bigint) OR \"email\" ILIKE $3 ESCAPE '\\')"
        ),
        "{}",
        built.sql
    );
    assert_eq!(
        built.params,
        vec![
            Value::String("paid".to_owned()),
            Value::I64(1_000),
            Value::String("%@example.com".to_owned())
        ]
    );
}

#[test]
fn renders_not_expression_and_standalone_null_checks() {
    let built = select(orders())
        .filter(rqb_core::not(all([
            field("status").eq("cancelled"),
            field("createdAt").is_not_null(),
        ])))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("WHERE NOT ((\"status\" = $1 AND \"created_at\" IS NOT NULL))"),
        "{}",
        built.sql
    );
}

#[test]
fn replace_filter_keeps_explicit_replace_semantics() {
    let built = select(orders())
        .filter(field("status").eq("paid"))
        .replace_filter(field("totalCents").gt(1_000))
        .build_rows_pg()
        .unwrap();

    assert!(
        built.sql.contains("WHERE \"total_cents\" > $1::bigint"),
        "{}",
        built.sql
    );
    assert!(!built.sql.contains("\"status\" ="), "{}", built.sql);
    assert_eq!(built.params, vec![1_000.into()]);
}

#[test]
fn replace_request_keeps_explicit_replace_semantics() {
    let request = SearchRequest {
        query: Some(field("status").eq("paid")),
        ..SearchRequest::new()
    };

    let built = select(orders())
        .filter(field("totalCents").gt(1_000))
        .replace_request(request)
        .build_rows_pg()
        .unwrap();

    assert!(
        !built.sql.contains("WHERE \"total_cents\""),
        "{}",
        built.sql
    );
    assert!(built.sql.contains("WHERE \"status\" = $1"), "{}", built.sql);
    assert_eq!(built.params, vec!["paid".into()]);
}

#[test]
fn renders_debug_sql_with_params() {
    let built = select(orders())
        .fields(["email"])
        .filter(field("status").eq("paid"))
        .build_rows_pg()
        .unwrap();

    let debug = built.debug_sql().to_string();
    assert!(debug.starts_with("SELECT \"email\" FROM \"order_search_view\" WHERE \"status\" = $1"));
    assert!(debug.contains("-- params: [String(\"paid\")]"));
}

#[test]
fn renders_debug_select_sql_with_rows_and_count() {
    let built = select(orders())
        .fields(["email"])
        .filter(field("status").eq("paid"))
        .build_pg()
        .unwrap();

    let debug = built.debug_sql().to_string();
    assert!(debug.contains("-- rows"));
    assert!(debug.contains("SELECT \"email\" FROM \"order_search_view\" WHERE \"status\" = $1"));
    assert!(debug.contains("-- count"));
    assert!(debug.contains("SELECT count(*) FROM \"order_search_view\" WHERE \"status\" = $1"));
    assert!(debug.contains("-- params: [String(\"paid\")]"));
}

#[cfg(feature = "runtime-tokio-postgres")]
#[test]
fn page_map_preserves_pagination_metadata() {
    let page = Page {
        items: vec![1, 2, 3],
        total: 30,
        limit: 3,
        offset: 6,
    };

    let mapped = page.map(|value| format!("item-{value}"));

    assert_eq!(mapped.items, vec!["item-1", "item-2", "item-3"]);
    assert_eq!(mapped.total, 30);
    assert_eq!(mapped.limit, 3);
    assert_eq!(mapped.offset, 6);
}

#[cfg(feature = "runtime-tokio-postgres")]
#[test]
fn postgres_error_helpers_classify_variants() {
    let core = Error::Core(rqb_core::Error::UnknownField {
        dataset: "orders".to_owned(),
        field: "missing".to_owned(),
    });
    assert!(core.is_core());
    assert_eq!(
        core.as_core(),
        Some(&rqb_core::Error::UnknownField {
            dataset: "orders".to_owned(),
            field: "missing".to_owned(),
        })
    );

    let unique = Error::UniqueViolation {
        constraint: Some("users_email_key".to_owned()),
        detail: Some("duplicate key".to_owned()),
    };
    assert!(unique.is_unique_violation());
    assert!(unique.is_constraint("users_email_key"));
    assert_eq!(unique.constraint_name(), Some("users_email_key"));
    assert_eq!(unique.detail(), Some("duplicate key"));

    assert!(
        Error::ForeignKeyViolation {
            constraint: None,
            detail: None,
        }
        .is_foreign_key_violation()
    );
    assert!(
        Error::NotNullViolation {
            column: Some("email".to_owned()),
        }
        .is_not_null_violation()
    );
    assert!(
        Error::CheckViolation {
            constraint: Some("orders_total_positive".to_owned()),
        }
        .is_check_violation()
    );
    assert!(Error::Connection("closed".to_owned()).is_connection());
}

#[cfg(feature = "runtime-tokio-postgres")]
#[test]
fn result_ext_maps_conflicts_and_preserves_other_errors() {
    #[derive(Debug, PartialEq)]
    enum AppError {
        Conflict,
        Db,
    }

    impl From<Error> for AppError {
        fn from(_: Error) -> Self {
            Self::Db
        }
    }

    let ok = Ok::<_, Error>(7)
        .on_conflict(|_| AppError::Conflict)
        .unwrap();
    assert_eq!(ok, 7);

    let conflict = Err::<(), _>(Error::UniqueViolation {
        constraint: Some("users_email_key".to_owned()),
        detail: None,
    })
    .on_constraint("users_email_key", |_| AppError::Conflict)
    .unwrap_err();
    assert_eq!(conflict, AppError::Conflict);

    let passthrough = Err::<(), _>(Error::CheckViolation {
        constraint: Some("orders_total_positive".to_owned()),
    })
    .on_conflict(|_| AppError::Conflict)
    .unwrap_err();
    assert_eq!(passthrough, AppError::Db);

    let optional = Err::<(), _>(Error::NotFound).optional().unwrap();
    assert_eq!(optional, None);
}

#[test]
fn renders_json_agg_with_order_and_filter() {
    let built = select(orders_table().alias("o"))
        .join(
            users_table().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .fields([field("u.email")])
        .json_agg("orders", [field("o.id"), field("o.status")])
        .order_within("orders", Sort::desc("o.createdAt"))
        .filter_agg("orders", field("o.status").eq("paid"))
        .group_by([field("u.email")])
        .build_rows_pg()
        .unwrap();

    let expected_json_agg = format!(
        "COALESCE(jsonb_agg(jsonb_build_object('id', {}, 'status', \"o\".\"status\") ORDER BY \"o\".\"created_at\" DESC) FILTER (WHERE \"o\".\"status\" = $1), '[]'::jsonb) AS \"orders\"",
        uuid_value("\"o\".\"id\"")
    );
    assert!(built.sql.contains(&expected_json_agg));
    assert!(built.sql.contains("GROUP BY \"u\".\"email\""));
}

#[test]
fn renders_json_agg_default_empty_auto_group_by_and_root_aliases() {
    let built = select(users_table().alias("u"))
        .left_join(
            orders_table().alias("o"),
            field("u.id").eq_col(field("o.userId")),
        )
        .fields([field("u.id"), field("u.email")])
        .json_agg("orders", [field("o.id"), field("o.status")])
        .filter_agg("orders", field("o.id").is_not_null())
        .build_rows_pg()
        .unwrap();

    let expected_select = format!(
        "SELECT {}, \"u\".\"email\" AS \"email\", COALESCE(jsonb_agg(jsonb_build_object('id', {}, 'status', \"o\".\"status\")) FILTER (WHERE \"o\".\"id\" IS NOT NULL), '[]'::jsonb) AS \"orders\"",
        uuid_projection("\"u\".\"id\"", "id", true),
        uuid_value("\"o\".\"id\"")
    );
    assert!(built.sql.contains(&expected_select));
    assert!(built.sql.contains("GROUP BY \"u\".\"id\", \"u\".\"email\""));
}

#[test]
fn renders_json_agg_nullable_without_default_empty() {
    let built = select(orders_table().alias("o"))
        .json_agg_nullable("orders", [field("o.id")])
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("jsonb_agg(jsonb_build_object("));
    assert!(!built.sql.contains("COALESCE("));
}

#[test]
fn escapes_like_wildcards() {
    let built = select(orders())
        .filter(field("email").contains("a_b%c\\d"))
        .build_rows_pg()
        .unwrap();
    assert_eq!(
        built.params[0],
        Value::String("%a\\_b\\%c\\\\d%".to_owned())
    );
}

#[test]
fn renders_join_with_qualified_columns() {
    let built = select(orders_table().alias("o"))
        .join(
            users_table().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .fields([field("o.id"), field("u.email")])
        .filter(field("u.email").contains("@example.com"))
        .order_by(Sort::desc("o.createdAt"))
        .limit(10)
        .build_rows_pg()
        .unwrap();

    let expected = format!(
        "SELECT {}, \"u\".\"email\" AS \"u_email\" \
         FROM \"orders\" AS \"o\" JOIN \"app_users\" AS \"u\" ON \"o\".\"user_id\" = \"u\".\"id\" \
         WHERE \"u\".\"email\" ILIKE $1 ESCAPE '\\' \
         ORDER BY \"o\".\"created_at\" DESC LIMIT 10 OFFSET 0",
        uuid_projection("\"o\".\"id\"", "id", true)
    );
    assert_eq!(built.sql, expected);
    assert_eq!(
        built.params,
        vec![Value::String("%@example.com%".to_owned())]
    );
}

#[test]
fn renders_right_full_and_cross_joins() {
    let right = select(orders_table().alias("o"))
        .right_join(
            users_table().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .fields([field("o.id")])
        .build_rows_pg()
        .unwrap();
    assert!(right.sql.contains("RIGHT JOIN \"app_users\" AS \"u\" ON"));

    let full = select(orders_table().alias("o"))
        .full_join(
            users_table().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .fields([field("o.id")])
        .build_rows_pg()
        .unwrap();
    assert!(full.sql.contains("FULL JOIN \"app_users\" AS \"u\" ON"));

    let cross = select(orders_table().alias("o"))
        .cross_join(users_table().alias("u"))
        .fields([field("o.id"), field("u.email")])
        .build_rows_pg()
        .unwrap();
    assert!(cross.sql.contains("CROSS JOIN \"app_users\" AS \"u\""));
    assert!(!cross.sql.contains(" CROSS JOIN \"app_users\" AS \"u\" ON "));
}

#[test]
fn join_default_projection_uses_root_fields_only() {
    let built = select(users_table().alias("u"))
        .left_join(
            orders_table().alias("o"),
            field("u.id").eq_col(field("o.userId")),
        )
        .filter(field("o.status").eq("paid"))
        .build_rows_pg()
        .unwrap();

    let aliases = built
        .columns
        .iter()
        .map(SelectColumn::alias)
        .collect::<Vec<_>>();
    assert_eq!(aliases, vec!["id", "email", "status"]);

    let expected_select = format!(
        "SELECT {}, \"u\".\"email\" AS \"email\", \"u\".\"status\" AS \"status\" FROM",
        uuid_projection("\"u\".\"id\"", "id", true)
    );
    assert!(built.sql.contains(&expected_select));
    assert!(built.sql.contains("LEFT JOIN \"orders\" AS \"o\""));
    assert!(built.sql.contains("WHERE \"o\".\"status\" = $1"));
}

#[test]
fn renders_correlated_exists_subquery() {
    let order = orders_table().alias("o");
    let event = Dataset::table("events").alias("e").fields([
        Field::new("id", FieldType::Uuid),
        Field::mapped("orderId", "order_id", FieldType::Uuid),
        Field::mapped("eventType", "event_type", FieldType::Text),
    ]);

    let built = select(order)
        .fields([field("o.id")])
        .filter(exists(
            select(event).filter(
                field("e.orderId")
                    .eq_col(field("o.id"))
                    .and(field("e.eventType").eq("paid")),
            ),
        ))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains(
        "WHERE EXISTS (SELECT 1 FROM \"events\" AS \"e\" WHERE (\"e\".\"order_id\" = \"o\".\"id\" AND \"e\".\"event_type\" = $1))"
    ));
    assert_eq!(built.params, vec!["paid".into()]);
}

#[test]
fn renders_not_exists_subquery() {
    let built = select(orders_table().alias("o"))
        .fields([field("o.id")])
        .filter(not_exists(
            select(Dataset::table("events").alias("e").fields([
                Field::new("id", FieldType::Uuid),
                Field::mapped("orderId", "order_id", FieldType::Uuid),
            ]))
            .filter(field("e.orderId").eq_col(field("o.id"))),
        ))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains(
        "WHERE NOT EXISTS (SELECT 1 FROM \"events\" AS \"e\" WHERE \"e\".\"order_id\" = \"o\".\"id\")"
    ));
}

#[test]
fn renders_in_subquery_without_default_limit() {
    let subquery = select(orders_table().alias("o"))
        .fields([field("o.userId")])
        .filter(field("o.status").eq("paid"));

    let built = select(users_table())
        .fields(["email"])
        .filter(field("id").in_subquery(subquery))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains(
        "WHERE \"id\" IN (SELECT \"o\".\"user_id\" FROM \"orders\" AS \"o\" WHERE \"o\".\"status\" = $1)"
    ), "{}", built.sql);
    assert!(!built.sql.contains("WHERE \"o\".\"status\" = $1 LIMIT"));
    assert_eq!(built.params, vec!["paid".into()]);
}

#[test]
fn renders_not_in_subquery_without_default_limit() {
    let subquery = select(orders_table().alias("o"))
        .fields([field("o.userId")])
        .filter(field("o.status").eq("paid"));

    let built = select(users_table())
        .fields(["email"])
        .filter(field("id").not_in_subquery(subquery))
        .build_rows_pg()
        .unwrap();

    assert!(
        built.sql.contains(
            "WHERE \"id\" NOT IN (SELECT \"o\".\"user_id\" FROM \"orders\" AS \"o\" WHERE \"o\".\"status\" = $1)"
        ),
        "{}",
        built.sql
    );
    assert!(!built.sql.contains("WHERE \"o\".\"status\" = $1 LIMIT"));
    assert_eq!(built.params, vec!["paid".into()]);
}

#[test]
fn rejects_in_subquery_with_more_than_one_column() {
    let query = select(users_table())
        .filter(field("id").in_subquery(
            select(orders_table().alias("o")).fields([field("o.id"), field("o.userId")]),
        ))
        .build();

    let err = query.build_rows_pg().unwrap_err();
    assert!(matches!(
        err,
        Error::Core(rqb_core::Error::InvalidSubquerySelection {
            expected: 1,
            actual: 2,
        })
    ));
}

#[test]
fn renders_not_in() {
    let built = select(orders())
        .filter(field("status").not_in(["draft", "cancelled"]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE NOT (\"status\" IN ($1, $2))"));
    assert_eq!(built.params.len(), 2);

    let empty = select(orders())
        .filter(field("status").not_in(Vec::<String>::new()))
        .build_rows_pg()
        .unwrap();
    assert!(empty.sql.contains("WHERE TRUE "));
    assert!(empty.params.is_empty());
}

#[test]
fn renders_enum_casts() {
    let built = select(enum_orders())
        .fields([field("status"), field("statusHistory")])
        .filter(all([
            field("status").eq(OrderStatus::Paid),
            field("status").not_in([OrderStatus::Draft, OrderStatus::Cancelled]),
            field("statusHistory").has(OrderStatus::Paid),
            field("statusHistory").contains_any([OrderStatus::Paid, OrderStatus::Refunded]),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.starts_with(
        "SELECT \"status\"::text AS \"status\", \"status_history\"::text[] AS \"statusHistory\""
    ));
    assert!(
        built
            .sql
            .contains("\"status\" = $1::text::\"public\".\"order_status\"")
    );
    assert!(built.sql.contains(
            "NOT (\"status\" IN ($2::text::\"public\".\"order_status\", $3::text::\"public\".\"order_status\"))"
        ));
    assert!(
        built
            .sql
            .contains("$4::text::\"public\".\"order_status\" = ANY(\"status_history\")")
    );
    assert!(
        built
            .sql
            .contains("\"status_history\" && $5::text[]::\"public\".\"order_status\"[]")
    );
    assert_eq!(built.params.len(), 5);
}

#[test]
fn renders_string_agg_enum_with_text_cast() {
    let built = select(enum_orders())
        .agg(string_agg("status", ",", "statuses"))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("string_agg(\"status\"::text, ',') AS \"statuses\"")
    );
}

#[test]
fn renders_not_between() {
    let built = select(orders())
        .filter(field("totalCents").not_between(1_000, 2_000))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("\"total_cents\" NOT BETWEEN $1::bigint AND $2::bigint")
    );
}

#[test]
fn renders_not_starts_with_and_not_ends_with() {
    let built = select(orders())
        .filter(all([
            field("email").not_starts_with("ada"),
            field("email").not_ends_with("@example.com"),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"email\" NOT ILIKE $1 ESCAPE '\\'"));
    assert!(built.sql.contains("\"email\" NOT ILIKE $2 ESCAPE '\\'"));
    assert_eq!(built.params[0], Value::String("ada%".to_owned()));
    assert_eq!(built.params[1], Value::String("%@example.com".to_owned()));
}

#[test]
fn renders_is_distinct_from() {
    let built = select(orders())
        .filter(all([
            field("status").is_distinct_from("paid"),
            field("metadata.campaign").is_not_distinct_from("spring"),
        ]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("\"status\" IS DISTINCT FROM $1"));
    assert!(
        built
            .sql
            .contains("\"metadata\" #> ARRAY[$2]::text[] IS NOT DISTINCT FROM $3::jsonb")
    );
}

#[test]
fn renders_nulls_order() {
    let built = select(orders())
        .order_by(field("createdAt").desc().nulls_last())
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("ORDER BY \"created_at\" DESC NULLS LAST")
    );
}

#[test]
fn renders_array_contains_scalar() {
    let built = select(orders())
        .filter(field("tags").has("vip"))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE $1 = ANY(\"tags\")"));
    assert_eq!(built.params, vec![Value::String("vip".to_owned())]);
}

#[test]
fn renders_array_not_contains() {
    let built = select(orders())
        .filter(field("tags").not_has("vip"))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE NOT ($1 = ANY(\"tags\"))"));
}

#[test]
fn renders_array_is_empty() {
    let built = select(orders())
        .filter(field("tags").is_empty())
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE cardinality(\"tags\") = 0"));
}

#[test]
fn renders_json_key_exists() {
    let built = select(orders())
        .filter(field("metadata").key_exists("campaign"))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE \"metadata\" ? $1"));
    assert_eq!(built.params, vec![Value::String("campaign".to_owned())]);
}

#[test]
fn renders_json_keys_exist_any() {
    let built = select(orders())
        .filter(field("metadata").keys_exist_any(["campaign", "score"]))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE \"metadata\" ?| $1::text[]"));
}

#[test]
fn renders_regex() {
    let built = select(orders())
        .filter(field("name").regex("cam.*"))
        .build_rows_pg()
        .unwrap();

    assert!(built.sql.contains("WHERE \"name\" ~* $1"));
}

#[test]
fn renders_not_regex_on_json_path() {
    let built = select(orders())
        .filter(field("metadata.campaign").not_regex("spr.*"))
        .build_rows_pg()
        .unwrap();

    assert!(
        built
            .sql
            .contains("WHERE \"metadata\" #>> ARRAY[$1]::text[] !~* $2")
    );
}

#[test]
fn renders_text_search() {
    let built = select(orders())
        .filter(field("name").search("camera bag"))
        .build_rows_pg()
        .unwrap();

    assert!(
        built.sql.contains(
            "WHERE to_tsvector('english', \"name\") @@ websearch_to_tsquery('english', $1)"
        )
    );
    assert_eq!(built.params, vec![Value::String("camera bag".to_owned())]);
}
