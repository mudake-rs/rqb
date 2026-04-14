//! Merge a client JSON SearchRequest with server-owned filters.
//!
//! The client can choose filters, sort, limit, and offset. The server still
//! owns the dataset, projection, and required predicates before rendering.

use rqb::prelude::*;

const ID: Field = Field::new("id", FieldType::Uuid);
const EMAIL: Field = Field::new("email", FieldType::Text);
const ORGANIZATION_ID: Field = Field::mapped("organizationId", "organization_id", FieldType::Uuid);
const STATUS: Field = Field::new("status", FieldType::Text);
const TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
    .sortable(false)
    .json_paths(JsonPathPolicy::Dynamic);
const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

fn order_search() -> Dataset {
    Dataset::view("order_search_view")
        .fields([
            ID,
            EMAIL,
            ORGANIZATION_ID,
            STATUS,
            TOTAL_CENTS,
            METADATA,
            CREATED_AT,
        ])
        .default_limit(20)
        .max_limit(100)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request: SearchRequest = serde_json::from_value(serde_json::json!({
        "sort": [{ "field": "createdAt", "dir": "desc" }],
        "filter": {
            "and": [
                { "field": "status", "operator": "equals", "value": "paid" },
                { "field": "metadata.score", "operator": "gte", "value": 80 }
            ]
        },
        "limit": 20,
        "offset": 0
    }))?;

    let built = select(order_search())
        .fields([ID, EMAIL, STATUS, TOTAL_CENTS])
        .filter(ORGANIZATION_ID.eq("00000000-0000-0000-0000-000000000001"))
        .request(request)
        .build_pg()?;

    println!("{}", built.debug_sql());
    Ok(())
}
