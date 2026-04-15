use chrono::{DateTime, Utc};
use rqb::Insertable;
use serde::{Deserialize, Serialize};
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
