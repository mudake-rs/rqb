use rqb::prelude::*;
use serde_json::json;
use uuid::Uuid;

mod schema;

use schema::order_search_view as orders;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let current_org = Uuid::nil();
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "status", "operator": "equals", "value": "paid" },
                { "field": "total_cents", "operator": "gte", "value": 5000 }
            ]
        },
        "sort": [{ "field": "created_at", "dir": "desc" }],
        "limit": 20,
        "offset": 0
    }))
    .unwrap();

    let built = select(orders::view())
        .column(orders::ID)
        .column(orders::STATUS)
        .column(orders::TOTAL_CENTS)
        .filter(orders::ORGANIZATION_ID.eq(current_org))
        .request(request)?
        .build()?;

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"total_cents\" FROM \"sample\".\"order_search_view\" WHERE (\"organization_id\" = $1 AND (\"status\" = $2 AND \"total_cents\" >= $3)) ORDER BY \"created_at\" DESC LIMIT $4 OFFSET $5"
    );

    println!("{}", built.sql);
    Ok(())
}
