use crate::{
    Assignment, AssignmentValue, BoolExpr, Field, Insert, InsertBody, IntoColumns, Meta, OpSet,
    Param, RawStmt, Select, SelectItem, Source, Stmt, ValueExpr, and, array, array_agg, bool_and,
    case, coalesce, count_all, count_distinct, cte, current_date, current_timestamp, delete_from,
    extract, function_source, insert, json_agg, json_get_text, lag, merge_into, param,
    percentile_cont, raw, raw_expr, raw_predicate, row, row_number, scalar_subquery, select, slice,
    subscript, table, to_jsonb, true_, update, values_source, window,
};

static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static EMAIL_META: Meta = Meta::new("email", "email_address", "text").ops(OpSet::text());
static ACTIVE_META: Meta = Meta::new("active", "active", "bool").ops(OpSet::equality());
static UUID_META: Meta = Meta::new("id", "id", "uuid").ops(OpSet::equality());
static ORDER_USER_ID_META: Meta = Meta::new("user_id", "user_id", "int4").ops(OpSet::ordered());
static TOTAL_META: Meta = Meta::new("total_cents", "total_cents", "int8").ops(OpSet::ordered());
static TAGS_META: Meta = Meta::new("tags", "tags", "text[]").ops(OpSet::equality());
static PAYLOAD_META: Meta = Meta::new("payload", "payload", "jsonb").ops(OpSet::equality());
static SCORE_RANGE_META: Meta =
    Meta::new("score_range", "score_range", "int4range").ops(OpSet::equality());
static USERS_FIELDS: [&Meta; 2] = [&ID_META, &EMAIL_META];
static UUID_FIELDS: [&Meta; 1] = [&UUID_META];
static ORDERS_FIELDS: [&Meta; 5] = [
    &ORDER_USER_ID_META,
    &TOTAL_META,
    &TAGS_META,
    &PAYLOAD_META,
    &SCORE_RANGE_META,
];
const ID: Field<i32> = Field::new(&ID_META);
const EMAIL: Field<String> = Field::new(&EMAIL_META);
const ACTIVE: Field<bool> = Field::new(&ACTIVE_META);
const UUID_ID: Field<uuid::Uuid> = Field::new(&UUID_META);
const ORDER_USER_ID: Field<i32> = Field::new(&ORDER_USER_ID_META);
const TOTAL: Field<i64> = Field::new(&TOTAL_META);
const TAGS: Field<Vec<String>> = Field::new(&TAGS_META);
const PAYLOAD: Field<serde_json::Value> = Field::new(&PAYLOAD_META);
const SCORE_RANGE: Field<sqlx::postgres::types::PgRange<i32>> = Field::new(&SCORE_RANGE_META);

fn users() -> Source {
    Source::Table {
        name: "public.app_users",
        alias: None,
        fields: &USERS_FIELDS,
    }
}

fn orders() -> Source {
    Source::Table {
        name: "public.orders",
        alias: None,
        fields: &ORDERS_FIELDS,
    }
}

#[test]
fn select_renders_typed_predicate_and_default_projection() {
    let stmt = Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source: users(),
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: Vec::new(),
        filter: Some(ID.eq(42)),
        group_by: Vec::new(),
        having: None,
        order: Vec::new(),
        row_limit: None,
        offset: None,
        lock: None,
    }));

    let built = stmt.build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE \"id\" = $1"
    );
    assert_eq!(built.params.len(), 1);
    assert!(built.cacheable);
}

#[test]
fn raw_fragments_are_numbered_in_render_order() {
    let stmt = Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source: Source::Raw {
            sql: "select ?::int4 as id".to_owned(),
            alias: "generated".to_owned(),
            params: vec![Param::typed(1_i32)],
            fields: vec![ID_META],
        },
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: vec![SelectItem {
            expr: raw_expr("?::text", [Param::typed("first".to_owned())]),
            alias: Some("label".to_owned()),
        }],
        filter: Some(raw_predicate("id > ?", [Param::typed(2_i32)])),
        group_by: Vec::new(),
        having: None,
        order: Vec::new(),
        row_limit: None,
        offset: None,
        lock: None,
    }));

    let built = stmt.build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT $1::text AS \"label\" FROM (select $2::int4 as id) AS \"generated\" (\"id\") WHERE id > $3"
    );
    assert_eq!(built.params.len(), 3);
    assert!(!built.cacheable);
}

#[test]
fn raw_fragments_only_replace_placeholders_outside_sql_contexts() {
    let built = raw("SELECT '?' AS literal, \"?col\" AS ident, $$?$$ AS body, \
         /* ? */ $tag$?$tag$ AS tagged, ?? AS escaped, ?::int4 AS bound -- ?")
    .bind(42_i32)
    .build()
    .unwrap();

    assert_eq!(
        built.sql,
        "SELECT '?' AS literal, \"?col\" AS ident, $$?$$ AS body, /* ? */ $tag$?$tag$ AS tagged, ? AS escaped, $1::int4 AS bound -- ?"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn raw_stmt_without_placeholders_keeps_sql_and_disables_cache() {
    let built = raw("SELECT id, email FROM app_users ORDER BY id")
        .build()
        .unwrap();

    assert_eq!(built.sql, "SELECT id, email FROM app_users ORDER BY id");
    assert!(built.params.is_empty());
    assert!(!built.cacheable);
}

#[test]
fn insert_renders_columns_values_and_returning() {
    let insert = Insert {
        ctes: Vec::new(),
        target: users(),
        body: InsertBody::Values(vec![Assignment {
            field: EMAIL_META,
            value: AssignmentValue::Expr(ValueExpr::Param(Param::typed(
                "egor@example.com".to_owned(),
            ))),
        }]),
        conflict: None,
        returning: vec![SelectItem {
            expr: ID.expr(),
            alias: None,
        }],
    };

    let built = insert.build().unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"app_users\" (\"email_address\") VALUES ($1) RETURNING \"id\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn write_returning_accepts_tuple_columns() {
    let insert_sql = insert(users())
        .set(ID.set(1))
        .returning((ID, EMAIL))
        .build()
        .unwrap();
    let update_sql = update(users())
        .set(EMAIL.set("ada@example.com".to_owned()))
        .filter(ID.eq(1))
        .returning((ID, EMAIL))
        .build()
        .unwrap();
    let delete_sql = delete_from(users())
        .filter(ID.eq(1))
        .returning((ID, EMAIL))
        .build()
        .unwrap();
    let merge_sql = merge_into(users(), orders(), ID.eq_field(ORDER_USER_ID))
        .when_matched()
        .update(EMAIL.set("merged@example.com".to_owned()))
        .returning((ID, EMAIL))
        .build()
        .unwrap();

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\") VALUES ($1) RETURNING \"id\", \"email_address\" AS \"email\""
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = $1 WHERE \"id\" = $2 RETURNING \"id\", \"email_address\" AS \"email\""
    );
    assert_eq!(
        delete_sql.sql,
        "DELETE FROM \"public\".\"app_users\" WHERE \"id\" = $1 RETURNING \"id\", \"email_address\" AS \"email\""
    );
    assert_eq!(
        merge_sql.sql,
        "MERGE INTO \"public\".\"app_users\" USING \"public\".\"orders\" ON \"id\" = \"user_id\" WHEN MATCHED THEN UPDATE SET \"email_address\" = $1 RETURNING \"id\", \"email_address\" AS \"email\""
    );
}

#[test]
fn raw_stmt_rejects_bind_mismatch_before_rendering() {
    let err = RawStmt {
        sql: "select ?".to_owned(),
        params: Vec::new(),
    }
    .build()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        }
    ));
}

#[test]
fn typed_field_can_bind_any_sqlx_supported_type() {
    let stmt = Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source: Source::Table {
            name: "app_users",
            alias: None,
            fields: &UUID_FIELDS,
        },
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: vec![SelectItem {
            expr: UUID_ID.expr(),
            alias: None,
        }],
        filter: Some(UUID_ID.eq(uuid::Uuid::nil())),
        group_by: Vec::new(),
        having: None,
        order: Vec::new(),
        row_limit: None,
        offset: None,
        lock: None,
    }));

    let built = stmt.build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\" FROM \"app_users\" WHERE \"id\" = $1"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn value_expr_accepts_common_sqlx_postgres_literal_types() {
    let built = select(users())
        .expr(ValueExpr::from(vec![0xde, 0xad, 0xbe, 0xef]))
        .expr(ValueExpr::from(sqlx::postgres::types::PgInterval {
            months: 0,
            days: 1,
            microseconds: 2,
        }))
        .expr(ValueExpr::from(std::time::Duration::from_micros(3)))
        .expr(ValueExpr::from(chrono::Duration::microseconds(4)))
        .expr(ValueExpr::from(sqlx::types::BigDecimal::from(5_i32)))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT $1, $2, $3, $4, $5 FROM \"public\".\"app_users\""
    );
    assert_eq!(
        built.params.debug_names(),
        vec![
            std::any::type_name::<Vec<u8>>(),
            std::any::type_name::<sqlx::postgres::types::PgInterval>(),
            std::any::type_name::<std::time::Duration>(),
            std::any::type_name::<chrono::Duration>(),
            std::any::type_name::<sqlx::types::BigDecimal>(),
        ]
    );
    built.arguments().unwrap();
}

#[test]
fn null_value_expr_and_set_null_render_without_parameters() {
    let selected = select(users()).expr(crate::null()).build().unwrap();

    assert_eq!(selected.sql, "SELECT NULL FROM \"public\".\"app_users\"");
    assert_eq!(selected.params.len(), 0);

    let updated = update(users())
        .set(EMAIL.set_null())
        .filter(ID.eq(1))
        .build()
        .unwrap();

    assert_eq!(
        updated.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = NULL WHERE \"id\" = $1"
    );
    assert_eq!(updated.params.len(), 1);
}

#[test]
fn default_write_values_render_without_parameters() {
    let inserted = insert(users())
        .set(ID.set(1))
        .set(EMAIL.set_default())
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        inserted.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, DEFAULT) RETURNING \"id\""
    );
    assert_eq!(inserted.params.len(), 1);

    let default_row = insert(users())
        .default_values()
        .returning(ID)
        .build()
        .unwrap();
    assert_eq!(
        default_row.sql,
        "INSERT INTO \"public\".\"app_users\" DEFAULT VALUES RETURNING \"id\""
    );
    assert_eq!(default_row.params.len(), 0);

    let updated = update(users())
        .set(EMAIL.set_default())
        .filter(ID.eq(1))
        .build()
        .unwrap();
    assert_eq!(
        updated.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = DEFAULT WHERE \"id\" = $1"
    );
    assert_eq!(updated.params.len(), 1);
}

#[test]
fn sql_literal_and_typed_date_part_render_without_parameters() {
    let bucket = crate::date_trunc_part(crate::DatePart::Day, current_timestamp());
    let built = select(users())
        .expr_as(bucket.clone(), "day_bucket")
        .expr_as(count_all(), "rows")
        .group_by(bucket.clone())
        .order_asc(bucket)
        .expr_as(
            crate::date_trunc(crate::literal("author's"), current_date()),
            "quoted_part",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT date_trunc('day', CURRENT_TIMESTAMP) AS \"day_bucket\", count(*) AS \"rows\", date_trunc('author''s', CURRENT_DATE) AS \"quoted_part\" FROM \"public\".\"app_users\" GROUP BY date_trunc('day', CURRENT_TIMESTAMP) ORDER BY date_trunc('day', CURRENT_TIMESTAMP) ASC"
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn insert_from_select_all_projects_source_fields_and_conflict_uses_excluded() {
    let incoming = values_source([(1_i32, "egor@example.com")], "incoming", (ID, EMAIL));

    let built = insert(users())
        .from_select_all(incoming)
        .on_conflict(ID)
        .do_update_excluded(EMAIL)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") SELECT \"incoming\".\"id\", \"incoming\".\"email_address\" FROM (VALUES ($1, $2)) AS \"incoming\" (\"id\", \"email_address\") ON CONFLICT (\"id\") DO UPDATE SET \"email_address\" = EXCLUDED.\"email_address\""
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn insert_with_cte_renders_before_insert_and_preserves_bind_order() {
    let ids = cte("ids", raw("SELECT ?::int4 AS id").bind(1_i32), ID);
    let ids_source = ids.source();

    let built = insert(users())
        .with(ids)
        .from_select_all(ids_source)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"ids\" (\"id\") AS (SELECT $1::int4 AS id) INSERT INTO \"public\".\"app_users\" (\"id\") SELECT \"id\" FROM \"ids\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn insert_from_select_all_handles_sources_without_explicit_alias() {
    let staging = table("staging.app_users", &USERS_FIELDS);
    let built = insert(users()).from_select_all(staging).build().unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") SELECT \"id\", \"email_address\" FROM \"staging\".\"app_users\""
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn ergonomic_constructors_build_the_same_typed_ast() {
    let built = select(table("public.app_users", &USERS_FIELDS))
        .column(ID)
        .expr_as(EMAIL, "email")
        .filter(and([ID.gt(10), ID.lt(20)]))
        .filter_if(false, ID.eq(999))
        .filter_option(Some("egor".to_owned()), |email| EMAIL.ne(email))
        .apply(|query| query.order_desc(ID))
        .limit(50)
        .offset(100)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE (\"id\" > $1 AND \"id\" < $2 AND \"email_address\" <> $3) ORDER BY \"id\" DESC LIMIT $4 OFFSET $5"
    );
    assert_eq!(built.params.len(), 5);
}

#[test]
fn conditional_builder_helpers_skip_or_apply_optional_clauses() {
    let select_sql = select(users())
        .column(ID)
        .filter(ID.eq(1))
        .or_filter_option(Some("egor@example.com".to_owned()), |email| EMAIL.eq(email))
        .or_filter_if(false, ACTIVE.eq(false))
        .build()
        .unwrap();

    let insert_sql = insert(users())
        .set_if(false, ID.set(999))
        .set_option(Some("egor@example.com".to_owned()), |email| {
            EMAIL.set(email)
        })
        .set_option(None::<i32>, |id| ID.set(id))
        .build()
        .unwrap();

    let update_sql = update(users())
        .set_if(true, EMAIL.set("new@example.com".to_owned()))
        .set_option(None::<i32>, |id| ID.set(id))
        .filter_if(false, ID.eq(999))
        .filter_option(Some(1), |id| ID.eq(id))
        .build()
        .unwrap();

    let delete_sql = delete_from(users())
        .filter_if(true, ACTIVE.eq(false))
        .filter_option(None::<i32>, |id| ID.eq(id))
        .build()
        .unwrap();

    assert_eq!(
        select_sql.sql,
        "SELECT \"id\" FROM \"public\".\"app_users\" WHERE (\"id\" = $1 OR \"email_address\" = $2)"
    );
    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"public\".\"app_users\" (\"email_address\") VALUES ($1)"
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = $1 WHERE \"id\" = $2"
    );
    assert_eq!(
        delete_sql.sql,
        "DELETE FROM \"public\".\"app_users\" WHERE \"active\" = $1"
    );
}

#[test]
fn joins_render_qualified_fields_and_keep_param_order() {
    let built = select(users().alias("u"))
        .join(
            orders().alias("o"),
            ID.at("u").eq_field(ORDER_USER_ID.at("o")),
        )
        .expr_as(EMAIL.at("u"), "email")
        .column(TOTAL.at("o"))
        .filter(TOTAL.at("o").gte(5000))
        .order_desc(TOTAL.at("o"))
        .limit(10)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"email_address\" AS \"email\", \"o\".\"total_cents\" AS \"o_total_cents\" FROM \"public\".\"app_users\" AS \"u\" JOIN \"public\".\"orders\" AS \"o\" ON \"u\".\"id\" = \"o\".\"user_id\" WHERE \"o\".\"total_cents\" >= $1 ORDER BY \"o\".\"total_cents\" DESC LIMIT $2"
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn select_clause_stack_preserves_global_param_order() {
    let active_ids = cte(
        "active_ids",
        select(users()).column(ID).filter(ACTIVE.eq(true)),
        ID,
    );

    let built = select(users().alias("u"))
        .with(active_ids.clone())
        .join(
            active_ids.source().alias("a"),
            ID.at("u").eq_field(ID.at("a")),
        )
        .join(
            orders().alias("o"),
            and([
                ID.at("u").eq_field(ORDER_USER_ID.at("o")),
                TOTAL.at("o").gt(1000_i64),
            ]),
        )
        .column(EMAIL.at("u"))
        .expr_as(count_all(), "rows")
        .filter(ID.at("u").gt(10))
        .group_by(EMAIL.at("u"))
        .having(count_all().gt(1_i64))
        .order_asc(EMAIL.at("u"))
        .limit(25)
        .offset(50)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"active_ids\" (\"id\") AS (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"active\" = $1) SELECT \"u\".\"email_address\" AS \"u_email\", count(*) AS \"rows\" FROM \"public\".\"app_users\" AS \"u\" JOIN \"active_ids\" AS \"a\" ON \"u\".\"id\" = \"a\".\"id\" JOIN \"public\".\"orders\" AS \"o\" ON (\"u\".\"id\" = \"o\".\"user_id\" AND \"o\".\"total_cents\" > $2) WHERE \"u\".\"id\" > $3 GROUP BY \"u\".\"email_address\" HAVING count(*) > $4 ORDER BY \"u\".\"email_address\" ASC LIMIT $5 OFFSET $6"
    );
    assert_eq!(built.params.len(), 6);
    assert!(built.cacheable);
}

#[test]
fn right_full_cross_and_lateral_joins_render_in_clause_order() {
    let recent = select(orders())
        .column(ORDER_USER_ID)
        .filter(TOTAL.gt(100))
        .infer_source("recent")
        .unwrap();

    let built = select(users().alias("u"))
        .right_join(
            orders().alias("ro"),
            ID.at("u").eq_field(ORDER_USER_ID.at("ro")),
        )
        .full_join(
            orders().alias("fo"),
            ID.at("u").eq_field(ORDER_USER_ID.at("fo")),
        )
        .cross_join(orders().alias("co"))
        .left_join_lateral(recent, true_())
        .column(ID.at("u"))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"id\" AS \"u_id\" FROM \"public\".\"app_users\" AS \"u\" RIGHT JOIN \"public\".\"orders\" AS \"ro\" ON \"u\".\"id\" = \"ro\".\"user_id\" FULL JOIN \"public\".\"orders\" AS \"fo\" ON \"u\".\"id\" = \"fo\".\"user_id\" CROSS JOIN \"public\".\"orders\" AS \"co\" LEFT JOIN LATERAL (SELECT \"user_id\" FROM \"public\".\"orders\" WHERE \"total_cents\" > $1) AS \"recent\" (\"user_id\") ON TRUE"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn aliased_root_default_projection_is_qualified() {
    let built = select(users().alias("u")).build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\""
    );
}

#[test]
fn default_columns_can_be_extended_with_computed_items() {
    let built = select(users())
        .default_columns()
        .expr_as(crate::length(EMAIL), "email_length")
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\", length(\"email_address\") AS \"email_length\" FROM \"public\".\"app_users\""
    );
}

#[test]
fn default_columns_preserves_aliased_root_projection_shape() {
    let built = select(users().alias("u"))
        .default_columns()
        .expr_as(crate::length(EMAIL.at("u")), "email_length")
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\", length(\"u\".\"email_address\") AS \"email_length\" FROM \"public\".\"app_users\" AS \"u\""
    );
}

#[test]
fn default_columns_respects_builder_order() {
    let built = select(users())
        .expr_as(crate::length(EMAIL), "email_length")
        .default_columns()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT length(\"email_address\") AS \"email_length\", \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\""
    );
}

#[test]
fn default_columns_only_expands_root_fields_for_joined_queries() {
    let built = select(users().alias("u"))
        .join(
            orders().alias("o"),
            ID.at("u").eq_field(ORDER_USER_ID.at("o")),
        )
        .default_columns()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\" JOIN \"public\".\"orders\" AS \"o\" ON \"u\".\"id\" = \"o\".\"user_id\""
    );
}

#[test]
fn everyday_predicates_render_without_raw_sql() {
    let built = select(users())
        .filter(BoolExpr::and([
            ID.is_not_null(),
            ID.in_list([1, 2, 3]),
            EMAIL.like("%@example.com"),
            EMAIL.contains("50%_match"),
            EMAIL.regex("@example\\.com$"),
            EMAIL.iregex("@example\\.org$"),
            EMAIL.is_distinct_from("blocked@example.com".to_owned()),
            ID.between(10, 20),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE (\"id\" IS NOT NULL AND \"id\" IN ($1, $2, $3) AND \"email_address\" LIKE $4 AND \"email_address\" ILIKE $5 ESCAPE '\\' AND \"email_address\" ~ $6 AND \"email_address\" ~* $7 AND \"email_address\" IS DISTINCT FROM $8 AND \"id\" BETWEEN $9 AND $10)"
    );
    assert_eq!(built.params.len(), 10);
}

#[test]
fn negated_field_predicates_render_without_raw_sql() {
    let built = select(users())
        .filter(BoolExpr::and([
            ID.not_in([1, 2]),
            EMAIL.not_like("%@spam.test"),
            EMAIL.not_ilike("%@spam.test"),
            EMAIL.not_contains("spam"),
            EMAIL.not_starts_with("tmp"),
            EMAIL.not_ends_with(".bak"),
            EMAIL.not_regex("^tmp"),
            EMAIL.not_iregex("^tmp"),
            ID.not_between(10, 20),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE (\"id\" NOT IN ($1, $2) AND \"email_address\" NOT LIKE $3 AND \"email_address\" NOT ILIKE $4 AND \"email_address\" NOT ILIKE $5 ESCAPE '\\' AND \"email_address\" NOT ILIKE $6 ESCAPE '\\' AND \"email_address\" NOT ILIKE $7 ESCAPE '\\' AND \"email_address\" !~ $8 AND \"email_address\" !~* $9 AND \"id\" NOT BETWEEN $10 AND $11)"
    );
    assert_eq!(built.params.len(), 11);
}

#[test]
fn distinct_group_and_having_render_as_select_clauses() {
    fn count_id() -> ValueExpr {
        crate::count(ID)
    }

    let built = select(users())
        .distinct_on(EMAIL)
        .column(EMAIL)
        .expr_as(count_id(), "user_count")
        .group_by(EMAIL)
        .having(count_id().gt(1_i64))
        .order_asc(EMAIL)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT DISTINCT ON (\"email_address\") \"email_address\" AS \"email\", count(\"id\") AS \"user_count\" FROM \"public\".\"app_users\" GROUP BY \"email_address\" HAVING count(\"id\") > $1 ORDER BY \"email_address\" ASC"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn write_constructors_use_field_t_assignments() {
    let insert_sql = insert(users())
        .set(EMAIL.set("new@example.com".to_owned()))
        .returning(ID)
        .build()
        .unwrap();
    let update_sql = update(users())
        .set(EMAIL.set("updated@example.com".to_owned()))
        .filter(ID.eq(1))
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"public\".\"app_users\" (\"email_address\") VALUES ($1) RETURNING \"id\""
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = $1 WHERE \"id\" = $2 RETURNING \"id\""
    );
}

#[test]
fn write_assignment_batches_accept_tuples() {
    let insert_sql = insert(users())
        .set_many((
            ID.set(1),
            EMAIL.set("old@example.com".to_owned()),
            EMAIL.set("new@example.com".to_owned()),
        ))
        .returning(ID)
        .build()
        .unwrap();

    let update_sql = update(users())
        .set_many((
            EMAIL.set_expr(EMAIL.expr().op("||", "@example.com")),
            ID.set(2),
        ))
        .filter(ID.eq(1))
        .build()
        .unwrap();
    let wide_insert_sql = insert(users())
        .set_many((
            ID.set(1),
            EMAIL.set("email-2".to_owned()),
            ID.set(3),
            EMAIL.set("email-4".to_owned()),
            ID.set(5),
            EMAIL.set("email-6".to_owned()),
            ID.set(7),
            EMAIL.set("email-8".to_owned()),
            ID.set(9),
            EMAIL.set("email-10".to_owned()),
            ID.set(11),
            EMAIL.set("email-12".to_owned()),
            ID.set(13),
            EMAIL.set("email-14".to_owned()),
            ID.set(15),
            EMAIL.set("email-16".to_owned()),
        ))
        .build()
        .unwrap();

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, $2) RETURNING \"id\""
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = (\"email_address\" || $1), \"id\" = $2 WHERE \"id\" = $3"
    );
    assert_eq!(
        wide_insert_sql.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, $2)"
    );
    assert_eq!(insert_sql.params.len(), 2);
    assert_eq!(update_sql.params.len(), 3);
    assert_eq!(wide_insert_sql.params.len(), 2);
}

#[test]
fn pg18_returning_old_new_and_computed_assignments_render() {
    let built = update(users())
        .set(EMAIL.set_expr(EMAIL.expr().op("||", "@new.example")))
        .filter(ID.eq(1))
        .returning_as(EMAIL.old_value(), "old_email")
        .returning_as(EMAIL.new_value(), "new_email")
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = (\"email_address\" || $1) WHERE \"id\" = $2 RETURNING \"old\".\"email_address\" AS \"old_email\", \"new\".\"email_address\" AS \"new_email\""
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn expression_backed_text_and_array_predicates_render() {
    let text_built = select(users())
        .column(ID)
        .filter(EMAIL.like_expr(crate::lower("%@example.com")))
        .filter(EMAIL.contains_expr(crate::lower(EMAIL)))
        .filter(EMAIL.text_search_expr(crate::plainto_tsquery(EMAIL)))
        .filter(EMAIL.websearch_expr(EMAIL))
        .build()
        .unwrap();

    assert_eq!(
        text_built.sql,
        "SELECT \"id\" FROM \"public\".\"app_users\" WHERE (\"email_address\" LIKE lower($1) AND \"email_address\" ILIKE (($2 || replace(replace(replace(lower(\"email_address\"), $3, $4), $5, $6), $7, $8)) || $9) ESCAPE '\\' AND to_tsvector(\"email_address\") @@ plainto_tsquery(\"email_address\") AND to_tsvector(\"email_address\") @@ websearch_to_tsquery(\"email_address\"))"
    );
    assert_eq!(text_built.params.len(), 9);

    let array_built = select(orders())
        .column(ORDER_USER_ID)
        .filter(TAGS.overlaps_expr(array(["vip", "staff"])))
        .filter(TAGS.contains_expr(crate::array_cat(TAGS, array(["paid"]))))
        .filter(TAGS.contained_by_expr(crate::array_cat(TAGS, array(["archived"]))))
        .filter(TAGS.has_expr(crate::lower("VIP")))
        .filter(TAGS.not_has_expr("blocked"))
        .build()
        .unwrap();

    assert_eq!(
        array_built.sql,
        "SELECT \"user_id\" FROM \"public\".\"orders\" WHERE (\"tags\" && ARRAY[$1, $2] AND \"tags\" @> array_cat(\"tags\", ARRAY[$3]) AND \"tags\" <@ array_cat(\"tags\", ARRAY[$4]) AND lower($5) = ANY(\"tags\") AND NOT ($6 = ANY(\"tags\")))"
    );
    assert_eq!(array_built.params.len(), 6);
}

#[test]
fn variadic_select_projection_helpers_render() {
    let sixteen_items = (
        ID, EMAIL, ID, EMAIL, ID, EMAIL, ID, EMAIL, ID, EMAIL, ID, EMAIL, ID, EMAIL, ID, EMAIL,
    )
        .into_columns();
    let built = select(users())
        .columns((ID, EMAIL))
        .expr(crate::lower(EMAIL))
        .expr(crate::upper(EMAIL))
        .expr_as(crate::length(EMAIL), "email_length")
        .expr_as(crate::count_all(), "rows")
        .build()
        .unwrap();

    assert_eq!(sixteen_items.items.len(), 16);
    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\", lower(\"email_address\"), upper(\"email_address\"), length(\"email_address\") AS \"email_length\", count(*) AS \"rows\" FROM \"public\".\"app_users\""
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn frame_bound_constructors_render_without_manual_boxing() {
    let built = select(orders())
        .column(ORDER_USER_ID)
        .expr_as(
            lag(TOTAL).over(
                window()
                    .partition_by(ORDER_USER_ID)
                    .order_asc(TOTAL)
                    .frame(crate::rows(crate::preceding(2_i32)).between(crate::following(1_i32))),
            ),
            "nearby_total",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"user_id\", lag(\"total_cents\") OVER (PARTITION BY \"user_id\" ORDER BY \"total_cents\" ASC ROWS BETWEEN $1 PRECEDING AND $2 FOLLOWING) AS \"nearby_total\" FROM \"public\".\"orders\""
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn later_write_assignments_replace_earlier_ones_for_same_column() {
    let insert_sql = insert(users())
        .set(ID.set(1))
        .set(EMAIL.set("old@example.com".to_owned()))
        .set(EMAIL.set("new@example.com".to_owned()))
        .build()
        .unwrap();
    let update_sql = update(users())
        .set(EMAIL.set("old@example.com".to_owned()))
        .set(EMAIL.set("new@example.com".to_owned()))
        .filter(ID.eq(1))
        .build()
        .unwrap();

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, $2)"
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"app_users\" SET \"email_address\" = $1 WHERE \"id\" = $2"
    );
    assert_eq!(insert_sql.params.len(), 2);
    assert_eq!(update_sql.params.len(), 2);
}

#[test]
fn insert_from_select_renders_columns_and_nested_select_params() {
    let source = select(users()).columns((ID, EMAIL)).filter(ID.gt(10));
    let built = insert(users())
        .from_select((ID, EMAIL), source)
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE \"id\" > $1 RETURNING \"id\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn insert_from_select_default_projection_ignores_joined_fields() {
    let source = select(users().alias("u")).join(
        orders().alias("o"),
        ID.at("u").eq_field(ORDER_USER_ID.at("o")),
    );
    let built = insert(users())
        .from_select((ID, EMAIL), source)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\" JOIN \"public\".\"orders\" AS \"o\" ON \"u\".\"id\" = \"o\".\"user_id\""
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn insert_on_conflict_renders_update_and_do_nothing_actions() {
    let update = insert(users())
        .set_many((ID.set(1), EMAIL.set("new@example.com".to_owned())))
        .on_conflict(ID)
        .target_where(ID.gt(0))
        .do_update_set_where(
            (EMAIL.set_excluded(), ID.set(2)),
            EMAIL.ne("old@example.com".to_owned()),
        )
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        update.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, $2) ON CONFLICT (\"id\") WHERE \"id\" > $3 DO UPDATE SET \"email_address\" = EXCLUDED.\"email_address\", \"id\" = $4 WHERE \"email_address\" <> $5 RETURNING \"id\""
    );
    assert_eq!(update.params.len(), 5);

    let excluded = insert(users())
        .set_many((ID.set(1), EMAIL.set("new@example.com".to_owned())))
        .on_conflict(ID)
        .do_update_excluded((EMAIL, ID))
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        excluded.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, $2) ON CONFLICT (\"id\") DO UPDATE SET \"email_address\" = EXCLUDED.\"email_address\", \"id\" = EXCLUDED.\"id\" RETURNING \"id\""
    );
    assert_eq!(excluded.params.len(), 2);

    let nothing = insert(users())
        .set(ID.set(1))
        .on_conflict_constraint("app_users_pkey")
        .do_nothing()
        .build()
        .unwrap();

    assert_eq!(
        nothing.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\") VALUES ($1) ON CONFLICT ON CONSTRAINT \"app_users_pkey\" DO NOTHING"
    );
}

#[test]
fn default_assignments_render_in_conflict_and_merge_actions() {
    let conflict = insert(users())
        .set(ID.set(1))
        .on_conflict(ID)
        .do_update_set(EMAIL.set_default())
        .build()
        .unwrap();

    assert_eq!(
        conflict.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\") VALUES ($1) ON CONFLICT (\"id\") DO UPDATE SET \"email_address\" = DEFAULT"
    );
    assert_eq!(conflict.params.len(), 1);

    let merge = merge_into(
        users().alias("u"),
        orders().alias("incoming"),
        ID.at("u").eq_field(ORDER_USER_ID.at("incoming")),
    )
    .when_matched()
    .update(EMAIL.set_default())
    .when_not_matched()
    .insert((ID.set(1), EMAIL.set_default()))
    .build()
    .unwrap();

    assert_eq!(
        merge.sql,
        "MERGE INTO \"public\".\"app_users\" AS \"u\" USING \"public\".\"orders\" AS \"incoming\" ON \"u\".\"id\" = \"incoming\".\"user_id\" WHEN MATCHED THEN UPDATE SET \"email_address\" = DEFAULT WHEN NOT MATCHED THEN INSERT (\"id\", \"email_address\") VALUES ($1, DEFAULT)"
    );
    assert_eq!(merge.params.len(), 1);
}

#[test]
fn merge_with_using_subquery_keeps_param_order_across_actions() {
    let incoming = select(orders())
        .columns((ORDER_USER_ID, TOTAL))
        .filter(TOTAL.gt(500_i64))
        .infer_source("incoming")
        .unwrap();

    let built = merge_into(
        users().alias("u"),
        incoming,
        and([
            ID.at("u").eq_field(ORDER_USER_ID.at("incoming")),
            TOTAL.at("incoming").gt(750_i64),
        ]),
    )
    .when_matched_if(TOTAL.at("incoming").gt(1000_i64))
    .update(EMAIL.set("merged@example.com".to_owned()))
    .when_not_matched()
    .insert((ID.set(1), EMAIL.set("new@example.com".to_owned())))
    .returning_as(ID.at("u"), "id")
    .build()
    .unwrap();

    assert_eq!(
        built.sql,
        "MERGE INTO \"public\".\"app_users\" AS \"u\" USING (SELECT \"user_id\", \"total_cents\" FROM \"public\".\"orders\" WHERE \"total_cents\" > $1) AS \"incoming\" (\"user_id\", \"total_cents\") ON (\"u\".\"id\" = \"incoming\".\"user_id\" AND \"incoming\".\"total_cents\" > $2) WHEN MATCHED AND \"incoming\".\"total_cents\" > $3 THEN UPDATE SET \"email_address\" = $4 WHEN NOT MATCHED THEN INSERT (\"id\", \"email_address\") VALUES ($5, $6) RETURNING \"u\".\"id\" AS \"id\""
    );
    assert_eq!(built.params.len(), 6);
    assert!(built.cacheable);
}

#[test]
fn set_queries_render_with_order_limit_and_param_order() {
    let left = select(users()).column(ID).filter(ID.gt(10));
    let right = select(users()).column(ID).filter(ID.lt(3));

    let built = left
        .union_all(right)
        .order_desc(ID)
        .limit(5)
        .offset(2)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "(SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"id\" > $1) UNION ALL (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"id\" < $2) ORDER BY \"id\" DESC LIMIT $3 OFFSET $4"
    );
    assert_eq!(built.params.len(), 4);
}

#[test]
fn in_subquery_predicate_renders_server_owned_query_shape() {
    let subquery = select(orders())
        .column(ORDER_USER_ID)
        .filter(TOTAL.gt(1000));
    let built = select(users())
        .filter(ID.in_subquery(subquery))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE \"id\" IN (SELECT \"user_id\" FROM \"public\".\"orders\" WHERE \"total_cents\" > $1)"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn scalar_subquery_renders_inside_delete_predicates() {
    let cutoff_total = scalar_subquery(
        select(orders())
            .column(TOTAL)
            .filter(ORDER_USER_ID.eq(7))
            .order_desc(TOTAL)
            .offset(200)
            .limit(1),
    );
    let built = crate::delete_from(orders())
        .filter(ORDER_USER_ID.eq(7))
        .filter(TOTAL.expr().lte(cutoff_total))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "DELETE FROM \"public\".\"orders\" WHERE (\"user_id\" = $1 AND \"total_cents\" <= (SELECT \"total_cents\" FROM \"public\".\"orders\" WHERE \"user_id\" = $2 ORDER BY \"total_cents\" DESC LIMIT $3 OFFSET $4))"
    );
    assert_eq!(built.params.len(), 4);
}

#[test]
fn select_infer_source_infers_field_metadata_from_projection() {
    let recent = select(orders())
        .column(ORDER_USER_ID)
        .column(TOTAL)
        .filter(TOTAL.gt(1000))
        .infer_source("recent_orders")
        .unwrap();

    let built = select(recent.alias("r"))
        .column(ORDER_USER_ID.at("r"))
        .column(TOTAL.at("r"))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"r\".\"user_id\" AS \"r_user_id\", \"r\".\"total_cents\" AS \"r_total_cents\" FROM (SELECT \"user_id\", \"total_cents\" FROM \"public\".\"orders\" WHERE \"total_cents\" > $1) AS \"r\" (\"user_id\", \"total_cents\")"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn select_infer_source_rejects_projection_aliases_that_need_explicit_fields() {
    let err = select(users())
        .expr_as(EMAIL, "unknown_output_name")
        .infer_source("emails")
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSelectShape { message }
            if message == "infer_source cannot infer fields from aliased projection; use into_source"
    ));
}

#[test]
fn recursive_cte_ref_renders_columns_and_body_params() {
    let seed = select(users()).column(ID).filter(ID.eq(1));
    let recursive_arm = select(crate::cte_ref("active_users", vec![ID_META]))
        .column(ID)
        .filter(ID.lt(10));
    let active_users =
        cte("active_users", seed.union_all(recursive_arm), vec![ID_META]).recursive();

    let built = select(active_users.source())
        .with(active_users)
        .column(ID)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH RECURSIVE \"active_users\" (\"id\") AS ((SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"id\" = $1) UNION ALL (SELECT \"id\" FROM \"active_users\" WHERE \"id\" < $2)) SELECT \"id\" FROM \"active_users\""
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn not_materialized_cte_renders_hint_and_auto_columns() {
    let active = select(users())
        .column(ID)
        .filter(ACTIVE.eq(true))
        .infer_cte("active_users")
        .unwrap()
        .not_materialized();

    let built = select(active.source())
        .with(active)
        .column(ID)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"active_users\" (\"id\") AS NOT MATERIALIZED (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"active\" = $1) SELECT \"id\" FROM \"active_users\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn joined_cte_definitions_render_before_select_and_keep_params() {
    let big_orders = cte(
        "big_orders",
        select(orders())
            .column(ORDER_USER_ID)
            .filter(TOTAL.gt(1000)),
        vec![ORDER_USER_ID_META],
    );
    let big_orders_source = big_orders.source().alias("bo");

    let built = select(users().alias("u"))
        .with(big_orders)
        .join(
            big_orders_source,
            ID.at("u").eq_field(ORDER_USER_ID.at("bo")),
        )
        .column(EMAIL.at("u"))
        .filter(ID.at("u").gt(10))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"big_orders\" (\"user_id\") AS (SELECT \"user_id\" FROM \"public\".\"orders\" WHERE \"total_cents\" > $1) SELECT \"u\".\"email_address\" AS \"u_email\" FROM \"public\".\"app_users\" AS \"u\" JOIN \"big_orders\" AS \"bo\" ON \"u\".\"id\" = \"bo\".\"user_id\" WHERE \"u\".\"id\" > $2"
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn cte_fields_must_match_select_projection() {
    let invalid = cte(
        "broken",
        select(users()).column(ID),
        vec![ID_META, EMAIL_META],
    );

    let err = select(invalid.source()).with(invalid).build().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidCteShape(err) if err.name == "broken"
    ));
}

#[test]
fn window_helpers_render_over_partition_and_order_specs() {
    let built = select(users())
        .column(ID)
        .expr_as(
            row_number().over(window().partition_by(EMAIL).order_desc(ID)),
            "row_no",
        )
        .expr_as(
            lag(EMAIL)
                .offset(ValueExpr::Param(Param::typed(2_i32)))
                .over(window().order_asc(ID)),
            "previous_email",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", row_number() OVER (PARTITION BY \"email_address\" ORDER BY \"id\" DESC) AS \"row_no\", lag(\"email_address\", $1) OVER (ORDER BY \"id\" ASC) AS \"previous_email\" FROM \"public\".\"app_users\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn aggregate_helpers_render_over_window_specs() {
    let built = select(orders())
        .column(ORDER_USER_ID)
        .expr_as(
            crate::sum(TOTAL).over(window().partition_by(ORDER_USER_ID).order_desc(TOTAL)),
            "running_total",
        )
        .expr_as(
            count_all()
                .aggregate_filter(TOTAL.gt(0))
                .over(window().partition_by(ORDER_USER_ID)),
            "positive_user_rows",
        )
        .expr_as(count_all().over(window()), "all_rows")
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"user_id\", sum(\"total_cents\") OVER (PARTITION BY \"user_id\" ORDER BY \"total_cents\" DESC) AS \"running_total\", count(*) FILTER (WHERE \"total_cents\" > $1) OVER (PARTITION BY \"user_id\") AS \"positive_user_rows\", count(*) OVER () AS \"all_rows\" FROM \"public\".\"orders\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn aggregate_helpers_render_common_postgres_aggregates() {
    let built = select(users())
        .expr_as(count_all(), "total")
        .expr_as(count_distinct(EMAIL), "unique_emails")
        .expr_as(
            array_agg(EMAIL)
                .aggregate_order_desc(ID)
                .aggregate_filter(ID.gt(10)),
            "emails",
        )
        .expr_as(
            json_agg(ID)
                .aggregate_order_asc(EMAIL)
                .aggregate_filter(ID.gt(20)),
            "ids_json",
        )
        .expr_as(crate::string_agg(EMAIL, ","), "emails_csv")
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT count(*) AS \"total\", count(DISTINCT \"email_address\") AS \"unique_emails\", array_agg(\"email_address\" ORDER BY \"id\" DESC) FILTER (WHERE \"id\" > $1) AS \"emails\", json_agg(\"id\" ORDER BY \"email_address\" ASC) FILTER (WHERE \"id\" > $2) AS \"ids_json\", string_agg(\"email_address\", $3) AS \"emails_csv\" FROM \"public\".\"app_users\""
    );
    assert_eq!(built.params.len(), 3);
}

#[test]
fn jsonb_agg_object_renders_keyed_objects_with_filter_and_order() {
    let built = select(orders())
        .expr_as(
            crate::jsonb_agg_object![ORDER_USER_ID, TOTAL]
                .aggregate_order_desc(TOTAL)
                .aggregate_filter(TOTAL.gt(0)),
            "orders",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT jsonb_agg(jsonb_build_object($1, \"user_id\", $2, \"total_cents\") ORDER BY \"total_cents\" DESC) FILTER (WHERE \"total_cents\" > $3) AS \"orders\" FROM \"public\".\"orders\""
    );
    assert_eq!(built.params.len(), 3);
}

#[test]
fn jsonb_agg_object_accepts_explicit_keys_for_computed_values() {
    let built = select(orders())
        .expr_as(
            crate::jsonb_agg_object![
                ORDER_USER_ID,
                ("source", json_get_text(PAYLOAD, param("source".to_owned()))),
            ],
            "orders",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT jsonb_agg(jsonb_build_object($1, \"user_id\", $2, (\"payload\" ->> $3))) AS \"orders\" FROM \"public\".\"orders\""
    );
    assert_eq!(built.params.len(), 3);
}

#[test]
fn count_query_renders_through_normal_ast_path() {
    let query = select(users())
        .column(ID)
        .filter(ID.gt(10))
        .order_desc(ID)
        .limit(25)
        .offset(50);

    let count = query.build_count().unwrap();

    assert_eq!(
        count.sql,
        "SELECT count(*) FROM (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"id\" > $1) AS \"rqb_count\""
    );
    assert_eq!(count.params.len(), 1);
    assert!(count.cacheable);
}

#[test]
fn count_query_preserves_ctes_and_strips_runtime_clauses() {
    let ids = cte("ids", raw("SELECT ?::int4 AS id").bind(1_i32), ID);

    let count = select(users().alias("u"))
        .with(ids.clone())
        .join(ids.source().alias("i"), ID.at("u").eq_field(ID.at("i")))
        .column(ID.at("u"))
        .filter(raw_predicate("\"u\".\"id\" > ?", [Param::typed(2_i32)]))
        .order_desc(ID.at("u"))
        .limit(10)
        .offset(20)
        .for_update()
        .skip_locked()
        .build_count()
        .unwrap();

    assert_eq!(
        count.sql,
        "SELECT count(*) FROM (WITH \"ids\" (\"id\") AS (SELECT $1::int4 AS id) SELECT \"u\".\"id\" AS \"u_id\" FROM \"public\".\"app_users\" AS \"u\" JOIN \"ids\" AS \"i\" ON \"u\".\"id\" = \"i\".\"id\" WHERE \"u\".\"id\" > $2) AS \"rqb_count\""
    );
    assert_eq!(count.params.len(), 2);
    assert!(!count.cacheable);
}

#[test]
fn keyword_helpers_render_without_function_parentheses_or_params() {
    let built = select(users())
        .expr_as(current_timestamp(), "ts")
        .expr_as(current_date(), "today")
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT CURRENT_TIMESTAMP AS \"ts\", CURRENT_DATE AS \"today\" FROM \"public\".\"app_users\""
    );
    assert_eq!(built.params.len(), 0);
    assert!(built.cacheable);
}

#[test]
fn row_array_subscript_slice_extract_and_cast_render_value_expressions() {
    let built = select(orders())
        .expr_as(row((ORDER_USER_ID, TOTAL)), "row_value")
        .expr_as(array([ORDER_USER_ID.expr(), ValueExpr::from(2_i32)]), "ids")
        .expr_as(subscript(TAGS, ValueExpr::from(1_i32)), "first_tag")
        .expr_as(
            slice(
                TAGS,
                Some(ValueExpr::from(1_i32)),
                Some(ValueExpr::from(3_i32)),
            ),
            "tag_slice",
        )
        .expr_as(extract("year", current_timestamp()), "year")
        .expr_as(
            ValueExpr::Cast {
                expr: Box::new(ValueExpr::from("42")),
                pg: "int4",
            },
            "answer",
        )
        .filter(row((ORDER_USER_ID, TOTAL)).lt((1_i32, 100_i64)))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT ROW(\"user_id\", \"total_cents\") AS \"row_value\", ARRAY[\"user_id\", $1] AS \"ids\", \"tags\"[$2] AS \"first_tag\", \"tags\"[$3:$4] AS \"tag_slice\", extract(year FROM CURRENT_TIMESTAMP) AS \"year\", CAST($5 AS int4) AS \"answer\" FROM \"public\".\"orders\" WHERE ROW(\"user_id\", \"total_cents\") < ROW($6, $7)"
    );
    assert_eq!(built.params.len(), 7);
}

#[test]
fn array_and_json_predicates_render_without_raw_sql() {
    let built = select(orders())
        .filter(BoolExpr::and([
            TAGS.contains(vec!["paid".to_owned(), "vip".to_owned()]),
            TAGS.has("urgent".to_owned()),
            TAGS.is_not_empty(),
            PAYLOAD.has_key("source"),
            PAYLOAD.has_any_keys(vec!["card".to_owned(), "bank".to_owned()]),
            PAYLOAD.contains(serde_json::json!({ "channel": "web" })),
            SCORE_RANGE.range_contains(42),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"user_id\", \"total_cents\", \"tags\", \"payload\", \"score_range\" FROM \"public\".\"orders\" WHERE (\"tags\" @> $1 AND $2 = ANY(\"tags\") AND cardinality(\"tags\") > 0 AND \"payload\" ? $3 AND \"payload\" ?| $4 AND \"payload\" @> $5 AND \"score_range\" @> $6)"
    );
    assert_eq!(built.params.len(), 6);
}

#[test]
fn postgres_tier1_select_clauses_render_without_raw_sql() {
    let built = select(orders())
        .column(ORDER_USER_ID)
        .expr_as(crate::sum(TOTAL), "total")
        .rollup([ORDER_USER_ID])
        .grouping_sets(vec![vec![ORDER_USER_ID.expr()], vec![TOTAL.expr()]])
        .order_desc_nulls_last(TOTAL)
        .fetch_first_with_ties(param(10_i64))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"user_id\", sum(\"total_cents\") AS \"total\" FROM \"public\".\"orders\" GROUP BY ROLLUP(\"user_id\"), GROUPING SETS ((\"user_id\"), (\"total_cents\")) ORDER BY \"total_cents\" DESC NULLS LAST FETCH FIRST $1 ROWS WITH TIES"
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn update_from_delete_using_and_lock_of_render_with_aliases() {
    let update_sql = update(users().alias("u"))
        .set(EMAIL.set("merged@example.com".to_owned()))
        .from(orders().alias("o"))
        .filter(ID.at("u").eq_field(ORDER_USER_ID.at("o")))
        .build()
        .unwrap();

    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"app_users\" AS \"u\" SET \"email_address\" = $1 FROM \"public\".\"orders\" AS \"o\" WHERE \"u\".\"id\" = \"o\".\"user_id\""
    );
    assert_eq!(update_sql.params.len(), 1);

    let delete_sql = crate::delete_from(users().alias("u"))
        .using(orders().alias("o"))
        .filter(ID.at("u").eq_field(ORDER_USER_ID.at("o")))
        .build()
        .unwrap();

    assert_eq!(
        delete_sql.sql,
        "DELETE FROM \"public\".\"app_users\" AS \"u\" USING \"public\".\"orders\" AS \"o\" WHERE \"u\".\"id\" = \"o\".\"user_id\""
    );

    let lock_sql = select(users().alias("u"))
        .for_update_of("u")
        .nowait()
        .build()
        .unwrap();

    assert_eq!(
        lock_sql.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\" FOR UPDATE OF \"u\" NOWAIT"
    );

    let no_key_update_sql = select(users().alias("u"))
        .for_no_key_update_of("u")
        .skip_locked()
        .build()
        .unwrap();

    assert_eq!(
        no_key_update_sql.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\" FOR NO KEY UPDATE OF \"u\" SKIP LOCKED"
    );

    let key_share_sql = select(users()).for_key_share().build().unwrap();

    assert_eq!(
        key_share_sql.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" FOR KEY SHARE"
    );

    let key_share_of_sql = select(users().alias("u"))
        .for_key_share_of("u")
        .build()
        .unwrap();

    assert_eq!(
        key_share_of_sql.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\" FOR KEY SHARE OF \"u\""
    );
}

#[test]
fn update_with_cte_renders_with_prefix_and_keeps_param_order() {
    let active_ids = cte(
        "active_ids",
        select(users()).column(ID).filter(ACTIVE.eq(true)),
        ID,
    );
    let active_source = active_ids.source().alias("a");

    let built = update(users().alias("u"))
        .with(active_ids)
        .set(EMAIL.set("active@example.com".to_owned()))
        .from(active_source)
        .filter(ID.at("u").eq_field(ID.at("a")))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"active_ids\" (\"id\") AS (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"active\" = $1) UPDATE \"public\".\"app_users\" AS \"u\" SET \"email_address\" = $2 FROM \"active_ids\" AS \"a\" WHERE \"u\".\"id\" = \"a\".\"id\""
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn delete_with_cte_renders_with_prefix_and_keeps_param_order() {
    let small_orders = cte(
        "small_orders",
        select(orders())
            .column(ORDER_USER_ID)
            .filter(TOTAL.lt(5_000_i64)),
        ORDER_USER_ID,
    );
    let small_source = small_orders.source().alias("small");

    let built = delete_from(users().alias("u"))
        .with(small_orders)
        .using(small_source)
        .filter(and([
            ID.at("u").eq_field(ORDER_USER_ID.at("small")),
            ACTIVE.at("u").eq(false),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"small_orders\" (\"user_id\") AS (SELECT \"user_id\" FROM \"public\".\"orders\" WHERE \"total_cents\" < $1) DELETE FROM \"public\".\"app_users\" AS \"u\" USING \"small_orders\" AS \"small\" WHERE (\"u\".\"id\" = \"small\".\"user_id\" AND \"u\".\"active\" = $2)"
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn window_frames_and_more_window_functions_render() {
    let built = select(users())
        .expr_as(
            crate::first_value(EMAIL).over(
                window()
                    .partition_by(ACTIVE)
                    .order_asc_nulls_last(ID)
                    .frame(crate::rows(crate::unbounded_preceding()).between(crate::current_row())),
            ),
            "first_email",
        )
        .expr_as(
            crate::nth_value(EMAIL, param(2_i32)).over(window().order_desc(ID)),
            "second_email",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT first_value(\"email_address\") OVER (PARTITION BY \"active\" ORDER BY \"id\" ASC NULLS LAST ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS \"first_email\", nth_value(\"email_address\", $1) OVER (ORDER BY \"id\" DESC) AS \"second_email\" FROM \"public\".\"app_users\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn helper_functions_cover_common_postgres_builtins() {
    let built = select(orders())
        .expr_as(bool_and(ACTIVE), "all_active")
        .expr_as(percentile_cont(param(0.95_f64), TOTAL), "p95")
        .expr_as(
            case().when(TOTAL.gte(10_000), "large").else_("standard"),
            "bucket",
        )
        .expr_as(TOTAL.op("%", 100_i64), "total_remainder")
        .expr_as(
            coalesce([
                json_get_text(PAYLOAD, param("source".to_owned())),
                param("unknown".to_owned()),
            ]),
            "source",
        )
        .expr_as(to_jsonb(array([ORDER_USER_ID.expr(), TOTAL.expr()])), "ids")
        .filter(BoolExpr::and([
            ACTIVE.is_not_false(),
            EMAIL.similar_to("%(example|test)%"),
            EMAIL.websearch("rust postgres"),
            PAYLOAD
                .path_text(vec!["customer".to_owned(), "tier".to_owned()])
                .is_not_null(),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT bool_and(\"active\") AS \"all_active\", percentile_cont($1) WITHIN GROUP (ORDER BY \"total_cents\" ASC) AS \"p95\", CASE WHEN \"total_cents\" >= $2 THEN $3 ELSE $4 END AS \"bucket\", (\"total_cents\" % $5) AS \"total_remainder\", coalesce((\"payload\" ->> $6), $7) AS \"source\", to_jsonb(ARRAY[\"user_id\", \"total_cents\"]) AS \"ids\" FROM \"public\".\"orders\" WHERE (\"active\" IS NOT FALSE AND \"email_address\" SIMILAR TO $8 AND to_tsvector(\"email_address\") @@ websearch_to_tsquery($9) AND (\"payload\" #>> $10) IS NOT NULL)"
    );
    assert_eq!(built.params.len(), 10);
}

#[test]
fn postgres_gap_helpers_render_without_raw_sql() {
    let built = select(users())
        .expr_as(
            crate::to_char(crate::current_timestamp(), "YYYY-MM"),
            "month",
        )
        .expr_as(crate::to_date("2026-05-11", "YYYY-MM-DD"), "parsed_date")
        .expr_as(
            crate::to_timestamp("2026-05-11 12:00", "YYYY-MM-DD HH24:MI"),
            "parsed_ts",
        )
        .expr_as(crate::to_number("1,234.50", "9,999.99"), "parsed_number")
        .expr_as(
            crate::date_bin("1 hour", crate::current_timestamp(), "2000-01-01"),
            "bucket",
        )
        .expr_as(crate::octet_length(EMAIL), "email_bytes")
        .expr_as(crate::initcap(EMAIL), "email_title")
        .expr_as(crate::encode(param(vec![0xde_u8, 0xad]), "hex"), "encoded")
        .expr_as(crate::decode("dead", "hex"), "decoded")
        .expr_as(crate::ascii("A"), "ascii_a")
        .expr_as(crate::chr(65_i32), "chr_a")
        .expr_as(crate::current_user(), "current_user")
        .expr_as(crate::session_user(), "session_user")
        .expr_as(crate::current_schema(), "current_schema")
        .expr_as(crate::current_database(), "current_database")
        .filter(crate::isfinite(crate::current_timestamp()))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT to_char(CURRENT_TIMESTAMP, $1) AS \"month\", to_date($2, $3) AS \"parsed_date\", to_timestamp($4, $5) AS \"parsed_ts\", to_number($6, $7) AS \"parsed_number\", date_bin($8, CURRENT_TIMESTAMP, $9) AS \"bucket\", octet_length(\"email_address\") AS \"email_bytes\", initcap(\"email_address\") AS \"email_title\", encode($10, $11) AS \"encoded\", decode($12, $13) AS \"decoded\", ascii($14) AS \"ascii_a\", chr($15) AS \"chr_a\", CURRENT_USER AS \"current_user\", SESSION_USER AS \"session_user\", CURRENT_SCHEMA AS \"current_schema\", current_database() AS \"current_database\" FROM \"public\".\"app_users\" WHERE isfinite(CURRENT_TIMESTAMP) IS TRUE"
    );
    assert_eq!(built.params.len(), 15);
}

#[test]
fn json_symmetry_helpers_render_without_raw_sql() {
    let built = select(orders())
        .expr_as(
            crate::json_build_object(vec![
                param("id".to_owned()),
                ORDER_USER_ID.expr(),
                param("total".to_owned()),
                TOTAL.expr(),
            ]),
            "json_obj",
        )
        .expr_as(
            crate::json_build_array(vec![ORDER_USER_ID.expr(), TOTAL.expr()]),
            "json_arr",
        )
        .expr_as(crate::json_object(array(["id", "42"])), "json_object")
        .expr_as(crate::jsonb_typeof(PAYLOAD), "json_type")
        .expr_as(
            crate::json_array_length(crate::json_build_array([TOTAL])),
            "json_len",
        )
        .expr_as(
            crate::jsonb_array_length(crate::jsonb_build_array([TOTAL])),
            "jsonb_len",
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT json_build_object($1, \"user_id\", $2, \"total_cents\") AS \"json_obj\", json_build_array(\"user_id\", \"total_cents\") AS \"json_arr\", json_object(ARRAY[$3, $4]) AS \"json_object\", jsonb_typeof(\"payload\") AS \"json_type\", json_array_length(json_build_array(\"total_cents\")) AS \"json_len\", jsonb_array_length(jsonb_build_array(\"total_cents\")) AS \"jsonb_len\" FROM \"public\".\"orders\""
    );
    assert_eq!(built.params.len(), 4);
}

#[test]
fn postgres_18_function_helpers_render_without_raw_sql() {
    let built = select(users())
        .expr_as(crate::casefold(EMAIL), "folded_email")
        .expr_as(crate::normalize_form(EMAIL, "NFC"), "normalized_email")
        .expr_as(crate::gamma(ID), "gamma_id")
        .expr_as(crate::lgamma(ID), "lgamma_id")
        .expr_as(crate::crc32(param(vec![1_u8, 2, 3])), "crc")
        .expr_as(crate::uuidv4(), "uuid_v4")
        .expr_as(crate::gen_random_uuid(), "random_uuid")
        .expr_as(
            crate::uuidv7_shift(param("1 hour".to_owned()).cast("interval")),
            "shifted_uuid_v7",
        )
        .filter(crate::starts_with(EMAIL, "egor"))
        .filter(crate::unicode_assigned(EMAIL))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT casefold(\"email_address\") AS \"folded_email\", normalize(\"email_address\", NFC) AS \"normalized_email\", gamma(\"id\") AS \"gamma_id\", lgamma(\"id\") AS \"lgamma_id\", crc32($1) AS \"crc\", uuidv4() AS \"uuid_v4\", gen_random_uuid() AS \"random_uuid\", uuidv7(CAST($2 AS interval)) AS \"shifted_uuid_v7\" FROM \"public\".\"app_users\" WHERE (starts_with(\"email_address\", $3) IS TRUE AND unicode_assigned(\"email_address\") IS TRUE)"
    );
    assert_eq!(built.params.len(), 3);
}

#[test]
fn values_and_srf_sources_render_without_raw_sql() {
    static KEY_META: Meta = Meta::new("key", "key", "text").ops(OpSet::text());
    static IDX_META: Meta = Meta::new("idx", "idx", "int4").ops(OpSet::ordered());
    const KEY: Field<String> = Field::new(&KEY_META);
    const IDX: Field<i32> = Field::new(&IDX_META);

    let values = crate::values_source([(1_i32, "alpha"), (2_i32, "beta")], "input", (ID, EMAIL));
    let values_built = select(values)
        .filter(ID.at("input").gt(1_i32))
        .build()
        .unwrap();

    assert_eq!(
        values_built.sql,
        "SELECT \"input\".\"id\", \"input\".\"email_address\" AS \"email\" FROM (VALUES ($1, $2), ($3, $4)) AS \"input\" (\"id\", \"email_address\") WHERE \"input\".\"id\" > $5"
    );
    assert_eq!(values_built.params.len(), 5);

    let series = crate::generate_series_step_source(1_i32, 3_i32, 1_i32, "g", ID);
    let series_built = select(series).build().unwrap();

    assert_eq!(
        series_built.sql,
        "SELECT \"g\".\"id\" FROM generate_series($1, $2, $3) AS \"g\" (\"id\")"
    );
    assert_eq!(series_built.params.len(), 3);

    let keys = crate::jsonb_object_keys_source(
        param(serde_json::json!({"source": "sample"})),
        "keys",
        KEY,
    );
    let keys_built = select(keys).build().unwrap();

    assert_eq!(
        keys_built.sql,
        "SELECT \"keys\".\"key\" FROM jsonb_object_keys($1) AS \"keys\" (\"key\")"
    );
    assert_eq!(keys_built.params.len(), 1);

    let subscripts = crate::generate_subscripts_source(array(["a", "b"]), 1_i32, "idxs", IDX);
    let subscripts_built = select(subscripts).build().unwrap();

    assert_eq!(
        subscripts_built.sql,
        "SELECT \"idxs\".\"idx\" FROM generate_subscripts(ARRAY[$1, $2], $3) AS \"idxs\" (\"idx\")"
    );
    assert_eq!(subscripts_built.params.len(), 3);

    let parts = crate::regexp_split_to_table_source("alpha,beta", ",", "parts", KEY);
    let parts_built = select(parts).build().unwrap();

    assert_eq!(
        parts_built.sql,
        "SELECT \"parts\".\"key\" FROM regexp_split_to_table($1, $2) AS \"parts\" (\"key\")"
    );
    assert_eq!(parts_built.params.len(), 2);
}

#[test]
fn competitor_text_fts_system_and_analytics_helpers_render_without_raw_sql() {
    let built = select(users())
        .expr_as(crate::format("user:%s", [EMAIL]), "formatted")
        .expr_as(crate::translate(EMAIL, "@.", "__"), "translated")
        .expr_as(crate::repeat("*", 3_i32), "repeated")
        .expr_as(crate::width_bucket(ID, 0_i32, 100_i32, 10_i32), "bucket")
        .expr_as(crate::version(), "pg_version")
        .expr_as(
            crate::ts_headline(vec![EMAIL.into(), crate::phraseto_tsquery("hello world")]),
            "headline",
        )
        .filter(crate::starts_with(EMAIL, "egor"))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT format($1, \"email_address\") AS \"formatted\", translate(\"email_address\", $2, $3) AS \"translated\", repeat($4, $5) AS \"repeated\", width_bucket(\"id\", $6, $7, $8) AS \"bucket\", version() AS \"pg_version\", ts_headline(\"email_address\", phraseto_tsquery($9)) AS \"headline\" FROM \"public\".\"app_users\" WHERE starts_with(\"email_address\", $10) IS TRUE"
    );
    assert_eq!(built.params.len(), 10);
}

#[test]
fn json_utility_and_range_helpers_render_without_raw_sql() {
    let built = select(orders())
        .expr_as(crate::jsonb_pretty(PAYLOAD), "pretty_payload")
        .expr_as(crate::array_to_json(TAGS), "tags_json")
        .expr_as(crate::row_to_json(row((ORDER_USER_ID, TOTAL))), "row_json")
        .expr_as(crate::range_lower(SCORE_RANGE), "lower_score")
        .expr_as(crate::range_upper(SCORE_RANGE), "upper_score")
        .expr_as(
            crate::function("range_merge", [SCORE_RANGE, SCORE_RANGE]),
            "merged_score",
        )
        .expr_as(
            crate::range_merge(raw_expr("'{}'::int4multirange", Vec::<Param>::new())),
            "merged_multi",
        )
        .filter(BoolExpr::and([
            crate::isempty(SCORE_RANGE),
            crate::lower_inc(SCORE_RANGE),
            crate::upper_inc(SCORE_RANGE),
            crate::lower_inf(SCORE_RANGE),
            crate::upper_inf(SCORE_RANGE),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT jsonb_pretty(\"payload\") AS \"pretty_payload\", array_to_json(\"tags\") AS \"tags_json\", row_to_json(ROW(\"user_id\", \"total_cents\")) AS \"row_json\", lower(\"score_range\") AS \"lower_score\", upper(\"score_range\") AS \"upper_score\", range_merge(\"score_range\", \"score_range\") AS \"merged_score\", range_merge('{}'::int4multirange) AS \"merged_multi\" FROM \"public\".\"orders\" WHERE (isempty(\"score_range\") IS TRUE AND lower_inc(\"score_range\") IS TRUE AND upper_inc(\"score_range\") IS TRUE AND lower_inf(\"score_range\") IS TRUE AND upper_inf(\"score_range\") IS TRUE)"
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn array_and_aggregate_fill_in_helpers_render_without_raw_sql() {
    let array_built = select(orders())
        .expr_as(crate::array_cat(TAGS, array(["vip", "new"])), "tag_cat")
        .expr_as(crate::array_dims(TAGS), "tag_dims")
        .expr_as(crate::array_lower(TAGS, 1_i32), "tag_lower")
        .expr_as(crate::array_upper(TAGS, 1_i32), "tag_upper")
        .expr_as(crate::array_ndims(TAGS), "tag_ndims")
        .expr_as(crate::trim_array(TAGS, 1_i32), "tag_trimmed")
        .expr_as(crate::array_reverse(TAGS), "tag_reverse")
        .expr_as(crate::array_sample(TAGS, 2_i32), "tag_sample")
        .expr_as(crate::array_shuffle(TAGS), "tag_shuffle")
        .expr_as(crate::array_sort_desc(TAGS), "tag_sort_desc")
        .build()
        .unwrap();

    assert_eq!(
        array_built.sql,
        "SELECT array_cat(\"tags\", ARRAY[$1, $2]) AS \"tag_cat\", array_dims(\"tags\") AS \"tag_dims\", array_lower(\"tags\", $3) AS \"tag_lower\", array_upper(\"tags\", $4) AS \"tag_upper\", array_ndims(\"tags\") AS \"tag_ndims\", trim_array(\"tags\", $5) AS \"tag_trimmed\", array_reverse(\"tags\") AS \"tag_reverse\", array_sample(\"tags\", $6) AS \"tag_sample\", array_shuffle(\"tags\") AS \"tag_shuffle\", array_sort(\"tags\", $7, $8) AS \"tag_sort_desc\" FROM \"public\".\"orders\""
    );
    assert_eq!(array_built.params.len(), 8);

    let aggregate_built = select(orders())
        .column(ORDER_USER_ID)
        .expr_as(crate::grouping([ORDER_USER_ID]), "grouping_mask")
        .expr_as(crate::any_value(PAYLOAD), "any_payload")
        .expr_as(crate::sum_distinct(TOTAL), "distinct_total")
        .expr_as(crate::avg_distinct(TOTAL), "distinct_avg")
        .expr_as(crate::bit_xor(TOTAL), "checksum")
        .expr_as(crate::jsonb_agg_strict(PAYLOAD), "payloads")
        .expr_as(
            crate::jsonb_object_agg_unique(ORDER_USER_ID, PAYLOAD),
            "payload_by_user",
        )
        .expr_as(crate::range_agg(SCORE_RANGE), "score_ranges")
        .rollup([ORDER_USER_ID])
        .build()
        .unwrap();

    assert_eq!(
        aggregate_built.sql,
        "SELECT \"user_id\", GROUPING(\"user_id\") AS \"grouping_mask\", any_value(\"payload\") AS \"any_payload\", sum(DISTINCT \"total_cents\") AS \"distinct_total\", avg(DISTINCT \"total_cents\") AS \"distinct_avg\", bit_xor(\"total_cents\") AS \"checksum\", jsonb_agg_strict(\"payload\") AS \"payloads\", jsonb_object_agg_unique(\"user_id\", \"payload\") AS \"payload_by_user\", range_agg(\"score_range\") AS \"score_ranges\" FROM \"public\".\"orders\" GROUP BY ROLLUP(\"user_id\")"
    );
    assert_eq!(aggregate_built.params.len(), 0);
}

#[test]
fn boolean_tests_render_all_truth_variants() {
    let built = select(users())
        .filter(BoolExpr::and([
            ACTIVE.is_true(),
            ACTIVE.is_not_false(),
            ACTIVE.is_unknown(),
            ACTIVE.is_not_unknown(),
        ]))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE (\"active\" IS TRUE AND \"active\" IS NOT FALSE AND \"active\" IS UNKNOWN AND \"active\" IS NOT UNKNOWN)"
    );
}

#[test]
fn set_query_fetch_with_ties_renders_after_order_by() {
    let built = select(users())
        .column(ID)
        .union(select(users()).column(ID).filter(ID.gt(100)))
        .order_asc(ID)
        .fetch_first_with_ties(param(5_i64))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "(SELECT \"id\" FROM \"public\".\"app_users\") UNION (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"id\" > $1) ORDER BY \"id\" ASC FETCH FIRST $2 ROWS WITH TIES"
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn free_set_query_constructor_renders_raw_left_operand() {
    let built = crate::union(
        raw("SELECT ?::int4 AS id").bind(1_i32),
        select(users()).column(ID).filter(ID.gt(10)),
    )
    .build()
    .unwrap();

    assert_eq!(
        built.sql,
        "(SELECT $1::int4 AS id) UNION (SELECT \"id\" FROM \"public\".\"app_users\" WHERE \"id\" > $2)"
    );
    assert_eq!(built.params.len(), 2);
    assert!(!built.cacheable);
}

#[test]
fn cte_hints_function_sources_and_merge_render() {
    static SERIES_META: Meta = Meta::new("value", "value", "int4").ops(OpSet::ordered());
    static ORDINALITY_META: Meta =
        Meta::new("ordinality", "ordinality", "int8").ops(OpSet::ordered());
    let series = function_source(
        "generate_series",
        vec![param(1_i32), param(3_i32)],
        "g",
        vec![SERIES_META, ORDINALITY_META],
    )
    .with_ordinality();
    let generated = cte(
        "generated",
        select(series),
        vec![SERIES_META, ORDINALITY_META],
    )
    .materialized();

    let built = select(generated.source())
        .with(generated)
        .column(Field::<i32>::new(&SERIES_META))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "WITH \"generated\" (\"value\", \"ordinality\") AS MATERIALIZED (SELECT \"g\".\"value\", \"g\".\"ordinality\" FROM generate_series($1, $2) WITH ORDINALITY AS \"g\" (\"value\", \"ordinality\")) SELECT \"value\" FROM \"generated\""
    );
    assert_eq!(built.params.len(), 2);

    let merge = merge_into(
        users().alias("u"),
        orders().alias("incoming"),
        ID.at("u").eq_field(ORDER_USER_ID.at("incoming")),
    )
    .when_matched_if(TOTAL.at("incoming").gt(1000))
    .update((EMAIL.set("merged@example.com".to_owned()), ID.set(9)))
    .when_not_matched()
    .insert((ID.set(1), EMAIL.set("new@example.com".to_owned())))
    .returning_as(ID.at("u"), "id");

    let built = merge.build().unwrap();

    assert_eq!(
        built.sql,
        "MERGE INTO \"public\".\"app_users\" AS \"u\" USING \"public\".\"orders\" AS \"incoming\" ON \"u\".\"id\" = \"incoming\".\"user_id\" WHEN MATCHED AND \"incoming\".\"total_cents\" > $1 THEN UPDATE SET \"email_address\" = $2, \"id\" = $3 WHEN NOT MATCHED THEN INSERT (\"id\", \"email_address\") VALUES ($4, $5) RETURNING \"u\".\"id\" AS \"id\""
    );
    assert_eq!(built.params.len(), 5);
}
