use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    OrderStatus,
    orders::{Order, OrderMetadata, OrderStats as DbOrderStats},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
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

impl From<Order> for OrderResponse {
    fn from(order: Order) -> Self {
        Self {
            id: order.id,
            email: order.email,
            organization_id: order.organization_id,
            status: order.status,
            status_history: order.status_history,
            channel: order.channel,
            total_cents: order.total_cents,
            items_count: order.items_count,
            metadata: order.metadata,
            tags: order.tags,
            created_at: order.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStats {
    pub status: OrderStatus,
    pub orders: i64,
    pub total_cents: f64,
}

impl From<DbOrderStats> for OrderStats {
    fn from(stats: DbOrderStats) -> Self {
        Self {
            status: stats.status,
            orders: stats.orders,
            total_cents: stats.total_cents,
        }
    }
}
