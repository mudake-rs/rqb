use sqlx::postgres::PgPoolOptions;

mod error;
mod routes;
mod services;
mod types;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://rqb:rqb@localhost:55432/rqb".to_owned());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)?;

    // This sample builds the router without listening on a port. The service
    // modules still compile against the same pool/transaction shapes used in a
    // real axum application.
    let _app = routes::router(pool);

    println!("REST API sample router built");
    Ok(())
}
