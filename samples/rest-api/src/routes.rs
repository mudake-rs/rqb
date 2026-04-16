use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use rqb::prelude::SearchRequest;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::{orders, users};
use crate::types::{
    CheckoutResponse, CreateOrder, CreateUser, OrderSearchRow, Page, UserOrderSummaryRow, UserRow,
};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/users/{id}", get(get_user))
        .route("/users/search", post(search_users))
        .route("/users", post(create_user))
        .route("/users/{id}/deactivate", post(deactivate_user))
        .route("/orders/search", post(search_orders))
        .route("/orders/summary", get(order_summary))
        .route("/checkout", post(checkout))
        .with_state(AppState { pool })
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UserRow>> {
    let user = users::find(&state.pool, id).await?;
    Ok(Json(user))
}

async fn search_users(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> ApiResult<Json<Vec<UserRow>>> {
    let users = users::search(&state.pool, request).await?;
    Ok(Json(users))
}

async fn create_user(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> ApiResult<(StatusCode, Json<UserRow>)> {
    validate_user_input(&input)?;

    let user = users::create(&state.pool, input).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

async fn deactivate_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    users::deactivate(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn search_orders(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> ApiResult<Json<Page<OrderSearchRow>>> {
    // The route accepts only the JSON search envelope. The service adds server
    // filters and owns pagination/count semantics.
    let orders = orders::search(&state.pool, request).await?;
    Ok(Json(orders))
}

async fn order_summary(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<UserOrderSummaryRow>>> {
    let summary = orders::summary(&state.pool).await?;
    Ok(Json(summary))
}

async fn checkout(
    State(state): State<AppState>,
    Json(input): Json<CreateOrder>,
) -> ApiResult<(StatusCode, Json<CheckoutResponse>)> {
    // Request validation stays at the HTTP boundary; the DB service receives a
    // clean command DTO.
    if input.total_cents <= 0 {
        return Err(ApiError::BadRequest("total_cents must be positive".to_owned()));
    }

    let response = orders::checkout(&state.pool, input).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

fn validate_user_input(input: &CreateUser) -> ApiResult<()> {
    if input.email.trim().is_empty() {
        return Err(ApiError::BadRequest("email is required".to_owned()));
    }
    if input.display_name.trim().is_empty() {
        return Err(ApiError::BadRequest("display_name is required".to_owned()));
    }
    Ok(())
}
