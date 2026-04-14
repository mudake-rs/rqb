use chrono::{DateTime, Utc};
use rqb::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::schema::{app_users as users, enums::OrderStatus, enums::UserStatus, orders};

#[derive(Debug, Clone)]
pub struct UserListQuery {
    pub status: Option<UserStatus>,
    pub tag: Option<String>,
    pub limit: u32,
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub struct CreateUser {
    pub organization_id: Uuid,
    pub email: String,
    pub status: UserStatus,
    pub profile: UserProfile,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<UserStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<UserProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub status: UserStatus,
    pub profile: UserProfile,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub score: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserWithOrders {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub status: UserStatus,
    pub profile: UserProfile,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub orders: Vec<OrderSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub id: Uuid,
    pub status: OrderStatus,
    pub channel: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewUser {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: UserStatus,
    profile: UserProfile,
    tags: Vec<String>,
}

pub struct UserService;

impl UserService {
    pub async fn list(
        exec: &impl PgExecutor,
        query: UserListQuery,
    ) -> rqb::postgres::Result<Page<User>> {
        // Optional query params stay as Option<T> until they reach the builder; no match/apply
        // boilerplate is needed for common filters.
        let page = select(users::dataset())
            .filter_option(query.status, |status| users::STATUS.eq(status))
            .filter_option(query.tag, |tag| users::TAGS.has(tag))
            .order_by(users::CREATED_AT.desc())
            .limit(query.limit)
            .offset(query.offset)
            .page_as::<User>(exec)
            .await?;
        Ok(page)
    }

    pub async fn get_with_orders(
        exec: &impl PgExecutor,
        id: Uuid,
    ) -> rqb::postgres::Result<UserWithOrders> {
        let user = users::table().alias("u");
        let order = orders::table().alias("o");
        // Generated Relation helpers keep joined fields ergonomic: `user.id()` is qualified as
        // `u.id`, while root output aliases are stripped back to `id`, `email`, etc. for serde.
        select(&user)
            .left_join(&order, user.id().eq_col(order.user_id()))
            .agg(
                json_agg(
                    "orders",
                    [
                        order.id(),
                        order.status(),
                        order.channel(),
                        order.created_at(),
                    ],
                )
                .filter(order.id().is_not_null()),
            )
            .filter(user.id().eq(id))
            .fetch_one_as(exec)
            .await
    }

    pub async fn create(exec: &impl PgExecutor, user: CreateUser) -> rqb::postgres::Result<User> {
        let id = Uuid::new_v4();
        let row = NewUser {
            id,
            organization_id: user.organization_id,
            email: user.email,
            status: user.status,
            profile: user.profile,
            tags: user.tags,
        };
        // A write `fetch_*` without `.returning(...)` returns all selectable fields by default,
        // so create can deserialize the inserted row directly.
        insert(users::dataset())
            .value(&row)
            .fetch_one_as(exec)
            .await
    }

    pub async fn patch(
        exec: &impl PgExecutor,
        id: Uuid,
        patch: UserPatch,
    ) -> rqb::postgres::Result<User> {
        // `set_from` skips None fields through serde and default returning gives the updated DTO.
        update(users::dataset())
            .set_from(&patch)
            .filter(users::ID.eq(id))
            .fetch_one_as(exec)
            .await
    }
}
