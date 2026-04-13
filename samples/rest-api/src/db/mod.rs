use rqb::prelude::Db;

pub mod orders;
pub mod users;

pub use orders::OrderService;
pub use rqb_sample_base::schema;
pub use schema::enums::{OrderStatus, UserStatus};
pub use users::UserService;

#[derive(Clone)]
pub struct AppServices {
    db: Db,
}

impl AppServices {
    pub async fn connect(database_url: &str) -> rqb::postgres::Result<Self> {
        // One pooled Db is shared by handlers; transaction boundaries stay explicit at call sites.
        let db = rqb::connect(database_url).await?;
        Ok(Self::new(db))
    }

    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &Db {
        &self.db
    }
}
