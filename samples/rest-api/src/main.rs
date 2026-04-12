mod db;
mod error;
mod orders;
mod pagination;
mod sort;
mod users;

use actix_web::{App, HttpServer, middleware::Logger, web};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::AppServices;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://rqb:rqb@localhost:55432/rqb".to_owned());
    // AppServices owns the rqb Db pool; handlers pass it or an explicit transaction to DB services.
    let services = AppServices::connect(&database_url).await?;

    tracing::info!("listening on http://127.0.0.1:3000");
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // actix clones this cheap service container per worker; the underlying Db is pooled.
            .app_data(web::Data::new(services.clone()))
            .configure(orders::configure)
            .configure(users::configure)
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await?;

    Ok(())
}
