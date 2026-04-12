pub mod handlers;
pub mod requests;
pub mod responses;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .route("", web::get().to(handlers::list_users))
            .route("", web::post().to(handlers::create_user))
            .route("/{id}", web::get().to(handlers::get_user))
            .route("/{id}", web::patch().to(handlers::patch_user)),
    );
}
