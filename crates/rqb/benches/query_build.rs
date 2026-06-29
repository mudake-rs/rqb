use std::hint::black_box;

use chrono::{DateTime, Utc};
use divan::Bencher;
use rqb::prelude::*;
use rqb::{OrderItem, dsl::json_agg};
use serde_json::json;
use uuid::Uuid;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

static USER_ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static USER_EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::text());
static USER_ACTIVE_META: Meta = Meta::new("active", "active", "bool").ops(OpSet::equality());
static USER_CREATED_AT_META: Meta =
    Meta::new("created_at", "created_at", "timestamptz").ops(OpSet::ordered());
static ORDER_ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static ORDER_USER_ID_META: Meta = Meta::new("user_id", "user_id", "int4").ops(OpSet::ordered());
static ORDER_STATUS_META: Meta = Meta::new("status", "status", "text").ops(OpSet::text());
static ORDER_TOTAL_META: Meta =
    Meta::new("total_cents", "total_cents", "int8").ops(OpSet::ordered());

static OLD_ID_META: Meta = Meta::new("id", "id", "uuid")
    .ops(OpSet::ordered())
    .json(JsonKind::Uuid);
static OLD_EMAIL_META: Meta = Meta::new("email", "email", "text")
    .ops(OpSet::text())
    .json(JsonKind::Text);
static OLD_STATUS_META: Meta = Meta::new("status", "status", "text")
    .ops(OpSet::text())
    .json(JsonKind::Text);
static OLD_NAME_META: Meta = Meta::new("name", "name", "text")
    .ops(OpSet::text())
    .json(JsonKind::Text);
static OLD_CREATED_AT_META: Meta = Meta::new("createdAt", "created_at", "timestamptz")
    .ops(OpSet::ordered())
    .json(JsonKind::Timestamptz);
static OLD_TOTAL_CENTS_META: Meta = Meta::new("totalCents", "total_cents", "int8")
    .ops(OpSet::ordered())
    .json(JsonKind::BigInt);
static OLD_TAGS_META: Meta = Meta::new("tags", "tags", "text[]").ops(OpSet::equality());
static OLD_METADATA_META: Meta = Meta::new("metadata", "metadata", "jsonb").json(JsonKind::Jsonb);

macro_rules! wide_text_meta {
    ($name:ident, $api:literal, $db:literal) => {
        static $name: Meta = Meta::new($api, $db, "text")
            .ops(OpSet::text())
            .json(JsonKind::Text);
    };
}

macro_rules! plain_text_meta {
    ($name:ident, $db:literal) => {
        static $name: Meta = Meta::new($db, $db, "text")
            .ops(OpSet::text())
            .json(JsonKind::Text);
    };
}

wide_text_meta!(NARROW_00_META, "field00", "field_00");
wide_text_meta!(NARROW_01_META, "field01", "field_01");
wide_text_meta!(NARROW_02_META, "field02", "field_02");
wide_text_meta!(NARROW_03_META, "field03", "field_03");

wide_text_meta!(WIDE_00_META, "field00", "field_00");
wide_text_meta!(WIDE_01_META, "field01", "field_01");
wide_text_meta!(WIDE_02_META, "field02", "field_02");
wide_text_meta!(WIDE_03_META, "field03", "field_03");
wide_text_meta!(WIDE_04_META, "field04", "field_04");
wide_text_meta!(WIDE_05_META, "field05", "field_05");
wide_text_meta!(WIDE_06_META, "field06", "field_06");
wide_text_meta!(WIDE_07_META, "field07", "field_07");
wide_text_meta!(WIDE_08_META, "field08", "field_08");
wide_text_meta!(WIDE_09_META, "field09", "field_09");
wide_text_meta!(WIDE_10_META, "field10", "field_10");
wide_text_meta!(WIDE_11_META, "field11", "field_11");
wide_text_meta!(WIDE_12_META, "field12", "field_12");
wide_text_meta!(WIDE_13_META, "field13", "field_13");
wide_text_meta!(WIDE_14_META, "field14", "field_14");
wide_text_meta!(WIDE_15_META, "field15", "field_15");
wide_text_meta!(WIDE_16_META, "field16", "field_16");
wide_text_meta!(WIDE_17_META, "field17", "field_17");
wide_text_meta!(WIDE_18_META, "field18", "field_18");
wide_text_meta!(WIDE_19_META, "field19", "field_19");

plain_text_meta!(PLAIN_00_META, "field_00");
plain_text_meta!(PLAIN_01_META, "field_01");
plain_text_meta!(PLAIN_02_META, "field_02");
plain_text_meta!(PLAIN_03_META, "field_03");
plain_text_meta!(PLAIN_04_META, "field_04");
plain_text_meta!(PLAIN_05_META, "field_05");
plain_text_meta!(PLAIN_06_META, "field_06");
plain_text_meta!(PLAIN_07_META, "field_07");
plain_text_meta!(PLAIN_08_META, "field_08");
plain_text_meta!(PLAIN_09_META, "field_09");
plain_text_meta!(PLAIN_10_META, "field_10");
plain_text_meta!(PLAIN_11_META, "field_11");
plain_text_meta!(PLAIN_12_META, "field_12");
plain_text_meta!(PLAIN_13_META, "field_13");
plain_text_meta!(PLAIN_14_META, "field_14");
plain_text_meta!(PLAIN_15_META, "field_15");
plain_text_meta!(PLAIN_16_META, "field_16");
plain_text_meta!(PLAIN_17_META, "field_17");
plain_text_meta!(PLAIN_18_META, "field_18");
plain_text_meta!(PLAIN_19_META, "field_19");

static USER_FIELDS: [&Meta; 4] = [
    &USER_ID_META,
    &USER_EMAIL_META,
    &USER_ACTIVE_META,
    &USER_CREATED_AT_META,
];
static ORDER_FIELDS: [&Meta; 4] = [
    &ORDER_ID_META,
    &ORDER_USER_ID_META,
    &ORDER_STATUS_META,
    &ORDER_TOTAL_META,
];
static OLD_ORDER_FIELDS: [&Meta; 8] = [
    &OLD_ID_META,
    &OLD_EMAIL_META,
    &OLD_STATUS_META,
    &OLD_NAME_META,
    &OLD_CREATED_AT_META,
    &OLD_TOTAL_CENTS_META,
    &OLD_TAGS_META,
    &OLD_METADATA_META,
];
static NARROW_FIELDS: [&Meta; 4] = [
    &NARROW_00_META,
    &NARROW_01_META,
    &NARROW_02_META,
    &NARROW_03_META,
];
static WIDE_FIELDS: [&Meta; 20] = [
    &WIDE_00_META,
    &WIDE_01_META,
    &WIDE_02_META,
    &WIDE_03_META,
    &WIDE_04_META,
    &WIDE_05_META,
    &WIDE_06_META,
    &WIDE_07_META,
    &WIDE_08_META,
    &WIDE_09_META,
    &WIDE_10_META,
    &WIDE_11_META,
    &WIDE_12_META,
    &WIDE_13_META,
    &WIDE_14_META,
    &WIDE_15_META,
    &WIDE_16_META,
    &WIDE_17_META,
    &WIDE_18_META,
    &WIDE_19_META,
];
static PLAIN_FIELDS: [&Meta; 20] = [
    &PLAIN_00_META,
    &PLAIN_01_META,
    &PLAIN_02_META,
    &PLAIN_03_META,
    &PLAIN_04_META,
    &PLAIN_05_META,
    &PLAIN_06_META,
    &PLAIN_07_META,
    &PLAIN_08_META,
    &PLAIN_09_META,
    &PLAIN_10_META,
    &PLAIN_11_META,
    &PLAIN_12_META,
    &PLAIN_13_META,
    &PLAIN_14_META,
    &PLAIN_15_META,
    &PLAIN_16_META,
    &PLAIN_17_META,
    &PLAIN_18_META,
    &PLAIN_19_META,
];

const USER_ID: Field<i32> = Field::new(&USER_ID_META);
const USER_EMAIL: Field<String> = Field::new(&USER_EMAIL_META);
const USER_ACTIVE: Field<bool> = Field::new(&USER_ACTIVE_META);
const USER_CREATED_AT: Field<String> = Field::new(&USER_CREATED_AT_META);
const ORDER_ID: Field<i32> = Field::new(&ORDER_ID_META);
const ORDER_USER_ID: Field<i32> = Field::new(&ORDER_USER_ID_META);
const ORDER_STATUS: Field<String> = Field::new(&ORDER_STATUS_META);
const ORDER_TOTAL: Field<i64> = Field::new(&ORDER_TOTAL_META);
const OLD_ID: Field<Uuid> = Field::new(&OLD_ID_META);
const OLD_EMAIL: Field<String> = Field::new(&OLD_EMAIL_META);
const OLD_STATUS: Field<String> = Field::new(&OLD_STATUS_META);
const OLD_CREATED_AT: Field<DateTime<Utc>> = Field::new(&OLD_CREATED_AT_META);
const OLD_TOTAL_CENTS: Field<i64> = Field::new(&OLD_TOTAL_CENTS_META);
const OLD_TAGS: Field<Vec<String>> = Field::new(&OLD_TAGS_META);

fn main() {
    divan::main();
}

fn users() -> Source {
    table("public.app_users", &USER_FIELDS)
}

fn orders() -> Source {
    table("public.orders", &ORDER_FIELDS)
}

fn old_order_search() -> Source {
    view("order_search_view", &OLD_ORDER_FIELDS)
}

fn narrow_search() -> Source {
    view("narrow_search_view", &NARROW_FIELDS).alias("n")
}

fn wide_search() -> Source {
    view("wide_search_view", &WIDE_FIELDS).alias("w")
}

fn plain_projection_source() -> Source {
    view("plain_projection_view", &PLAIN_FIELDS).alias("p")
}

fn old_simple_select_typed_query() -> Select {
    select(old_order_search())
        .column(OLD_ID)
        .column(OLD_EMAIL)
        .column(OLD_CREATED_AT)
        .filter(BoolExpr::and([
            OLD_STATUS.eq(black_box("paid")),
            OLD_TOTAL_CENTS.gte(black_box(1_000_i64)),
            OLD_EMAIL.starts_with(black_box("a")),
        ]))
        .order_desc(OLD_CREATED_AT)
        .limit(black_box(20))
}

fn old_nested_typed_query() -> Select {
    select(old_order_search())
        .column(OLD_ID)
        .column(OLD_EMAIL)
        .filter(BoolExpr::and([
            BoolExpr::or([
                OLD_STATUS.eq(black_box("paid")),
                OLD_STATUS.eq(black_box("draft")),
                OLD_TOTAL_CENTS.gte(black_box(10_000_i64)),
            ]),
            BoolExpr::negate(BoolExpr::or([OLD_EMAIL.is_null(), OLD_TAGS.is_null()])),
        ]))
        .order_desc(OLD_CREATED_AT)
        .limit(black_box(50))
}

fn select_join_json_aggregate_query() -> Select {
    select(users().alias("u"))
        .left_join(
            orders().alias("o"),
            USER_ID.at("u").eq_field(ORDER_USER_ID.at("o")),
        )
        .column(USER_ID.at("u"))
        .column(USER_EMAIL.at("u"))
        .expr_as(
            json_agg(ORDER_ID.at("o"))
                .aggregate_order_desc(ORDER_ID.at("o"))
                .aggregate_filter(ORDER_STATUS.at("o").eq(black_box("paid"))),
            "paid_order_ids",
        )
        .filter(USER_ACTIVE.at("u").eq(black_box(true)))
        .filter(ORDER_TOTAL.at("o").gte(black_box(5_000_i64)))
        .group_by(USER_ID.at("u"))
        .group_by(USER_EMAIL.at("u"))
}

fn qualified_field_refs_query() -> Select {
    select(users().alias("u"))
        .column(USER_ID.at("u"))
        .column(USER_EMAIL.at("u"))
        .filter(USER_ACTIVE.at("u").eq(black_box(true)))
        .filter(
            USER_CREATED_AT
                .at("u")
                .gte(black_box("2026-01-01".to_owned())),
        )
        .group_by(USER_ID.at("u"))
        .group_by(USER_EMAIL.at("u"))
}

fn unqualified_field_refs_query() -> Select {
    select(users())
        .column(USER_ID)
        .column(USER_EMAIL)
        .filter(USER_ACTIVE.eq(black_box(true)))
        .filter(USER_CREATED_AT.gte(black_box("2026-01-01".to_owned())))
        .group_by(USER_ID)
        .group_by(USER_EMAIL)
}

fn select_columns_from(source: Source, fields: &'static [&'static Meta]) -> Select {
    let mut query = select(source);
    for field in fields {
        query = query.column(*field);
    }
    query
}

fn make_old_json_search_request() -> SearchRequest {
    SearchRequest {
        filter: Some(SearchFilter::And(vec![
            SearchFilter::Predicate(SearchPredicate {
                field: "status".to_owned(),
                operator: SearchOperator::Equals,
                value: json!("paid"),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "totalCents".to_owned(),
                operator: SearchOperator::Gte,
                value: json!(1_000),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "email".to_owned(),
                operator: SearchOperator::Contains,
                value: json!("@example.com"),
            }),
        ])),
        sort: vec![SearchSort {
            field: "createdAt".to_owned(),
            dir: SortDirection::Desc,
        }],
        limit: Some(20),
        offset: None,
    }
}

fn make_wide_json_search_request() -> SearchRequest {
    SearchRequest {
        filter: Some(SearchFilter::And(vec![
            SearchFilter::Predicate(SearchPredicate {
                field: "field03".to_owned(),
                operator: SearchOperator::Contains,
                value: json!("alpha"),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "field07".to_owned(),
                operator: SearchOperator::StartsWith,
                value: json!("bravo"),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "field11".to_owned(),
                operator: SearchOperator::EndsWith,
                value: json!("charlie"),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "field13".to_owned(),
                operator: SearchOperator::ILike,
                value: json!("%delta%"),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "field17".to_owned(),
                operator: SearchOperator::Equals,
                value: json!("echo"),
            }),
            SearchFilter::Predicate(SearchPredicate {
                field: "field19".to_owned(),
                operator: SearchOperator::NotEquals,
                value: json!("foxtrot"),
            }),
        ])),
        sort: vec![
            SearchSort {
                field: "field05".to_owned(),
                dir: SortDirection::Asc,
            },
            SearchSort {
                field: "field15".to_owned(),
                dir: SortDirection::Desc,
            },
        ],
        limit: Some(50),
        offset: Some(100),
    }
}

fn search_request_one_eq(field: &str) -> SearchRequest {
    SearchRequest {
        filter: Some(SearchFilter::Predicate(SearchPredicate {
            field: field.to_owned(),
            operator: SearchOperator::Equals,
            value: json!("value"),
        })),
        sort: Vec::new(),
        limit: None,
        offset: None,
    }
}

fn search_request_page_only() -> SearchRequest {
    SearchRequest {
        filter: None,
        sort: Vec::new(),
        limit: Some(50),
        offset: Some(100),
    }
}

fn search_request_sort_only() -> SearchRequest {
    SearchRequest {
        filter: None,
        sort: vec![
            SearchSort {
                field: "field05".to_owned(),
                dir: SortDirection::Asc,
            },
            SearchSort {
                field: "field15".to_owned(),
                dir: SortDirection::Desc,
            },
        ],
        limit: None,
        offset: None,
    }
}

fn search_request_wide_filter_only() -> SearchRequest {
    let mut request = make_wide_json_search_request();
    request.sort = Vec::new();
    request.limit = None;
    request.offset = None;
    request
}

#[divan::bench]
fn old_simple_select_typed_build_ast() -> Select {
    old_simple_select_typed_query()
}

#[divan::bench]
fn old_simple_select_typed_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(old_simple_select_typed_query)
        .bench_values(|query| query.build().unwrap());
}

#[divan::bench]
fn old_simple_select_typed() -> BuiltQuery {
    old_simple_select_typed_query().build().unwrap()
}

#[divan::bench]
fn old_nested_typed_filter() -> BuiltQuery {
    old_nested_typed_query().build().unwrap()
}

#[divan::bench]
fn old_nested_typed_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(old_nested_typed_query)
        .bench_values(|query| query.build().unwrap());
}

#[divan::bench]
fn old_json_search_request(bencher: Bencher) {
    let request = make_old_json_search_request();

    bencher.bench(|| {
        select(old_order_search())
            .apply_search(black_box(request.clone()))
            .unwrap()
            .build()
            .unwrap()
    });
}

#[divan::bench]
fn phase_validate_prebuilt_select(bencher: Bencher) {
    bencher
        .with_inputs(select_join_json_aggregate_query)
        .bench_values(|query| query.validate().unwrap());
}

#[divan::bench]
fn phase_search_merge_small(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(old_order_search()), make_old_json_search_request()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_empty_narrow(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(narrow_search()), SearchRequest::default()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_empty_wide(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), SearchRequest::default()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_narrow_one_predicate(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(narrow_search()), search_request_one_eq("field00")))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_wide_one_predicate_first_field(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), search_request_one_eq("field00")))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

// Position probes: allocations should match; only the noisy timing delta can
// show the linear scan cost across source fields.
#[divan::bench]
fn phase_search_merge_wide_one_predicate_last_field(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), search_request_one_eq("field19")))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_wide_page_only(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), search_request_page_only()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_wide_sort_only(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), search_request_sort_only()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_wide_filter_only(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), search_request_wide_filter_only()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_search_merge_wide(bencher: Bencher) {
    bencher
        .with_inputs(|| (select(wide_search()), make_wide_json_search_request()))
        .bench_values(|(select, request)| select.apply_search(request).unwrap());
}

#[divan::bench]
fn phase_built_query_arguments(bencher: Bencher) {
    let built = select_join_json_aggregate_query().build().unwrap();

    bencher.bench(|| black_box(&built).arguments().unwrap());
}

#[divan::bench]
fn phase_built_query_clone(bencher: Bencher) {
    let built = select_join_json_aggregate_query().build().unwrap();

    bencher.bench(|| black_box(built.clone()));
}

#[divan::bench]
fn phase_sql_literal_render() -> BuiltQuery {
    select(users())
        .expr(rqb::dsl::date_trunc_part(
            rqb::dsl::DatePart::Day,
            rqb::dsl::current_timestamp(),
        ))
        .group_by(rqb::dsl::date_trunc_part(
            rqb::dsl::DatePart::Day,
            rqb::dsl::current_timestamp(),
        ))
        .order_by(OrderItem::asc(rqb::dsl::date_trunc_part(
            rqb::dsl::DatePart::Day,
            rqb::dsl::current_timestamp(),
        )))
        .build()
        .unwrap()
}

#[divan::bench]
fn phase_join_json_aggregate_build_ast() -> Select {
    select_join_json_aggregate_query()
}

#[divan::bench]
fn phase_join_json_aggregate_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(select_join_json_aggregate_query)
        .bench_values(|query| query.build().unwrap());
}

#[divan::bench]
fn phase_qualified_field_refs_build_ast() -> Select {
    qualified_field_refs_query()
}

#[divan::bench]
fn phase_unqualified_field_refs_build_ast() -> Select {
    unqualified_field_refs_query()
}

#[divan::bench]
fn phase_projection_default_renamed_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(|| select(wide_search()))
        .bench_values(|query| query.build().unwrap());
}

#[divan::bench]
fn phase_projection_default_plain_render_prebuilt(bencher: Bencher) {
    bencher
        .with_inputs(|| select(plain_projection_source()))
        .bench_values(|query| query.build().unwrap());
}

#[divan::bench]
fn phase_projection_explicit_renamed_build_ast() -> Select {
    select_columns_from(wide_search(), &WIDE_FIELDS)
}

#[divan::bench]
fn phase_projection_explicit_plain_build_ast() -> Select {
    select_columns_from(plain_projection_source(), &PLAIN_FIELDS)
}

#[divan::bench]
fn old_raw_query_build() -> BuiltQuery {
    raw("SELECT id, email FROM order_search_view WHERE status = ? AND total_cents >= ?")
        .bind(black_box("paid"))
        .bind(black_box(1_000_i64))
        .build()
        .unwrap()
}

#[divan::bench]
fn raw_no_placeholders() -> BuiltQuery {
    raw("SELECT id, email FROM order_search_view WHERE status = 'paid' ORDER BY id")
        .build()
        .unwrap()
}

#[divan::bench]
fn select_filtered_ordered() -> BuiltQuery {
    select(users())
        .column(USER_ID)
        .column(USER_EMAIL)
        .filter(USER_ACTIVE.eq(black_box(true)))
        .filter(USER_EMAIL.contains(black_box("@example.com")))
        .order_desc(USER_CREATED_AT)
        .limit(black_box(50))
        .offset(black_box(100))
        .build()
        .unwrap()
}

#[divan::bench]
fn select_join_json_aggregate() -> BuiltQuery {
    select_join_json_aggregate_query().build().unwrap()
}

#[divan::bench]
fn insert_on_conflict_update() -> BuiltQuery {
    insert(users())
        .set(USER_ID.set(black_box(42_i32)))
        .set(USER_EMAIL.set(black_box("user@example.com".to_owned())))
        .set(USER_ACTIVE.set(black_box(true)))
        .on_conflict(USER_ID)
        .do_update_set([
            USER_EMAIL.set(black_box("user@example.com".to_owned())),
            USER_ACTIVE.set(black_box(true)),
        ])
        .returning(USER_ID)
        .build()
        .unwrap()
}

#[divan::bench]
fn raw_placeholder_rewrite() -> BuiltQuery {
    raw("SELECT ?::int4 AS id, ?::text AS email, ?::bool AS active, ?? AS literal_q")
        .bind(black_box(42_i32))
        .bind(black_box("user@example.com".to_owned()))
        .bind(black_box(true))
        .build()
        .unwrap()
}
