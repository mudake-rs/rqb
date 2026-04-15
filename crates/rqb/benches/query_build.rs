use std::hint::black_box;

use divan::Bencher;
use rqb::chrono::{DateTime, Utc};
use rqb::prelude::*;
use rqb::serde_json::json;
use rqb::uuid::Uuid;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

static USER_ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static USER_EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
static USER_ACTIVE_META: Meta = Meta::new("active", "active", "bool").ops(OpSet::equality());
static USER_CREATED_AT_META: Meta =
    Meta::new("created_at", "created_at", "timestamptz").ops(OpSet::ordered());
static ORDER_ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static ORDER_USER_ID_META: Meta = Meta::new("user_id", "user_id", "int4").ops(OpSet::ordered());
static ORDER_STATUS_META: Meta = Meta::new("status", "status", "text").ops(OpSet::ordered());
static ORDER_TOTAL_META: Meta =
    Meta::new("total_cents", "total_cents", "int8").ops(OpSet::ordered());

static OLD_ID_META: Meta = Meta::new("id", "id", "uuid")
    .ops(OpSet::ordered())
    .json(JsonKind::Uuid);
static OLD_EMAIL_META: Meta = Meta::new("email", "email", "text")
    .ops(OpSet::ordered())
    .json(JsonKind::Text);
static OLD_STATUS_META: Meta = Meta::new("status", "status", "text")
    .ops(OpSet::ordered())
    .json(JsonKind::Text);
static OLD_NAME_META: Meta = Meta::new("name", "name", "text")
    .ops(OpSet::ordered())
    .json(JsonKind::Text);
static OLD_CREATED_AT_META: Meta = Meta::new("createdAt", "created_at", "timestamptz")
    .ops(OpSet::ordered())
    .json(JsonKind::Timestamptz);
static OLD_TOTAL_CENTS_META: Meta = Meta::new("totalCents", "total_cents", "int8")
    .ops(OpSet::ordered())
    .json(JsonKind::BigInt);
static OLD_TAGS_META: Meta = Meta::new("tags", "tags", "text[]").ops(OpSet::equality());
static OLD_METADATA_META: Meta = Meta::new("metadata", "metadata", "jsonb").json(JsonKind::Jsonb);

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
            .request(black_box(request.clone()))
            .unwrap()
            .build()
            .unwrap()
    });
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
    select(users().alias("u"))
        .left_join(
            orders().alias("o"),
            USER_ID.at("u").eq_field(ORDER_USER_ID.at("o")),
        )
        .column(USER_ID.at("u"))
        .column(USER_EMAIL.at("u"))
        .item(
            json_agg(ORDER_ID.at("o"))
                .aggregate_order_desc(ORDER_ID.at("o"))
                .aggregate_filter(ORDER_STATUS.at("o").eq(black_box("paid")))
                .alias("paid_order_ids"),
        )
        .filter(USER_ACTIVE.at("u").eq(black_box(true)))
        .filter(ORDER_TOTAL.at("o").gte(black_box(5_000_i64)))
        .group_by(USER_ID.at("u"))
        .group_by(USER_EMAIL.at("u"))
        .build()
        .unwrap()
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
