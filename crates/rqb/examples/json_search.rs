use rqb::prelude::*;

const ID: Field = Field::new("id", FieldType::Uuid);
const EMAIL: Field = Field::new("email", FieldType::Text);
const STATUS: Field = Field::new("status", FieldType::Text);
const TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
    .sortable(false)
    .json_paths(JsonPathPolicy::Dynamic);
const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

fn order_search() -> Dataset {
    Dataset::view("order_search_view")
        .fields([ID, EMAIL, STATUS, TOTAL_CENTS, METADATA, CREATED_AT])
        .default_limit(20)
        .max_limit(100)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request: SearchRequest = serde_json::from_value(serde_json::json!({
        "fields": ["id", "email", "status", "totalCents"],
        "sort": [{ "field": "createdAt", "dir": "DESC" }],
        "query": {
            "logical": "and",
            "predicates": [
                { "field": "status", "operator": "equals", "value": "paid" },
                { "field": "metadata.score", "operator": "gte", "value": 80 }
            ]
        },
        "limit": 20,
        "offset": 0
    }))?;

    let built = select(order_search())
        .filter(STATUS.ne("cancelled"))
        .request(request)
        .build_pg()?;

    println!("{}", built.debug_sql());
    Ok(())
}
