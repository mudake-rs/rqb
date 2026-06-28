use rqb::prelude::*;
use rqb_sample_schema::order_search_view;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
struct SearchApiError {
    status: u16,
    code: &'static str,
    field: Option<String>,
    detail: String,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let current_org = Uuid::nil();
    let merged_request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "status", "operator": "in", "value": ["paid", "refunded"] },
                { "field": "total_cents", "operator": "between", "value": [5000, 20000] },
                { "field": "user_email", "operator": "contains", "value": "@example.com" },
                { "field": "last_event_at", "operator": "isNotNull" }
            ]
        },
        "sort": [
            { "field": "created_at", "dir": "desc" },
            { "field": "total_cents", "dir": "asc" }
        ],
        "limit": 20,
        "offset": 0
    }))?;

    // JSON search is only the client-controlled part. The Rust query still owns
    // the source, organization filter, and this intentionally narrow response
    // projection.
    let merged = select(order_search_view::view())
        .columns((
            order_search_view::ID,
            order_search_view::STATUS,
            order_search_view::TOTAL_CENTS,
        ))
        .filter(order_search_view::ORGANIZATION_ID.eq(current_org))
        .apply_search(merged_request)?
        .build()?;

    assert_eq!(
        merged.sql,
        "SELECT \"id\", \"status\", \"total_cents\" FROM \"sample\".\"order_search_view\" WHERE (\"organization_id\" = $1 AND \"status\" IN ($2, $3) AND \"total_cents\" BETWEEN $4 AND $5 AND \"user_email\" ILIKE $6 ESCAPE '\\' AND \"last_event_at\" IS NOT NULL) ORDER BY \"created_at\" DESC, \"total_cents\" ASC LIMIT $7 OFFSET $8"
    );
    assert_eq!(merged.params.len(), 8);

    let standalone_request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "open" },
        "limit": 5
    }))?;
    // If the JSON request is allowed to own the whole search clause, start from
    // a fresh select. Do not call this shape for tenant/user-scoped endpoints.
    let standalone = select(order_search_view::view())
        .apply_search(standalone_request)?
        .build()?;

    assert_eq!(
        standalone.sql,
        "SELECT \"id\", \"user_id\", \"organization_id\", \"organization_slug\", \"user_email\", \"status\", \"total_cents\", \"tags\", \"metadata\", \"created_at\", \"item_count\", \"event_count\", \"last_event_at\" FROM \"sample\".\"order_search_view\" WHERE \"status\" = $1 LIMIT $2"
    );
    assert_eq!(standalone.params.len(), 2);

    let invalid_field: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "unknown", "operator": "equals", "value": "x" }
    }))?;
    // Unknown or non-json-exposed fields fail before SQL is rendered.
    assert_eq!(
        search_api_error(search_orders(current_org, invalid_field).unwrap_err()),
        SearchApiError {
            status: 400,
            code: "unknown_search_field",
            field: Some("unknown".to_owned()),
            detail: "unknown search field".to_owned(),
        }
    );

    let hidden_field: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "tags", "operator": "equals", "value": ["priority"] }
    }))?;
    assert_eq!(
        search_api_error(search_orders(current_org, hidden_field).unwrap_err()),
        SearchApiError {
            status: 400,
            code: "search_field_not_exposed",
            field: Some("tags".to_owned()),
            detail: "field is not exposed to JSON search".to_owned(),
        }
    );

    let bad_operator: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "total_cents", "operator": "contains", "value": "50" }
    }))?;
    assert_eq!(
        search_api_error(search_orders(current_org, bad_operator).unwrap_err()),
        SearchApiError {
            status: 400,
            code: "invalid_search_operator",
            field: Some("total_cents".to_owned()),
            detail: "operator contains is not allowed for this field".to_owned(),
        }
    );

    let bad_value: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "total_cents", "operator": "gte", "value": "many" }
    }))?;
    assert_eq!(
        search_api_error(search_orders(current_org, bad_value).unwrap_err()),
        SearchApiError {
            status: 400,
            code: "invalid_search_value",
            field: Some("total_cents".to_owned()),
            detail: "expected 64-bit integer".to_owned(),
        }
    );

    let bad_sort: SearchRequest = serde_json::from_value(json!({
        "sort": [{ "field": "metadata", "dir": "asc" }]
    }))?;
    assert_eq!(
        search_api_error(search_orders(current_org, bad_sort).unwrap_err()),
        SearchApiError {
            status: 400,
            code: "invalid_sort",
            field: Some("metadata".to_owned()),
            detail: "field is not sortable".to_owned(),
        }
    );

    let empty_logical: SearchRequest = serde_json::from_value(json!({
        "filter": { "and": [] }
    }))?;
    assert_eq!(
        search_api_error(search_orders(current_org, empty_logical).unwrap_err()),
        SearchApiError {
            status: 400,
            code: "empty_search_logical",
            field: None,
            detail: "and group must contain at least one filter".to_owned(),
        }
    );

    println!("{}", merged.sql);
    println!("{}", standalone.sql);
    Ok(())
}

fn search_orders(org_id: Uuid, request: SearchRequest) -> rqb::Result<BuiltQuery> {
    select(order_search_view::view())
        .columns((
            order_search_view::ID,
            order_search_view::STATUS,
            order_search_view::TOTAL_CENTS,
        ))
        .filter(order_search_view::ORGANIZATION_ID.eq(org_id))
        .apply_search(request)?
        .build()
}

fn search_api_error(error: rqb::Error) -> SearchApiError {
    match error {
        rqb::Error::InvalidSearchField { field } => SearchApiError {
            status: 400,
            code: "unknown_search_field",
            field: Some(field),
            detail: "unknown search field".to_owned(),
        },
        rqb::Error::SearchFieldNotExposed { field } => SearchApiError {
            status: 400,
            code: "search_field_not_exposed",
            field: Some(field),
            detail: "field is not exposed to JSON search".to_owned(),
        },
        rqb::Error::InvalidSearchOperator(err) => SearchApiError {
            status: 400,
            code: "invalid_search_operator",
            detail: format!(
                "operator {} is not allowed for this field",
                err.operator
            ),
            field: Some(err.field),
        },
        rqb::Error::InvalidSearchValue(err) => SearchApiError {
            status: 400,
            code: "invalid_search_value",
            detail: format!("expected {}", err.expected),
            field: Some(err.field),
        },
        rqb::Error::InvalidSort { field } => SearchApiError {
            status: 400,
            code: "invalid_sort",
            field: Some(field),
            detail: "field is not sortable".to_owned(),
        },
        rqb::Error::EmptySearchLogical { logical } => SearchApiError {
            status: 400,
            code: "empty_search_logical",
            field: None,
            detail: format!("{logical} group must contain at least one filter"),
        },
        other => SearchApiError {
            status: 500,
            code: "query_shape_error",
            field: None,
            detail: other.to_string(),
        },
    }
}
