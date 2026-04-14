use chrono::{DateTime, Utc};
use rqb::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::schema::{
    enums::OrderStatus, events, order_items, order_search_view as order_search, orders,
};

#[derive(Debug, Clone)]
pub struct OrderListQuery {
    pub status: Option<OrderStatus>,
    pub channel: Option<String>,
    pub min_total: Option<i64>,
    pub sort: Sort,
    pub limit: u32,
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub struct CreateOrder {
    pub user_id: Uuid,
    pub status: OrderStatus,
    pub channel: String,
    pub metadata: OrderMetadata,
    pub tags: Vec<String>,
    pub items: Vec<CreateOrderItem>,
}

#[derive(Debug, Clone)]
pub struct CreateOrderItem {
    pub product_id: Uuid,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OrderStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OrderMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: Uuid,
    pub email: String,
    pub organization_id: Uuid,
    pub status: OrderStatus,
    pub status_history: Vec<OrderStatus>,
    pub channel: String,
    pub total_cents: i64,
    pub items_count: i64,
    pub metadata: OrderMetadata,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderMetadata {
    #[serde(default)]
    pub score: Option<i64>,
    #[serde(default)]
    pub campaign: Option<String>,
    #[serde(default)]
    pub gift: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStats {
    pub status: OrderStatus,
    pub orders: i64,
    pub total_cents: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrder {
    id: Uuid,
    user_id: Uuid,
    status: OrderStatus,
    status_history: Vec<OrderStatus>,
    channel: String,
    metadata: OrderMetadata,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrderItem {
    id: Uuid,
    order_id: Uuid,
    product_id: Uuid,
    quantity: i32,
    unit_price_cents: i64,
    metadata: serde_json::Value,
}

pub struct OrderService;

impl OrderService {
    pub async fn list(
        exec: &impl PgExecutor,
        query: OrderListQuery,
    ) -> rqb::Result<Page<Order>> {
        // The list endpoint is the "normal code" path: typed params become optional filters,
        // and rqb still validates field names, operators, and sortability before SQL is rendered.
        let page = select(order_search::dataset())
            .filter_option(query.status, |status| order_search::STATUS.eq(status))
            .filter_option(query.channel, |channel| order_search::CHANNEL.eq(channel))
            .filter_option(query.min_total, |min_total| {
                order_search::TOTAL_CENTS.gte(min_total)
            })
            .order_by(query.sort)
            .limit(query.limit)
            .offset(query.offset)
            .page_as::<Order>(exec)
            .await?;
        Ok(page)
    }

    pub async fn get(exec: &impl PgExecutor, id: Uuid) -> rqb::Result<Order> {
        select(order_search::dataset())
            .filter(order_search::ID.eq(id))
            .fetch_one_as(exec)
            .await
    }

    pub async fn create(
        exec: &impl PgExecutor,
        order: CreateOrder,
    ) -> rqb::Result<Order> {
        // The caller owns transaction boundaries. Handlers can pass `&Db` for one-shot writes or
        // `&Tx` when multiple service calls must commit/rollback together.
        let order_id = Uuid::new_v4();

        insert(orders::dataset())
            .value(&NewOrder {
                id: order_id,
                user_id: order.user_id,
                status: order.status,
                status_history: vec![order.status],
                channel: order.channel,
                metadata: order.metadata,
                tags: order.tags,
            })
            .execute(exec)
            .await?;

        let items = order
            .items
            .into_iter()
            .map(|item| NewOrderItem {
                id: Uuid::new_v4(),
                order_id,
                product_id: item.product_id,
                quantity: item.quantity,
                unit_price_cents: item.unit_price_cents,
                metadata: item.metadata,
            })
            .collect::<Vec<_>>();
        if !items.is_empty() {
            insert(order_items::dataset())
                .values(&items)
                .execute(exec)
                .await?;
        }

        Self::get(exec, order_id).await
    }

    pub async fn patch(
        exec: &impl PgExecutor,
        id: Uuid,
        patch: OrderPatch,
    ) -> rqb::Result<Order> {
        // We only need a marker row from the write. The public response is loaded from
        // `order_search_view`, where totals and user fields are already joined/precomputed.
        update(orders::dataset())
            .set_from(&patch)
            .filter(orders::ID.eq(id))
            .returning([orders::ID])
            .fetch_one(exec)
            .await?;
        Self::get(exec, id).await
    }

    pub async fn delete(exec: &impl PgExecutor, id: Uuid) -> rqb::Result<Order> {
        let existing = select(order_search::dataset())
            .filter(order_search::ID.eq(id))
            .fetch_one_as(exec)
            .await?;

        update(events::dataset())
            .set_null(events::ORDER_ID)
            .filter(events::ORDER_ID.eq(id))
            .execute(exec)
            .await?;
        delete(order_items::dataset())
            .filter(order_items::ORDER_ID.eq(id))
            .execute(exec)
            .await?;
        delete(orders::dataset())
            .filter(orders::ID.eq(id))
            .execute(exec)
            .await?;
        Ok(existing)
    }

    pub async fn stats(exec: &impl PgExecutor) -> rqb::Result<Vec<OrderStats>> {
        // Aggregates use the same field descriptors as regular selects; there is no hand-written
        // SQL here, but rqb still renders GROUP BY for the selected non-aggregate field.
        select(order_search::dataset())
            .fields([order_search::STATUS])
            .agg(count("orders"))
            .agg(sum(order_search::TOTAL_CENTS, "totalCents"))
            .order_by(order_search::STATUS.asc())
            .fetch_all_as(exec)
            .await
    }

    pub async fn search(
        exec: &impl PgExecutor,
        request: SearchRequest,
    ) -> rqb::Result<Page<serde_json::Value>> {
        // This endpoint shows the JSON request API. Client-selected fields make the shape dynamic,
        // so the sample returns serde_json::Value instead of pretending it has a fixed DTO.
        let page = select(order_search::dataset())
            .request(request)
            .page_as::<serde_json::Value>(exec)
            .await?;
        Ok(page)
    }
}
