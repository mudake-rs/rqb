use actix_web::{HttpResponse, Responder, web};
use rqb::prelude::{SearchRequest, txn};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::{AppServices, OrderService},
    error::AppError,
    orders::requests::{CreateOrderRequest, OrderListParams, PatchOrderRequest},
    pagination::PaginatedResponse,
};

pub async fn list_orders(
    services: web::Data<AppServices>,
    query: web::Query<OrderListParams>,
) -> Result<impl Responder, AppError> {
    let params = query.into_inner();
    // HTTP-layer validation checks request shape before the service builds an rqb query.
    params.validate()?;
    let page = OrderService::list(services.db(), params.into_query()?).await?;
    let response = PaginatedResponse::from(page);
    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_order(
    services: web::Data<AppServices>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, AppError> {
    let response = OrderService::get(services.db(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_order(
    services: web::Data<AppServices>,
    payload: web::Json<CreateOrderRequest>,
) -> Result<impl Responder, AppError> {
    let payload = payload.into_inner();
    // Generated enum DTO fields deserialize before validation, so unknown statuses never reach
    // the service layer as arbitrary strings.
    payload.validate()?;
    // Explicit transaction example: the handler owns the boundary, and the DB service receives &Tx.
    let tx = services.db().begin().await?;
    let order = OrderService::create(&tx, payload.into()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(order))
}

pub async fn patch_order(
    services: web::Data<AppServices>,
    path: web::Path<Uuid>,
    payload: web::Json<PatchOrderRequest>,
) -> Result<impl Responder, AppError> {
    let payload = payload.into_inner();
    // Patch DTOs reject empty bodies, so the service does not run UPDATE with no assignments.
    payload.validate()?;
    let response = OrderService::patch(services.db(), path.into_inner(), payload.into()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_order(
    services: web::Data<AppServices>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    // Closure transaction example: useful when several service calls should rollback together.
    let order = services
        .db()
        .transaction(txn!(|tx| { OrderService::delete(tx, id).await }))
        .await?;
    Ok(HttpResponse::Ok().json(order))
}

pub async fn order_stats(services: web::Data<AppServices>) -> Result<impl Responder, AppError> {
    let response = OrderService::stats(services.db()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn search_orders(
    services: web::Data<AppServices>,
    payload: web::Json<SearchRequest>,
) -> Result<impl Responder, AppError> {
    // SearchRequest is intentionally dynamic JSON; rqb validates it against generated metadata.
    let response =
        PaginatedResponse::from(OrderService::search(services.db(), payload.into_inner()).await?);
    Ok(HttpResponse::Ok().json(response))
}
