//! Render INSERT, partial UPDATE, and DELETE from WriteRecord models.
//!
//! `value` and `set_from` use direct field/value DTOs. Patch DTOs can use
//! `#[rqb(skip_none)]` to skip absent `Option` fields.

use rqb::prelude::*;

mod order_fields {
    use super::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
    pub const STATUS: Field = Field::new("status", FieldType::Text);
    pub const METADATA: Field = Field::new("metadata", FieldType::Jsonb).sortable(false);
}

use order_fields::{ID, METADATA, STATUS, USER_ID};

fn orders() -> Dataset {
    Dataset::table("orders").fields([ID, USER_ID, STATUS, METADATA])
}

#[derive(WriteRecord)]
#[rqb(fields = order_fields)]
struct NewOrder {
    id: String,
    user_id: String,
    status: String,
    metadata: serde_json::Value,
}

#[derive(WriteRecord)]
#[rqb(fields = order_fields, skip_none)]
struct OrderPatch {
    status: Option<String>,
    metadata: Option<serde_json::Value>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let new_order = NewOrder {
        id: "30000000-0000-0000-0000-000000009999".to_owned(),
        user_id: "10000000-0000-0000-0000-000000000001".to_owned(),
        status: "draft".to_owned(),
        metadata: serde_json::json!({ "source": "example" }),
    };

    let insert_sql = insert(orders())
        .value(&new_order)
        .returning([ID, STATUS])
        .build_pg()?;

    println!("-- insert");
    println!("{}", insert_sql.debug_sql());

    let patch = OrderPatch {
        status: Some("paid".to_owned()),
        metadata: None,
    };

    let update_sql = update(orders())
        .set_from(&patch)
        .filter(ID.eq(new_order.id.as_str()))
        .returning([ID, STATUS])
        .build_pg()?;

    println!("-- update");
    println!("{}", update_sql.debug_sql());

    let delete_sql = delete(orders())
        .filter(ID.eq(new_order.id.as_str()))
        .returning([ID])
        .build_pg()?;

    println!("-- delete");
    println!("{}", delete_sql.debug_sql());

    Ok(())
}
