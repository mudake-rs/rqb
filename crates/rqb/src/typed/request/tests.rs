use serde_json::json;

use super::SearchRequest;
use crate::typed::{Field, JsonKind, Meta, OpSet, Source};

static ID_META: Meta = Meta::new("id", "id", "int4")
    .ops(OpSet::ordered())
    .json(JsonKind::Integer);
static STATUS_META: Meta = Meta::new("status", "status", "text")
    .ops(OpSet::ordered())
    .json(JsonKind::Text);
static ACTIVE_META: Meta = Meta::new("active", "active", "bool")
    .ops(OpSet::equality())
    .json(JsonKind::Bool);
static INTERNAL_META: Meta = Meta::new("internal", "internal", "text").ops(OpSet::ordered());
static FIELDS: [&Meta; 4] = [&ID_META, &STATUS_META, &ACTIVE_META, &INTERNAL_META];
const ID: Field<i32> = Field::new(&ID_META);

fn source() -> Source {
    Source::Table {
        name: "public.orders",
        alias: None,
        fields: &FIELDS,
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

    let built = crate::typed::select(source())
        .filter(ID.gt(10))
        .request(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (\"id\" > $1 AND (\"status\" = $2 AND \"id\" >= $3)) ORDER BY \"id\" DESC LIMIT $4 OFFSET $5"
    );
    assert_eq!(built.params.len(), 5);
}

#[test]
fn search_request_qualifies_fields_when_root_source_is_aliased() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "paid" },
        "sort": [{ "field": "id", "dir": "asc" }]
    }))
    .unwrap();

    let built = crate::typed::select(source().alias("o"))
        .request(request)
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

    let built = crate::typed::select(source())
        .request(request)
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
fn replace_request_replaces_existing_filter() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "paid" }
    }))
    .unwrap();

    let built = crate::typed::select(source())
        .filter(ID.gt(10))
        .replace_request(request)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE \"status\" = $1"
    );
}

#[test]
fn hidden_search_field_is_rejected() {
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "internal", "operator": "equals", "value": "x" }
    }))
    .unwrap();

    let err = crate::typed::select(source()).request(request).unwrap_err();

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

    let err = crate::typed::select(source()).request(request).unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSearchValue { field, expected: "boolean" }
            if field == "active"
    ));
}

#[test]
fn json_request_does_not_accept_projection_fields() {
    let err = serde_json::from_value::<SearchRequest>(json!({
        "fields": ["id"]
    }))
    .unwrap_err();

    assert!(err.to_string().contains("unknown field"));
}
