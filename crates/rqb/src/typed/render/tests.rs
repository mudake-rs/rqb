use crate::typed::{
    Assignment, BoolExpr, Field, Insert, Meta, OpSet, Param, RawStmt, Select, SelectItem, Source,
    Stmt, ValueExpr, array_agg, count_all, count_distinct, cte, insert, lag, row_number, select,
    table, update, window,
};

static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static EMAIL_META: Meta = Meta::new("email", "email_address", "text").ops(OpSet::ordered());
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
        limit: None,
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
            expr: ValueExpr::Raw {
                sql: "?::text".to_owned(),
                params: vec![Param::typed("first".to_owned())],
            },
            alias: Some("label".to_owned()),
        }],
        filter: Some(BoolExpr::Raw {
            sql: "id > ?".to_owned(),
            params: vec![Param::typed(2_i32)],
        }),
        group_by: Vec::new(),
        having: None,
        order: Vec::new(),
        limit: None,
        offset: None,
        lock: None,
    }));

    let built = stmt.build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT $1::text AS \"label\" FROM (select $2::int4 as id) AS \"generated\" WHERE id > $3"
    );
    assert_eq!(built.params.len(), 3);
    assert!(!built.cacheable);
}

#[test]
fn insert_renders_columns_values_and_returning() {
    let insert = Insert {
        target: users(),
        columns: Vec::new(),
        assignments: vec![Assignment {
            field: EMAIL_META,
            value: ValueExpr::Param(Param::typed("egor@example.com".to_owned())),
        }],
        source: None,
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
        limit: None,
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
fn ergonomic_constructors_build_the_same_typed_ast() {
    let built = select(table("public.app_users", &USERS_FIELDS))
        .column(ID)
        .item(EMAIL.alias("email"))
        .filter(BoolExpr::and([ID.gt(10), ID.lt(20)]))
        .filter_if(false, ID.eq(999))
        .filter_option(Some("egor".to_owned()), |email| EMAIL.ne(email))
        .apply(|query| query.order_desc(ID))
        .limit(50)
        .offset(100)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE ((\"id\" > $1 AND \"id\" < $2) AND \"email_address\" <> $3) ORDER BY \"id\" DESC LIMIT $4 OFFSET $5"
    );
    assert_eq!(built.params.len(), 5);
}

#[test]
fn joins_render_qualified_fields_and_keep_param_order() {
    let built = select(users().alias("u"))
        .join(
            orders().alias("o"),
            ID.at("u").eq_field(ORDER_USER_ID.at("o")),
        )
        .column(EMAIL.at("u").alias("email"))
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
fn aliased_root_default_projection_is_qualified() {
    let built = select(users().alias("u")).build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"id\", \"u\".\"email_address\" AS \"email\" FROM \"public\".\"app_users\" AS \"u\""
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
fn distinct_group_having_and_locks_render_as_select_clauses() {
    fn count_id() -> ValueExpr {
        crate::typed::count(ID)
    }

    let built = select(users())
        .distinct_on(EMAIL)
        .column(EMAIL)
        .item(count_id().alias("user_count"))
        .group_by(EMAIL)
        .having(BoolExpr::Compare {
            left: count_id(),
            op: crate::typed::BoolOp::Gt,
            right: ValueExpr::Param(Param::typed(1_i64)),
        })
        .order_asc(EMAIL)
        .for_update()
        .skip_locked()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT DISTINCT ON (\"email_address\") \"email_address\" AS \"email\", count(\"id\") AS \"user_count\" FROM \"public\".\"app_users\" GROUP BY \"email_address\" HAVING count(\"id\") > $1 ORDER BY \"email_address\" ASC FOR UPDATE SKIP LOCKED"
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
    let source = select(users()).column(ID).column(EMAIL).filter(ID.gt(10));
    let built = insert(users())
        .column(ID)
        .column(EMAIL)
        .from_select(source)
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
        .column(ID)
        .column(EMAIL)
        .from_select(source)
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
        .set(ID.set(1))
        .set(EMAIL.set("new@example.com".to_owned()))
        .on_conflict(ID)
        .target_where(ID.gt(0))
        .do_update_set_where(
            [EMAIL.set_excluded()],
            EMAIL.ne("old@example.com".to_owned()),
        )
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        update.sql,
        "INSERT INTO \"public\".\"app_users\" (\"id\", \"email_address\") VALUES ($1, $2) ON CONFLICT (\"id\") WHERE \"id\" > $3 DO UPDATE SET \"email_address\" = EXCLUDED.\"email_address\" WHERE \"email_address\" <> $4 RETURNING \"id\""
    );
    assert_eq!(update.params.len(), 4);

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

    let invalid_and = insert(users())
        .set(ID.set(1))
        .on_conflict_constraint("app_users_pkey")
        .and(EMAIL)
        .do_nothing()
        .build()
        .unwrap_err();

    assert!(matches!(
        invalid_and,
        crate::Error::InvalidInsertShape { message }
            if message == "and requires on_conflict(column), not on_conflict_constraint"
    ));

    let invalid_target_where = insert(users())
        .set(ID.set(1))
        .on_conflict_constraint("app_users_pkey")
        .target_where(ID.gt(0))
        .do_nothing()
        .build()
        .unwrap_err();

    assert!(matches!(
        invalid_target_where,
        crate::Error::InvalidInsertShape { message }
            if message == "target_where requires on_conflict(column), not on_conflict_constraint"
    ));
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
fn recursive_cte_source_renders_columns_and_body_params() {
    let seed = select(users()).column(ID).filter(ID.eq(1));
    let recursive_arm = select(crate::typed::cte_source("active_users", vec![ID_META]))
        .column(ID)
        .filter(ID.lt(10));
    let active_users = cte("active_users", seed.union_all(recursive_arm), vec![ID_META])
        .columns(["id"])
        .recursive();

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
fn joined_cte_definitions_render_before_select_and_keep_params() {
    let big_orders = cte(
        "big_orders",
        select(orders())
            .column(ORDER_USER_ID)
            .filter(TOTAL.gt(1000)),
        vec![ORDER_USER_ID_META],
    )
    .columns(["user_id"]);
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
fn cte_column_aliases_must_match_exposed_fields() {
    let invalid = cte(
        "broken",
        select(users()).column(ID),
        vec![ID_META, EMAIL_META],
    )
    .columns(["id"]);

    let err = select(invalid.source()).with(invalid).build().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidCteShape { name, .. } if name == "broken"
    ));
}

#[test]
fn window_helpers_render_over_partition_and_order_specs() {
    let built = select(users())
        .column(ID)
        .item(
            row_number()
                .over(window().partition_by(EMAIL).order_desc(ID))
                .alias("row_no"),
        )
        .item(
            lag(EMAIL)
                .offset(ValueExpr::Param(Param::typed(2_i32)))
                .over(window().order_asc(ID))
                .alias("previous_email"),
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
fn aggregate_helpers_render_common_postgres_aggregates() {
    let built = select(users())
        .item(count_all().alias("total"))
        .item(count_distinct(EMAIL).alias("unique_emails"))
        .item(
            array_agg(EMAIL)
                .aggregate_order_desc(ID)
                .aggregate_filter(ID.gt(10))
                .alias("emails"),
        )
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT count(*) AS \"total\", count(DISTINCT \"email_address\") AS \"unique_emails\", array_agg(\"email_address\" ORDER BY \"id\" DESC) FILTER (WHERE \"id\" > $1) AS \"emails\" FROM \"public\".\"app_users\""
    );
    assert_eq!(built.params.len(), 1);
}

#[test]
fn array_and_json_predicates_render_without_raw_sql() {
    let built = select(orders())
        .filter(BoolExpr::and([
            TAGS.contains_all(vec!["paid".to_owned(), "vip".to_owned()]),
            TAGS.has("urgent".to_owned()),
            TAGS.is_not_empty(),
            PAYLOAD.key_exists("source"),
            PAYLOAD.keys_exist_any(vec!["card".to_owned(), "bank".to_owned()]),
            PAYLOAD.json_contains(serde_json::json!({ "channel": "web" })),
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
