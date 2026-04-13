//! Build a typed SELECT by hand from field metadata.
//!
//! This is the smallest useful rqb shape: define fields, define a dataset,
//! compose filters, and inspect the rendered Postgres SQL.

use rqb::prelude::*;

const ID: Field = Field::new("id", FieldType::Uuid);
const EMAIL: Field = Field::new("email", FieldType::Text);
const STATUS: Field = Field::new("status", FieldType::Text);
const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
    .sortable(false)
    .json_paths(JsonPathPolicy::Dynamic);
const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = Dataset::view("order_search_view")
        .fields([ID, EMAIL, STATUS, TAGS, METADATA, CREATED_AT])
        .max_limit(500);

    let built = select(orders)
        .fields([ID, EMAIL, CREATED_AT])
        .filter_option(Some("paid"), |status| STATUS.eq(status))
        .filter(all([
            TAGS.contains_any(["vip", "gift"]),
            METADATA.path("score").gte(80),
        ]))
        .order_by(CREATED_AT.desc())
        .limit(20)
        .build_pg()?;

    println!("{}", built.rows.sql);
    println!("{:?}", built.rows.params);
    Ok(())
}
