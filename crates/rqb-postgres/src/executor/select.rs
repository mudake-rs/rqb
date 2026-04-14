use rqb_core::{SelectBuilder, SelectQuery};
use serde::de::DeserializeOwned;
use tokio_postgres::Row;

use crate::{BuildPostgres, Postgres, Result};

use super::driver::{Page, PgExecutor};
use super::query::{
    query_all, query_all_as, query_count, query_one, query_one_as, query_optional,
    query_optional_as, query_page_as,
};

#[allow(async_fn_in_trait)]
pub trait ExecutePostgres {
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>>;
    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row>;
    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>>;
    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned;
    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned;
    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned;
    async fn count(self, exec: &impl PgExecutor) -> Result<i64>;
    async fn page_as<T>(self, exec: &impl PgExecutor) -> Result<Page<T>>
    where
        T: DeserializeOwned;
}

impl ExecutePostgres for SelectBuilder {
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>> {
        query_all(exec, self.build_pg()?.rows).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        query_one(exec, self.limit(1).build_pg()?.rows).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        query_optional(exec, self.limit(1).build_pg()?.rows).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        query_all_as(exec, self.build_pg()?.rows).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        query_one_as(exec, self.limit(1).build_pg()?.rows).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        query_optional_as(exec, self.limit(1).build_pg()?.rows).await
    }

    async fn count(self, exec: &impl PgExecutor) -> Result<i64> {
        query_count(exec, self.build_pg()?.count).await
    }

    async fn page_as<T>(self, exec: &impl PgExecutor) -> Result<Page<T>>
    where
        T: DeserializeOwned,
    {
        let query = self.build();
        let (built, limit, offset) = Postgres::build_page(query)?;
        query_page_as(exec, built, limit, offset).await
    }
}

impl ExecutePostgres for SelectQuery {
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>> {
        query_all(exec, self.build_pg()?.rows).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        let mut query = self;
        query.request.limit = Some(1);
        query_one(exec, query.build_pg()?.rows).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        let mut query = self;
        query.request.limit = Some(1);
        query_optional(exec, query.build_pg()?.rows).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        query_all_as(exec, self.build_pg()?.rows).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut query = self;
        query.request.limit = Some(1);
        query_one_as(exec, query.build_pg()?.rows).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut query = self;
        query.request.limit = Some(1);
        query_optional_as(exec, query.build_pg()?.rows).await
    }

    async fn count(self, exec: &impl PgExecutor) -> Result<i64> {
        query_count(exec, self.build_pg()?.count).await
    }

    async fn page_as<T>(self, exec: &impl PgExecutor) -> Result<Page<T>>
    where
        T: DeserializeOwned,
    {
        let (built, limit, offset) = Postgres::build_page(self)?;
        query_page_as(exec, built, limit, offset).await
    }
}
