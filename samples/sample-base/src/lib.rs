pub mod schema;

pub use schema::enums::{OrderStatus, UserStatus};

pub const DATABASE_URL: &str = "postgres://rqb:rqb@localhost:55432/rqb";
pub const ACME_ORG_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const ADA_USER_ID: &str = "10000000-0000-0000-0000-000000000001";
pub const CAMERA_PRODUCT_ID: &str = "20000000-0000-0000-0000-000000000001";

pub async fn connect() -> rqb::postgres::Result<rqb::prelude::Db> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DATABASE_URL.to_owned());
    rqb::connect(&url).await
}

pub fn uuid(value: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(value).expect("sample UUID constant is valid")
}
