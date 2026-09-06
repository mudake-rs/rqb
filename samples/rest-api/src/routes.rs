use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rqb::prelude::SearchRequest;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::{orders, products, reports, users};
use crate::types::{
    CheckoutResponse, CreateOrder, CreateUser, CursorPage, DailyOrderStats, OrderCursor, OrderRow,
    OrderSearchRow, Page, PatchUser, ProductRow, TransitionOrder, UpsertProduct,
    UserOrderSummaryRow, UserRow,
};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/users/{id}", get(get_user).patch(patch_user))
        .route("/users/search", post(search_users))
        .route("/users", post(create_user))
        .route("/users/{id}/deactivate", post(deactivate_user))
        .route("/orders", get(list_orders_after))
        .route("/orders/filter", get(filter_orders))
        .route("/orders/search", post(search_orders))
        .route("/orders/export.csv", get(export_orders_csv))
        .route("/orders/{id}/transition", post(transition_order))
        .route("/orders/summary", get(order_summary))
        .route("/products/upsert", post(upsert_product))
        .route("/reports/orders-by-day", get(orders_by_day))
        .route("/checkout", post(checkout))
        .with_state(AppState { pool })
}

#[derive(Debug, Deserialize)]
struct OrderCursorQuery {
    user_id: Uuid,
    after_created_at: Option<chrono::DateTime<chrono::Utc>>,
    after_id: Option<Uuid>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OrderExportQuery {
    user_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct OrderFilterQuery {
    user_id: Uuid,
    status: Option<String>,
    min_total: Option<i64>,
    from_date: Option<chrono::DateTime<chrono::Utc>>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ReportQuery {
    days: Option<i64>,
}

async fn get_user(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<UserRow>> {
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

async fn patch_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<PatchUser>,
) -> ApiResult<Json<UserRow>> {
    // This shape is both the HTTP PATCH body and the DB changeset. Split those
    // structs only when the public API and the write model actually diverge.
    let assignments = rqb::Changeset::changeset_assignments(&input);
    if assignments.is_empty() {
        return Err(ApiError::BadRequest(
            "PATCH body must include at least one field".to_owned(),
        ));
    }

    let user = users::patch(&state.pool, id, assignments).await?;
    Ok(Json(user))
}

async fn deactivate_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    users::deactivate(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_orders_after(
    State(state): State<AppState>,
    Query(query): Query<OrderCursorQuery>,
) -> ApiResult<Json<CursorPage<OrderRow>>> {
    let cursor = match (query.after_created_at, query.after_id) {
        (Some(created_at), Some(id)) => Some(OrderCursor { created_at, id }),
        (None, None) => None,
        _ => {
            return Err(ApiError::BadRequest(
                "after_created_at and after_id must be passed together".to_owned(),
            ));
        }
    };
    let page = orders::list_after(
        &state.pool,
        query.user_id,
        cursor,
        query.limit.unwrap_or(50),
    )
    .await?;

    Ok(Json(page))
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

async fn filter_orders(
    State(state): State<AppState>,
    Query(query): Query<OrderFilterQuery>,
) -> ApiResult<Json<Vec<OrderRow>>> {
    // Serde parses query-string values at the HTTP boundary, so services
    // receive typed Rust values instead of raw strings.
    let rows = orders::filter(
        &state.pool,
        query.user_id,
        query.status,
        query.min_total,
        query.from_date,
        query.limit.unwrap_or(50),
    )
    .await?;

    Ok(Json(rows))
}

async fn export_orders_csv(
    State(state): State<AppState>,
    Query(query): Query<OrderExportQuery>,
) -> ApiResult<Response> {
    // PgPool is internally reference-counted; cloning it gives the response
    // stream an owned handle without opening a new database connection.
    let stream = orders::export_csv_stream(state.pool.clone(), query.user_id)?;
    let body = Body::from_stream(stream);

    // This is end-to-end streaming: Postgres rows are pulled lazily and each
    // CSV line becomes an HTTP body chunk. Query validation still happens
    // before response headers are returned.
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"orders.csv\"",
            ),
        ],
        body,
    )
        .into_response())
}

async fn transition_order(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<TransitionOrder>,
) -> ApiResult<Json<OrderRow>> {
    validate_transition(&input)?;

    let Some(order) = orders::transition(&state.pool, id, input).await? else {
        return Err(ApiError::Conflict(
            "order cannot transition to the requested status".to_owned(),
        ));
    };

    Ok(Json(order))
}

async fn order_summary(State(state): State<AppState>) -> ApiResult<Json<Vec<UserOrderSummaryRow>>> {
    let summary = orders::summary(&state.pool).await?;
    Ok(Json(summary))
}

async fn upsert_product(
    State(state): State<AppState>,
    Json(input): Json<UpsertProduct>,
) -> ApiResult<Json<ProductRow>> {
    validate_product(&input)?;

    let product = products::upsert(&state.pool, input).await?;
    Ok(Json(product))
}

async fn orders_by_day(
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> ApiResult<Json<Vec<DailyOrderStats>>> {
    let rows = reports::orders_by_day(&state.pool, query.days.unwrap_or(30)).await?;
    Ok(Json(rows))
}

async fn checkout(
    State(state): State<AppState>,
    Json(input): Json<CreateOrder>,
) -> ApiResult<(StatusCode, Json<CheckoutResponse>)> {
    // Request validation stays at the HTTP boundary; the DB service receives a
    // clean command DTO.
    if input.total_cents <= 0 {
        return Err(ApiError::BadRequest(
            "total_cents must be positive".to_owned(),
        ));
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

fn validate_transition(input: &TransitionOrder) -> ApiResult<()> {
    if !matches!(input.status.as_str(), "paid" | "canceled" | "refunded") {
        return Err(ApiError::BadRequest(
            "status must be paid, canceled, or refunded".to_owned(),
        ));
    }
    Ok(())
}

fn validate_product(input: &UpsertProduct) -> ApiResult<()> {
    if input.sku.trim().is_empty() {
        return Err(ApiError::BadRequest("sku is required".to_owned()));
    }
    if input.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".to_owned()));
    }
    if input.price_cents < 0 {
        return Err(ApiError::BadRequest(
            "price_cents must be non-negative".to_owned(),
        ));
    }
    Ok(())
}
