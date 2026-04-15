use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rqb::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool};

mod schema;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Serialize, sqlx::FromRow)]
struct UserRow {
    pub id: rqb::uuid::Uuid,
    pub email: String,
    pub display_name: String,
    pub active: bool,
    pub created_at: rqb::chrono::DateTime<rqb::chrono::Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct OrderRow {
    pub id: rqb::uuid::Uuid,
    pub user_id: rqb::uuid::Uuid,
    pub status: String,
    pub total_cents: i64,
    pub created_at: rqb::chrono::DateTime<rqb::chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateUser {
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
struct CreateOrder {
    pub user_id: rqb::uuid::Uuid,
    pub total_cents: i64,
}

#[derive(Debug, Serialize)]
struct CheckoutResponse {
    pub user: UserRow,
    pub order: OrderRow,
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    Db(Box<rqb::Error>),
}

impl From<rqb::Error> for ApiError {
    fn from(error: rqb::Error) -> Self {
        Self::Db(Box::new(error))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Db(error) => match *error {
                rqb::Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
                rqb::Error::UniqueViolation { constraint, .. } => (
                    StatusCode::CONFLICT,
                    format!("unique violation{}", suffix("constraint", constraint.as_deref())),
                ),
                rqb::Error::ForeignKeyViolation { constraint, .. } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "foreign key violation{}",
                        suffix("constraint", constraint.as_deref())
                    ),
                ),
                error => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            },
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

fn suffix(label: &str, value: Option<&str>) -> String {
    value
        .map(|value| format!(" on {label} `{value}`"))
        .unwrap_or_default()
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<rqb::uuid::Uuid>,
) -> ApiResult<Json<UserRow>> {
    let user = users_service::find(&state.pool, id).await?;
    Ok(Json(user))
}

async fn search_users(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> ApiResult<Json<Vec<UserRow>>> {
    // The client controls filter/sort/page only; the service still owns source
    // and projection.
    let users = users_service::search(&state.pool, request).await?;
    Ok(Json(users))
}

async fn create_user(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> ApiResult<(StatusCode, Json<UserRow>)> {
    validate_user_input(&input)?;

    let user = users_service::create(&state.pool, input).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

async fn deactivate_user(
    State(state): State<AppState>,
    Path(id): Path<rqb::uuid::Uuid>,
) -> ApiResult<StatusCode> {
    users_service::deactivate(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn checkout(
    State(state): State<AppState>,
    Json(input): Json<CreateOrder>,
) -> ApiResult<(StatusCode, Json<CheckoutResponse>)> {
    if input.total_cents <= 0 {
        return Err(ApiError::BadRequest("total_cents must be positive".to_owned()));
    }

    let response = orders_service::checkout(&state.pool, input).await?;
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

mod users_service {
    use super::schema::app_users as users;
    use super::*;

    pub async fn find<'e, E>(exec: E, id: rqb::uuid::Uuid) -> rqb::Result<UserRow>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        select(users::table())
            .filter(users::ID.eq(id))
            .fetch_one_as::<_, UserRow>(exec)
            .await
    }

    pub async fn search<'e, E>(exec: E, request: SearchRequest) -> rqb::Result<Vec<UserRow>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        select(users::table())
            .filter(users::ACTIVE.eq(true))
            .request(request)?
            .fetch_all_as::<_, UserRow>(exec)
            .await
    }

    pub async fn create<'e, E>(exec: E, input: CreateUser) -> rqb::Result<UserRow>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        insert(users::table())
            .set(users::ID.set(rqb::uuid::Uuid::new_v4()))
            .set(users::EMAIL.set(input.email))
            .set(users::DISPLAY_NAME.set(input.display_name))
            .set(users::ACTIVE.set(true))
            .returning_all()
            .fetch_one_as::<_, UserRow>(exec)
            .await
    }

    pub async fn deactivate(pool: &PgPool, id: rqb::uuid::Uuid) -> rqb::Result<()> {
        // Explicit transactions stay sqlx-native. Take the connection once and
        // pass it through the same rqb execution methods as the pool path.
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        update(users::table())
            .set(users::ACTIVE.set(false))
            .filter(users::ID.eq(id))
            .execute(&mut *conn)
            .await?;

        super::orders_service::cancel_open_for_user(conn, id).await?;
        tx.commit().await?;
        Ok(())
    }
}

mod orders_service {
    use super::schema::orders;
    use super::*;

    pub async fn checkout(pool: &PgPool, input: CreateOrder) -> rqb::Result<CheckoutResponse> {
        // Closure-style transactions are useful when the transaction body is the
        // service operation itself.
        rqb::tx!(pool, |conn| {
            let user = users_service::find(&mut *conn, input.user_id).await?;
            let order = create(&mut *conn, input).await?;
            Ok(CheckoutResponse { user, order })
        })
        .await
    }

    async fn create(conn: &mut PgConnection, input: CreateOrder) -> rqb::Result<OrderRow> {
        insert(orders::table())
            .set(orders::ID.set(rqb::uuid::Uuid::new_v4()))
            .set(orders::USER_ID.set(input.user_id))
            .set(orders::STATUS.set("open"))
            .set(orders::TOTAL_CENTS.set(input.total_cents))
            .returning_all()
            .fetch_one_as::<_, OrderRow>(conn)
            .await
    }

    pub async fn cancel_open_for_user(
        conn: &mut PgConnection,
        user_id: rqb::uuid::Uuid,
    ) -> rqb::Result<()> {
        update(orders::table())
            .set(orders::STATUS.set("canceled"))
            .filter(BoolExpr::and([
                orders::USER_ID.eq(user_id),
                orders::STATUS.eq("open"),
            ]))
            .execute(conn)
            .await?;
        Ok(())
    }
}

fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/users/{id}", get(get_user))
        .route("/users/search", post(search_users))
        .route("/users", post(create_user))
        .route("/users/{id}/deactivate", post(deactivate_user))
        .route("/checkout", post(checkout))
        .with_state(AppState { pool })
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://rqb:rqb@localhost:55432/rqb".to_owned());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)?;
    // Build the router for compile-checking; this sample does not start a
    // listener during cargo check.
    let _app = router(pool);

    println!("REST API sample router built");
    Ok(())
}
