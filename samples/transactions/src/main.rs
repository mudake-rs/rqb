use rqb::prelude::*;
use rqb_sample_base::{
    ADA_USER_ID, CAMERA_PRODUCT_ID, OrderStatus,
    schema::{order_items, orders},
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrder {
    id: Uuid,
    user_id: Uuid,
    status: OrderStatus,
    channel: String,
    metadata: serde_json::Value,
    tags: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrderItem {
    id: Uuid,
    order_id: Uuid,
    product_id: Uuid,
    quantity: i32,
    unit_price_cents: i64,
    metadata: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;
    let order = new_order();
    let item = new_item(order.id);

    let tx = db.begin().await?;
    insert(orders::dataset()).value(&order).execute(&tx).await?;

    let savepoint = tx.savepoint("item_insert").await?;
    insert(order_items::dataset())
        .value(&item)
        .execute(&savepoint)
        .await?;
    savepoint.release().await?;

    tx.commit().await?;
    println!("committed order {}", order.id);

    db.transaction(txn!(|tx| {
        update(orders::dataset())
            .set(orders::TAGS, vec!["sample", "closure"])
            .filter(orders::ID.eq(order.id))
            .execute(tx)
            .await?;
        Ok(())
    }))
    .await?;
    println!("committed tag update through closure transaction");

    let rollback_tx = db.begin().await?;
    update(orders::dataset())
        .set(orders::STATUS, OrderStatus::Cancelled)
        .filter(orders::ID.eq(order.id))
        .execute(&rollback_tx)
        .await?;
    rollback_tx.rollback().await?;
    println!("rolled back cancellation");

    db.transaction(txn!(|tx| {
        delete(order_items::dataset())
            .filter(order_items::ORDER_ID.eq(order.id))
            .execute(tx)
            .await?;
        delete(orders::dataset())
            .filter(orders::ID.eq(order.id))
            .execute(tx)
            .await?;
        Ok(())
    }))
    .await?;
    println!("cleaned up order {}", order.id);

    Ok(())
}

fn new_order() -> NewOrder {
    NewOrder {
        id: Uuid::new_v4(),
        user_id: rqb_sample_base::uuid(ADA_USER_ID),
        status: OrderStatus::Draft,
        channel: "sample".to_owned(),
        metadata: serde_json::json!({ "source": "transactions-sample" }),
        tags: vec!["sample".to_owned()],
    }
}

fn new_item(order_id: Uuid) -> NewOrderItem {
    NewOrderItem {
        id: Uuid::new_v4(),
        order_id,
        product_id: rqb_sample_base::uuid(CAMERA_PRODUCT_ID),
        quantity: 1,
        unit_price_cents: 10900,
        metadata: serde_json::json!({ "warehouse": "ams" }),
    }
}
