pub mod handlers;
pub mod requests;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .route("", web::get().to(handlers::list_orders))
            .route("", web::post().to(handlers::create_order))
            .route("/stats", web::get().to(handlers::order_stats))
            .route("/search", web::post().to(handlers::search_orders))
            .route("/{id}", web::get().to(handlers::get_order))
            .route("/{id}", web::patch().to(handlers::patch_order))
            .route("/{id}", web::delete().to(handlers::delete_order)),
    );
}
