use chrono::{DateTime, Utc};
use rqb::Insertable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub total_cents: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderSearchRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub organization_slug: Option<String>,
    pub user_email: String,
    pub status: String,
    pub total_cents: i64,
    pub tags: Vec<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub item_count: i64,
    pub event_count: i64,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserOrderSummaryRow {
    pub email: String,
    pub order_count: i64,
    pub orders: Option<Value>,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Deserialize, Insertable)]
#[rqb(table = rqb_sample_schema::app_users)]
pub struct CreateUser {
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, Insertable)]
#[rqb(table = rqb_sample_schema::orders)]
pub struct CreateOrder {
    pub user_id: Uuid,
    pub total_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub user: UserRow,
    pub order: OrderRow,
}
