use actix_web::{HttpResponse, Responder, web};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::{AppServices, UserService},
    error::AppError,
    pagination::PaginatedResponse,
    users::requests::{CreateUserRequest, PatchUserRequest, UserListParams},
};

pub async fn list_users(
    services: web::Data<AppServices>,
    query: web::Query<UserListParams>,
) -> Result<impl Responder, AppError> {
    let params = query.into_inner();
    // Keep web validation at the boundary; services receive already-normalized DTOs.
    params.validate()?;
    let page = UserService::list(services.db(), params.into()).await?;
    let response = PaginatedResponse::from(page);
    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_user(
    services: web::Data<AppServices>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, AppError> {
    // The "get user" route intentionally returns a nested orders aggregate, not only app_users.
    let response = UserService::get_with_orders(services.db(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_user(
    services: web::Data<AppServices>,
    payload: web::Json<CreateUserRequest>,
) -> Result<impl Responder, AppError> {
    let payload = payload.into_inner();
    // Generated enum DTO fields deserialize before validation, so services receive typed values.
    payload.validate()?;
    let response = UserService::create(services.db(), payload.into()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn patch_user(
    services: web::Data<AppServices>,
    path: web::Path<Uuid>,
    payload: web::Json<PatchUserRequest>,
) -> Result<impl Responder, AppError> {
    let payload = payload.into_inner();
    // Empty patch bodies are rejected before rqb sees the write builder.
    payload.validate()?;
    let response = UserService::patch(services.db(), path.into_inner(), payload.into()).await?;
    Ok(HttpResponse::Ok().json(response))
}
