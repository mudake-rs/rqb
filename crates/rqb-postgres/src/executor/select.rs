use rqb_core::{QueryExpr, SelectBuilder, SelectQuery, SetQuery};
use serde::de::DeserializeOwned;
use tokio_postgres::Row;

use crate::{BuildPostgres, BuildRowsPostgres, Postgres, Result};

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
        query_all(exec, self.build_rows_pg()?).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        query_one(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        query_optional(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        query_all_as(exec, self.build_rows_pg()?).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        query_one_as(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        query_optional_as(exec, self.limit(1).build_rows_pg()?).await
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
        query_all(exec, self.build_rows_pg()?).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        let mut query = self;
        query.request.limit = Some(1);
        query_one(exec, query.build_rows_pg()?).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        let mut query = self;
        query.request.limit = Some(1);
        query_optional(exec, query.build_rows_pg()?).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        query_all_as(exec, self.build_rows_pg()?).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut query = self;
        query.request.limit = Some(1);
        query_one_as(exec, query.build_rows_pg()?).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut query = self;
        query.request.limit = Some(1);
        query_optional_as(exec, query.build_rows_pg()?).await
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

impl ExecutePostgres for QueryExpr {
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>> {
        query_all(exec, self.build_rows_pg()?).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        query_one(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        query_optional(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        query_all_as(exec, self.build_rows_pg()?).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        query_one_as(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        query_optional_as(exec, self.limit(1).build_rows_pg()?).await
    }

    async fn count(self, exec: &impl PgExecutor) -> Result<i64> {
        let built = Postgres::build_query(self)?;
        query_count(exec, built.count).await
    }

    async fn page_as<T>(self, exec: &impl PgExecutor) -> Result<Page<T>>
    where
        T: DeserializeOwned,
    {
        let (built, limit, offset) = Postgres::build_query_page(self)?;
        query_page_as(exec, built, limit, offset).await
    }
}

impl ExecutePostgres for SetQuery {
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>> {
        QueryExpr::from(self).fetch_all(exec).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        QueryExpr::from(self).fetch_one(exec).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        QueryExpr::from(self).fetch_optional(exec).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        QueryExpr::from(self).fetch_all_as(exec).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        QueryExpr::from(self).fetch_one_as(exec).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        QueryExpr::from(self).fetch_optional_as(exec).await
    }

    async fn count(self, exec: &impl PgExecutor) -> Result<i64> {
        QueryExpr::from(self).count(exec).await
    }

    async fn page_as<T>(self, exec: &impl PgExecutor) -> Result<Page<T>>
    where
        T: DeserializeOwned,
    {
        QueryExpr::from(self).page_as(exec).await
    }
}
