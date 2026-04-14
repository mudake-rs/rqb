use rqb_core::RawQuery;
use serde::de::DeserializeOwned;
use tokio_postgres::{Row, types::FromSqlOwned};

use crate::{BuildPostgres, Result};

use super::driver::PgExecutor;
use super::query::{
    execute_query, query_all, query_one, query_one_scalar, query_optional, query_optional_scalar,
    query_scalar, raw_query_all_as, raw_query_one_as, raw_query_optional_as,
};

#[allow(async_fn_in_trait)]
pub trait ExecuteRawPostgres {
    async fn execute(self, exec: &impl PgExecutor) -> Result<u64>;
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
    async fn fetch_scalar<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: FromSqlOwned;
    async fn fetch_one_scalar<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: FromSqlOwned;
    async fn fetch_optional_scalar<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: FromSqlOwned;
}

impl ExecuteRawPostgres for RawQuery {
    async fn execute(self, exec: &impl PgExecutor) -> Result<u64> {
        execute_query(exec, self.build_pg()?).await
    }

    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>> {
        query_all(exec, self.build_pg()?).await
    }

    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
        query_one(exec, self.build_pg()?).await
    }

    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
        query_optional(exec, self.build_pg()?).await
    }

    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        raw_query_all_as(exec, self.build_pg()?).await
    }

    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned,
    {
        raw_query_one_as(exec, self.build_pg()?).await
    }

    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        raw_query_optional_as(exec, self.build_pg()?).await
    }

    async fn fetch_scalar<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: FromSqlOwned,
    {
        query_scalar(exec, self.build_pg()?).await
    }

    async fn fetch_one_scalar<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: FromSqlOwned,
    {
        query_one_scalar(exec, self.build_pg()?).await
    }

    async fn fetch_optional_scalar<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: FromSqlOwned,
    {
        query_optional_scalar(exec, self.build_pg()?).await
    }
}
