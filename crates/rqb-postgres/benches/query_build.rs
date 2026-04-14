use diesel::prelude::*;
use diesel::{debug_query, pg::Pg};
use divan::Bencher;
use rqb_core::{
    Dataset, ElemType, Field, FieldType, JsonPathPolicy, SearchRequest, SelectQuery, Sort, all,
    any, field, not, raw_query, select,
};
use rqb_postgres::{BuildPostgres, BuildRowsPostgres, BuiltQuery};
use sea_query::{Expr as SeaExpr, ExprTrait as _, Order, PostgresQueryBuilder, Query};
use sqlx::{Execute, Postgres, QueryBuilder};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

diesel::table! {
    order_search_view (id) {
        id -> Text,
        email -> Text,
        status -> Text,
        created_at -> Timestamptz,
        total_cents -> BigInt,
        tags -> Nullable<Array<Text>>,
    }
}

const ID: Field = Field::new("id", FieldType::Uuid);
const EMAIL: Field = Field::new("email", FieldType::Text);
const STATUS: Field = Field::new("status", FieldType::Text);
const NAME: Field = Field::new("name", FieldType::Text).text_search("english");
const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
const TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
    .sortable(false)
    .json_paths(JsonPathPolicy::Dynamic);

const ORDER_FIELDS: &[Field] = &[
    ID,
    EMAIL,
    STATUS,
    NAME,
    CREATED_AT,
    TOTAL_CENTS,
    TAGS,
    METADATA,
];

fn main() {
    divan::main();
}

fn orders() -> Dataset {
    Dataset::static_view("order_search_view")
        .static_fields(ORDER_FIELDS)
        .max_limit(500)
}

fn rqb_simple_select_query() -> SelectQuery {
    select(orders())
        .fields(["id", "email", "createdAt"])
        .filter(all([
            field("status").eq("paid"),
            field("totalCents").gte(1_000_i64),
            field("email").starts_with("a"),
        ]))
        .order_by(Sort::desc("createdAt"))
        .limit(20)
        .build()
}

fn rqb_simple_select_typed_query() -> SelectQuery {
    select(orders())
        .fields([ID, EMAIL, CREATED_AT])
        .filter(all([
            STATUS.eq("paid"),
            TOTAL_CENTS.gte(1_000_i64),
            EMAIL.starts_with("a"),
        ]))
        .order_by(CREATED_AT.desc())
        .limit(20)
        .build()
}

fn rqb_nested_dynamic_query() -> SelectQuery {
    select(orders())
        .fields(["id", "email"])
        .filter(all([
            any([
                field("status").eq("paid"),
                field("status").eq("draft"),
                field("totalCents").gte(10_000_i64),
            ]),
            not(any([field("email").is_null(), field("tags").is_null()])),
            field("metadata.campaign").eq("spring"),
        ]))
        .order_by(Sort::desc("createdAt"))
        .limit(50)
        .build()
}

#[divan::bench]
fn rqb_dataset_metadata() -> Dataset {
    orders()
}

#[divan::bench]
fn rqb_simple_select_build_ast() -> SelectQuery {
    rqb_simple_select_query()
}

#[divan::bench]
fn rqb_simple_select_typed_build_ast() -> SelectQuery {
    rqb_simple_select_typed_query()
}

#[divan::bench]
fn rqb_simple_select_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(rqb_simple_select_query)
        .bench_values(|query| query.build_rows_pg().unwrap());
}

#[divan::bench]
fn rqb_simple_select_typed_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(rqb_simple_select_typed_query)
        .bench_values(|query| query.build_rows_pg().unwrap());
}

#[divan::bench]
fn rqb_simple_select() -> BuiltQuery {
    rqb_simple_select_query().build_rows_pg().unwrap()
}

#[divan::bench]
fn rqb_simple_select_typed() -> BuiltQuery {
    rqb_simple_select_typed_query().build_rows_pg().unwrap()
}

#[divan::bench]
fn sea_query_simple_select() -> (String, sea_query::Values) {
    Query::select()
        .columns(["id", "email", "created_at"])
        .from("order_search_view")
        .and_where(SeaExpr::col("status").eq("paid"))
        .and_where(SeaExpr::col("total_cents").gte(1_000_i64))
        .and_where(SeaExpr::col("email").like("a%"))
        .order_by("created_at", Order::Desc)
        .limit(20)
        .build(PostgresQueryBuilder)
}

#[divan::bench]
fn sqlx_query_builder_simple_select() -> String {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT id, email, created_at FROM order_search_view WHERE status = ",
    );
    query
        .push_bind("paid")
        .push(" AND total_cents >= ")
        .push_bind(1_000_i64)
        .push(" AND email LIKE ")
        .push_bind("a%")
        .push(" ORDER BY created_at DESC LIMIT 20");

    query.build().sql().to_owned()
}

#[divan::bench]
fn diesel_debug_simple_select() -> String {
    use crate::order_search_view::dsl::*;

    let query = order_search_view
        .select((id, email, created_at))
        .filter(status.eq("paid"))
        .filter(total_cents.ge(1_000_i64))
        .filter(email.like("a%"))
        .order(created_at.desc())
        .limit(20);

    debug_query::<Pg, _>(&query).to_string()
}

#[divan::bench]
fn rqb_nested_dynamic_filter() -> BuiltQuery {
    rqb_nested_dynamic_query().build_rows_pg().unwrap()
}

#[divan::bench]
fn rqb_nested_dynamic_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(rqb_nested_dynamic_query)
        .bench_values(|query| query.build_rows_pg().unwrap());
}

#[divan::bench]
fn sea_query_nested_dynamic_filter() -> (String, sea_query::Values) {
    Query::select()
        .columns(["id", "email"])
        .from("order_search_view")
        .cond_where(sea_query::all![
            sea_query::any![
                SeaExpr::col("status").eq("paid"),
                SeaExpr::col("status").eq("draft"),
                SeaExpr::col("total_cents").gte(10_000_i64),
            ],
            sea_query::Expr::expr(sea_query::any![
                SeaExpr::col("email").is_null(),
                SeaExpr::col("tags").is_null(),
            ])
            .not(),
            SeaExpr::cust(r#""metadata" #>> ARRAY['campaign'] = 'spring'"#),
        ])
        .order_by("created_at", Order::Desc)
        .limit(50)
        .build(PostgresQueryBuilder)
}

#[divan::bench]
fn sqlx_query_builder_nested_dynamic_filter() -> String {
    let mut query =
        QueryBuilder::<Postgres>::new("SELECT id, email FROM order_search_view WHERE (status = ");
    query
        .push_bind("paid")
        .push(" OR status = ")
        .push_bind("draft")
        .push(" OR total_cents >= ")
        .push_bind(10_000_i64)
        .push(") AND NOT (email IS NULL OR tags IS NULL)")
        .push(" AND metadata #>> ARRAY['campaign'] = ")
        .push_bind("spring")
        .push(" ORDER BY created_at DESC LIMIT 50");

    query.build().sql().to_owned()
}

#[divan::bench]
fn diesel_debug_nested_static_filter() -> String {
    use crate::order_search_view::dsl::*;

    let query = order_search_view
        .select((id, email))
        .filter(
            status
                .eq("paid")
                .or(status.eq("draft"))
                .or(total_cents.ge(10_000_i64))
                .and(diesel::dsl::not(email.is_null().or(tags.is_null()))),
        )
        .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
            r#"metadata #>> ARRAY['campaign'] = 'spring'"#,
        ))
        .order(created_at.desc())
        .limit(50);

    debug_query::<Pg, _>(&query).to_string()
}

#[divan::bench]
fn rqb_raw_query_build() -> BuiltQuery {
    raw_query("SELECT id, email FROM order_search_view WHERE status = ? AND total_cents >= ?")
        .bind("paid")
        .bind(1_000_i64)
        .build_pg()
        .unwrap()
}

#[divan::bench]
fn rqb_json_search_request(bencher: Bencher) {
    let request = SearchRequest {
        fields: vec![field("id"), field("email"), field("createdAt")],
        sort: vec![field("createdAt").desc()],
        limit: Some(20),
        filter: Some(all([
            field("status").eq("paid"),
            field("metadata.score").gte(70),
            field("email").contains("@example.com"),
        ])),
        ..SearchRequest::new()
    };

    bencher.bench(|| {
        select(orders())
            .request(request.clone())
            .build_rows_pg()
            .unwrap()
    });
}
