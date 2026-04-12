use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::users::{User, UserWithOrders as DbUserWithOrders};

pub use crate::db::users::{OrderSummary, UserProfile};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub status: String,
    pub profile: UserProfile,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            organization_id: user.organization_id,
            email: user.email,
            status: user.status,
            profile: user.profile,
            tags: user.tags,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserWithOrders {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub status: String,
    pub profile: UserProfile,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub orders: Vec<OrderSummary>,
}

impl From<DbUserWithOrders> for UserWithOrders {
    fn from(user: DbUserWithOrders) -> Self {
        Self {
            id: user.id,
            organization_id: user.organization_id,
            email: user.email,
            status: user.status,
            profile: user.profile,
            tags: user.tags,
            created_at: user.created_at,
            orders: user.orders,
        }
    }
}
