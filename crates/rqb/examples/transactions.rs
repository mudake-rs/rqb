//! Run writes inside explicit transactions, savepoints, and closure transactions.
//!
//! This example connects to Postgres. Start the repository test database with
//! `make db-up`, then run with `DATABASE_URL=postgres://rqb:rqb@localhost:55432/rqb`.

use rqb::prelude::*;
use serde::Serialize;
use uuid::Uuid;

const ID: Field = Field::new("id", FieldType::Uuid);
const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
const ORDER_STATUS: EnumType = EnumType::new(
    Some("public"),
    "order_status",
    &["draft", "paid", "cancelled", "refunded"],
);
const STATUS: Field = Field::new("status", FieldType::Enum(ORDER_STATUS));
const CHANNEL: Field = Field::new("channel", FieldType::Text);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb).sortable(false);
const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);

fn orders() -> Dataset {
    Dataset::table("orders").fields([ID, USER_ID, STATUS, CHANNEL, METADATA, TAGS])
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrder {
    id: Uuid,
    user_id: Uuid,
    status: String,
    channel: String,
    metadata: serde_json::Value,
    tags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://rqb:rqb@localhost:55432/rqb".to_owned());
    let db = rqb::connect(&url).await?;

    let order = sample_order();

    let tx = db.begin().await?;
    insert(orders()).value(&order).execute(&tx).await?;

    let savepoint = tx.savepoint("after_order").await?;
    update(orders())
        .set(TAGS, vec!["example", "savepoint"])
        .filter(ID.eq(order.id))
        .execute(&savepoint)
        .await?;
    savepoint.release().await?;

    tx.commit().await?;

    let rollback_tx = db.begin().await?;
    update(orders())
        .set(STATUS, "cancelled")
        .filter(ID.eq(order.id))
        .execute(&rollback_tx)
        .await?;
    rollback_tx.rollback().await?;

    db.transaction(txn!(|tx| {
        update(orders())
            .set(TAGS, vec!["example", "closure"])
            .filter(ID.eq(order.id))
            .execute(tx)
            .await?;
        Ok(())
    }))
    .await?;

    db.transaction(txn!(|tx| {
        delete(orders()).filter(ID.eq(order.id)).execute(tx).await?;
        Ok(())
    }))
    .await?;

    Ok(())
}

fn sample_order() -> NewOrder {
    NewOrder {
        id: Uuid::new_v4(),
        user_id: Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
        status: "draft".to_owned(),
        channel: "example".to_owned(),
        metadata: serde_json::json!({ "source": "transactions-example" }),
        tags: vec!["example".to_owned()],
    }
}
