use rqb::prelude::*;
use rqb_sample_schema::order_search_view as orders;
use serde_json::json;
use uuid::Uuid;

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
        "sort": [{ "field": "created_at", "dir": "desc" }],
        "limit": 20,
        "offset": 0
    }))?;

    let merged = select(orders::view())
        .column(orders::ID)
        .column(orders::STATUS)
        .column(orders::TOTAL_CENTS)
        .filter(orders::ORGANIZATION_ID.eq(current_org))
        .request(merged_request)?
        .build()?;

    assert_eq!(
        merged.sql,
        "SELECT \"id\", \"status\", \"total_cents\" FROM \"sample\".\"order_search_view\" WHERE (\"organization_id\" = $1 AND \"status\" IN ($2, $3) AND \"total_cents\" BETWEEN $4 AND $5 AND \"user_email\" ILIKE $6 ESCAPE '\\' AND \"last_event_at\" IS NOT NULL) ORDER BY \"created_at\" DESC LIMIT $7 OFFSET $8"
    );
    assert_eq!(merged.params.len(), 8);

    let replacement_request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "open" },
        "limit": 5
    }))?;
    let replaced = select(orders::view())
        .filter(orders::ORGANIZATION_ID.eq(current_org))
        .replace_request(replacement_request)?
        .build()?;

    assert_eq!(
        replaced.sql,
        "SELECT \"id\", \"user_id\", \"organization_id\", \"organization_slug\", \"user_email\", \"status\", \"total_cents\", \"tags\", \"metadata\", \"created_at\", \"item_count\", \"event_count\", \"last_event_at\" FROM \"sample\".\"order_search_view\" WHERE \"status\" = $1 LIMIT $2"
    );
    assert_eq!(replaced.params.len(), 2);

    let invalid_request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "unknown", "operator": "equals", "value": "x" }
    }))?;
    assert!(matches!(
        select(orders::view()).request(invalid_request),
        Err(rqb::Error::InvalidSearchField { field }) if field == "unknown"
    ));

    println!("{}", merged.sql);
    println!("{}", replaced.sql);
    Ok(())
}
