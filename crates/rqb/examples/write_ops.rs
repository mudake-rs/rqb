//! Render INSERT, partial UPDATE, and DELETE from serde write models.
//!
//! `value` and `set_from` use normal serde structs, including
//! `skip_serializing_if` for partial updates.

use rqb::prelude::*;
use serde::Serialize;

const ID: Field = Field::new("id", FieldType::Uuid);
const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
const STATUS: Field = Field::new("status", FieldType::Text);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb).sortable(false);

fn orders() -> Dataset {
    Dataset::table("orders").fields([ID, USER_ID, STATUS, METADATA])
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrder {
    id: String,
    user_id: String,
    status: String,
    metadata: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
