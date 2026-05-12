use serde_json::json;

use super::SearchRequest;
use crate::{Field, JsonKind, Meta, OpSet, Source};

static ID_META: Meta = Meta::new("id", "id", "int4")
    .ops(OpSet::ordered())
    .json(JsonKind::Integer);
static STATUS_META: Meta = Meta::new("status", "status", "text")
    .ops(OpSet::text())
    .json(JsonKind::Text);
static ACTIVE_META: Meta = Meta::new("active", "active", "bool")
    .ops(OpSet::equality())
    .json(JsonKind::Bool);
static INTERNAL_META: Meta = Meta::new("internal", "internal", "text").ops(OpSet::text());
static DOMAIN_TEXT_META: Meta = Meta::new("slug", "slug", "public.slug_domain")
    .ops(OpSet::text())
    .json(JsonKind::Text);
static TEXT_NO_OPS_META: Meta = Meta::new("memo", "memo", "text").json(JsonKind::Text);
static FIELDS: [&Meta; 4] = [&ID_META, &STATUS_META, &ACTIVE_META, &INTERNAL_META];
static PATTERN_FIELDS: [&Meta; 2] = [&DOMAIN_TEXT_META, &TEXT_NO_OPS_META];
const ID: Field<i32> = Field::new(&ID_META);

static UUID_META: Meta = Meta::new("externalId", "external_id", "uuid")
    .ops(OpSet::equality())
    .json(JsonKind::Uuid);
static TSZ_META: Meta = Meta::new("createdAt", "created_at", "timestamptz")
    .ops(OpSet::ordered())
    .json(JsonKind::Timestamptz);
static SCORE_META: Meta = Meta::new("score", "score", "float8")
    .ops(OpSet::ordered())
    .json(JsonKind::Float);
static VALUE_FIELDS: [&Meta; 3] = [&UUID_META, &TSZ_META, &SCORE_META];
static BIG_ID_META: Meta = Meta::new("bigId", "big_id", "int8")
    .ops(OpSet::ordered())
    .json(JsonKind::BigInt);
static AMOUNT_META: Meta = Meta::new("amount", "amount", "numeric")
    .ops(OpSet::ordered())
    .json(JsonKind::NumericString);
static DAY_META: Meta = Meta::new("day", "day", "date")
    .ops(OpSet::ordered())
    .json(JsonKind::Date);
static LOCAL_TIME_META: Meta = Meta::new("localTime", "local_time", "time")
    .ops(OpSet::ordered())
    .json(JsonKind::Time);
static LOCAL_TS_META: Meta = Meta::new("localTs", "local_ts", "timestamp")
    .ops(OpSet::ordered())
    .json(JsonKind::Timestamp);
static PAYLOAD_META: Meta = Meta::new("payload", "payload", "jsonb")
    .ops(OpSet::equality())
    .json(JsonKind::Jsonb);
static EXTRA_VALUE_FIELDS: [&Meta; 6] = [
    &BIG_ID_META,
    &AMOUNT_META,
    &DAY_META,
    &LOCAL_TIME_META,
    &LOCAL_TS_META,
    &PAYLOAD_META,
];

fn source() -> Source {
    Source::Table {
        name: "public.orders",
        alias: None,
        fields: &FIELDS,
    }
}

fn value_source() -> Source {
    Source::Table {
        name: "public.events",
        alias: None,
        fields: &VALUE_FIELDS,
    }
}

fn extra_value_source() -> Source {
    Source::Table {
        name: "public.extra_values",
        alias: None,
        fields: &EXTRA_VALUE_FIELDS,
    }
}

fn pattern_source() -> Source {
    Source::Table {
        name: "public.patterns",
        alias: None,
        fields: &PATTERN_FIELDS,
    }
}

#[test]
fn search_request_merges_filter_and_applies_sort_limit_offset() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "status", "operator": "equals", "value": "paid" },
                { "field": "id", "operator": "gte", "value": 100 }
            ]
        },
        "sort": [{ "field": "id", "dir": "desc" }],
        "limit": 20,
        "offset": 40
    }))
    .unwrap();

    let built = crate::select(source())
        .filter(ID.gt(10))
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (\"id\" > $1 AND \"status\" = $2 AND \"id\" >= $3) ORDER BY \"id\" DESC LIMIT $4 OFFSET $5"
    );
    assert_eq!(built.params.len(), 5);
}

#[test]
fn search_request_applies_multiple_sort_keys_in_client_order() {
    let request: SearchRequest = serde_json::from_value(json!({
        "sort": [
            { "field": "status", "dir": "asc" },
            { "field": "id", "dir": "desc" }
        ]
    }))
    .unwrap();

    let built = crate::select(source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" ORDER BY \"status\" ASC, \"id\" DESC"
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn search_request_qualifies_fields_when_root_source_is_aliased() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "paid" },
        "sort": [{ "field": "id", "dir": "asc" }]
    }))
    .unwrap();

    let built = crate::select(source().alias("o"))
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"o\".\"id\", \"o\".\"status\", \"o\".\"active\", \"o\".\"internal\" FROM \"public\".\"orders\" AS \"o\" WHERE \"o\".\"status\" = $1 ORDER BY \"o\".\"id\" ASC"
    );
}

#[test]
fn search_request_supports_null_in_between_and_like() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "status", "operator": "isNotNull" },
                { "field": "status", "operator": "in", "value": ["paid", "open"] },
                { "field": "id", "operator": "between", "value": [10, 20] },
                { "field": "status", "operator": "contains", "value": "50%_match" },
                { "field": "status", "operator": "regex", "value": "^p" },
                { "field": "status", "operator": "iRegex", "value": "^paid" },
                { "field": "status", "operator": "iLike", "value": "p%" }
            ]
        }
    }))
    .unwrap();

    let built = crate::select(source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (\"status\" IS NOT NULL AND \"status\" IN ($1, $2) AND \"id\" BETWEEN $3 AND $4 AND \"status\" ILIKE $5 ESCAPE '\\' AND \"status\" ~ $6 AND \"status\" ~* $7 AND \"status\" ILIKE $8)"
    );
    assert_eq!(built.params.len(), 8);
}

#[test]
fn fresh_select_apply_search_uses_request_as_complete_search_clause() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "paid" }
    }))
    .unwrap();

    let built = crate::select(source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE \"status\" = $1"
    );
}

#[test]
fn apply_search_replaces_existing_sort_limit_and_offset() {
    let request: SearchRequest = serde_json::from_value(json!({
        "sort": [{ "field": "status", "dir": "asc" }],
        "limit": 5,
        "offset": 10
    }))
    .unwrap();

    let built = crate::select(source())
        .order_desc(ID)
        .limit(100)
        .offset(200)
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" ORDER BY \"status\" ASC LIMIT $1 OFFSET $2"
    );
    assert_eq!(built.params.len(), 2);
}

#[test]
fn hidden_search_field_is_rejected() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "internal", "operator": "equals", "value": "x" }
    }))
    .unwrap();

    let err = crate::select(source()).apply_search(request).unwrap_err();

    assert!(matches!(
        err,
        crate::Error::SearchFieldNotExposed { field } if field == "internal"
    ));
}

#[test]
fn invalid_json_value_is_rejected_before_rendering() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "active", "operator": "equals", "value": "yes" }
    }))
    .unwrap();

    let err = crate::select(source()).apply_search(request).unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSearchValue(err) if err.field == "active" && err.expected == "boolean"
    ));
}

#[test]
fn json_request_does_not_accept_projection_fields() {
    let err = serde_json::from_value::<SearchRequest>(json!({
        "fields": ["id"]
    }))
    .unwrap_err();

    assert!(err.is_data());
}

#[test]
fn json_request_rejects_unknown_predicate_fields() {
    let err = serde_json::from_value::<SearchRequest>(json!({
        "filter": {
            "field": "status",
            "operator": "equals",
            "value": "paid",
            "raw": "1=1"
        }
    }))
    .unwrap_err();

    assert!(err.is_data());
}

#[test]
fn json_filter_rejects_ambiguous_or_extra_logical_fields() {
    for payload in [
        json!({ "filter": { "and": [], "extra": "ignored" } }),
        json!({ "filter": { "and": [], "field": "status", "operator": "equals", "value": "paid" } }),
        json!({ "filter": { "and": [], "or": [] } }),
    ] {
        let err = serde_json::from_value::<SearchRequest>(payload).unwrap_err();
        assert!(err.is_data());
    }
}

#[test]
fn json_request_rejects_unknown_operator_names() {
    let err = serde_json::from_value::<SearchRequest>(json!({
        "filter": { "field": "status", "operator": "containsAll", "value": "paid" }
    }))
    .unwrap_err();

    assert!(err.is_data());
}

#[test]
fn search_pattern_operators_use_metadata_capability_not_pg_whitelist() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "slug", "operator": "startsWith", "value": "acme" }
    }))
    .unwrap();

    let built = crate::select(pattern_source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"slug\", \"memo\" FROM \"public\".\"patterns\" WHERE \"slug\" ILIKE $1 ESCAPE '\\'"
    );
}

#[test]
fn search_operators_reject_json_fields_without_matching_opset_capability() {
    let is_null: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "memo", "operator": "isNull" }
    }))
    .unwrap();
    let contains: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "memo", "operator": "contains", "value": "x" }
    }))
    .unwrap();

    assert!(matches!(
        crate::select(pattern_source())
            .apply_search(is_null)
            .unwrap_err(),
        crate::Error::InvalidSearchOperator(err)
            if err.field == "memo" && err.operator == "isNull"
    ));
    assert!(matches!(
        crate::select(pattern_source())
            .apply_search(contains)
            .unwrap_err(),
        crate::Error::InvalidSearchOperator(err)
            if err.field == "memo" && err.operator == "contains"
    ));
}

#[test]
fn search_pattern_values_have_a_length_limit() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "field": "status",
            "operator": "regex",
            "value": "x".repeat(1025)
        }
    }))
    .unwrap();

    assert!(matches!(
        crate::select(source()).apply_search(request).unwrap_err(),
        crate::Error::InvalidSearchValue(err)
            if err.field == "status" && err.expected == "string up to 1024 characters"
    ));
}

#[test]
fn empty_json_logical_groups_are_rejected() {
    let and_request: SearchRequest = serde_json::from_value(json!({
        "filter": { "and": [] }
    }))
    .unwrap();
    let or_request: SearchRequest = serde_json::from_value(json!({
        "filter": { "or": [] }
    }))
    .unwrap();

    assert!(matches!(
        crate::select(source())
            .apply_search(and_request)
            .unwrap_err(),
        crate::Error::EmptySearchLogical { logical: "and" }
    ));
    assert!(matches!(
        crate::select(source())
            .apply_search(or_request)
            .unwrap_err(),
        crate::Error::EmptySearchLogical { logical: "or" }
    ));
}

#[test]
fn not_filter_wraps_a_compiled_search_predicate() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "not": { "field": "status", "operator": "equals", "value": "canceled" }
        }
    }))
    .unwrap();

    let built = crate::select(source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE NOT (\"status\" = $1)"
    );
}

#[test]
fn unknown_search_field_is_rejected_before_rendering() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "missing", "operator": "equals", "value": "x" }
    }))
    .unwrap();

    let err = crate::select(source()).apply_search(request).unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSearchField { field } if field == "missing"
    ));
}

#[test]
fn sort_rejects_fields_without_ordering_capability() {
    let request: SearchRequest = serde_json::from_value(json!({
        "sort": [{ "field": "active", "dir": "asc" }]
    }))
    .unwrap();

    let err = crate::select(source()).apply_search(request).unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSort { field } if field == "active"
    ));
}

#[test]
fn empty_in_and_not_in_render_boolean_constants() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "status", "operator": "in", "value": [] },
                { "field": "status", "operator": "notIn", "value": [] }
            ]
        }
    }))
    .unwrap();

    let built = crate::select(source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (FALSE AND TRUE)"
    );
    assert_eq!(built.params.len(), 0);
}

#[test]
fn between_requires_exactly_two_json_values() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "id", "operator": "between", "value": [1, 2, 3] }
    }))
    .unwrap();

    let err = crate::select(source()).apply_search(request).unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSearchValue(err) if err.field == "id" && err.expected == "two-element array"
    ));
}

#[test]
fn json_value_kinds_compile_to_typed_params() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                {
                    "field": "externalId",
                    "operator": "equals",
                    "value": "00000000-0000-0000-0000-000000000000"
                },
                {
                    "field": "createdAt",
                    "operator": "gte",
                    "value": "2026-04-16T12:00:00Z"
                },
                { "field": "score", "operator": "lt", "value": 99.5 }
            ]
        }
    }))
    .unwrap();

    let built = crate::select(value_source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"external_id\" AS \"externalId\", \"created_at\" AS \"createdAt\", \"score\" FROM \"public\".\"events\" WHERE (\"external_id\" = $1 AND \"created_at\" >= $2 AND \"score\" < $3)"
    );
    assert_eq!(built.params.len(), 3);
}

#[test]
fn additional_json_value_kinds_compile_to_typed_params() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "bigId", "operator": "equals", "value": 9007199254740991_i64 },
                { "field": "amount", "operator": "gte", "value": "123.45" },
                { "field": "day", "operator": "equals", "value": "2026-04-16" },
                { "field": "localTime", "operator": "lt", "value": "12:30:45.123" },
                { "field": "localTs", "operator": "lte", "value": "2026-04-16T12:30:45" },
                { "field": "payload", "operator": "equals", "value": { "source": "web" } }
            ]
        }
    }))
    .unwrap();

    let built = crate::select(extra_value_source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"big_id\" AS \"bigId\", \"amount\", \"day\", \"local_time\" AS \"localTime\", \"local_ts\" AS \"localTs\", \"payload\" FROM \"public\".\"extra_values\" WHERE (\"big_id\" = $1 AND \"amount\" >= $2 AND \"day\" = $3 AND \"local_time\" < $4 AND \"local_ts\" <= $5 AND \"payload\" = $6)"
    );
    assert_eq!(built.params.len(), 6);
}

#[test]
fn invalid_json_value_kinds_are_rejected_with_field_specific_expectations() {
    let bad_int: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "id", "operator": "equals", "value": 2147483648_i64 }
    }))
    .unwrap();
    let bad_numeric: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "amount", "operator": "equals", "value": "not-decimal" }
    }))
    .unwrap();
    let bad_date: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "day", "operator": "equals", "value": "16-04-2026" }
    }))
    .unwrap();

    assert!(matches!(
        crate::select(source()).apply_search(bad_int).unwrap_err(),
        crate::Error::InvalidSearchValue(err) if err.field == "id" && err.expected == "32-bit integer"
    ));
    assert!(matches!(
        crate::select(extra_value_source())
            .apply_search(bad_numeric)
            .unwrap_err(),
        crate::Error::InvalidSearchValue(err) if err.field == "amount" && err.expected == "decimal string"
    ));
    assert!(matches!(
        crate::select(extra_value_source())
            .apply_search(bad_date)
            .unwrap_err(),
        crate::Error::InvalidSearchValue(err) if err.field == "day" && err.expected == "date string"
    ));
}

#[test]
fn negated_search_operators_render_negated_sql_forms() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "id", "operator": "notBetween", "value": [10, 20] },
                { "field": "status", "operator": "notContains", "value": "test" },
                { "field": "status", "operator": "notStartsWith", "value": "tmp" },
                { "field": "status", "operator": "notEndsWith", "value": ".bak" },
                { "field": "status", "operator": "notLike", "value": "x%" },
                { "field": "status", "operator": "notILike", "value": "y%" },
                { "field": "status", "operator": "notRegex", "value": "^x" },
                { "field": "status", "operator": "notIRegex", "value": "^y" }
            ]
        }
    }))
    .unwrap();

    let built = crate::select(source())
        .apply_search(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (\"id\" NOT BETWEEN $1 AND $2 AND \"status\" NOT ILIKE $3 ESCAPE '\\' AND \"status\" NOT ILIKE $4 ESCAPE '\\' AND \"status\" NOT ILIKE $5 ESCAPE '\\' AND \"status\" NOT LIKE $6 AND \"status\" NOT ILIKE $7 AND \"status\" !~ $8 AND \"status\" !~* $9)"
    );
    assert_eq!(built.params.len(), 9);
}

#[test]
fn malformed_uuid_json_value_is_rejected() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "externalId", "operator": "equals", "value": "not-a-uuid" }
    }))
    .unwrap();

    let err = crate::select(value_source())
        .apply_search(request)
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSearchValue(err) if err.field == "externalId" && err.expected == "UUID string"
    ));
}

#[test]
fn timestamptz_json_value_requires_timezone() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "createdAt", "operator": "equals", "value": "2026-04-16T12:00:00" }
    }))
    .unwrap();

    let err = crate::select(value_source())
        .apply_search(request)
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSearchValue(err) if err.field == "createdAt" && err.expected == "RFC3339 timestamp string"
    ));
}
