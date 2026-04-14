use rqb_core::{
    DeleteBuilder, DeleteQuery, InsertBuilder, InsertQuery, UpdateBuilder, UpdateQuery,
};
use serde::de::DeserializeOwned;
use tokio_postgres::Row;

use crate::{BuildPostgres, Result};

use super::driver::PgExecutor;
use super::query::{
    execute_query, query_all, query_all_as, query_one, query_one_as, query_optional,
    query_optional_as,
};

#[allow(async_fn_in_trait)]
pub trait ExecuteWritePostgres {
    async fn execute(self, exec: &impl PgExecutor) -> Result<u64>;
    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row>;
    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>>;
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>>;
    async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
    where
        T: DeserializeOwned;
    async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
    where
        T: DeserializeOwned;
    async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
    where
        T: DeserializeOwned;
}

macro_rules! impl_execute_write {
    ($ty:ty) => {
        impl ExecuteWritePostgres for $ty {
            async fn execute(self, exec: &impl PgExecutor) -> Result<u64> {
                execute_query(exec, self.build_pg()?).await
            }

            async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row> {
                query_one(exec, self.returning_all_if_empty().build_pg()?).await
            }

            async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>> {
                query_optional(exec, self.returning_all_if_empty().build_pg()?).await
            }

            async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>> {
                query_all(exec, self.returning_all_if_empty().build_pg()?).await
            }

            async fn fetch_one_as<T>(self, exec: &impl PgExecutor) -> Result<T>
            where
                T: DeserializeOwned,
            {
                query_one_as(exec, self.returning_all_if_empty().build_pg()?).await
            }

            async fn fetch_optional_as<T>(self, exec: &impl PgExecutor) -> Result<Option<T>>
            where
                T: DeserializeOwned,
            {
                query_optional_as(exec, self.returning_all_if_empty().build_pg()?).await
            }

            async fn fetch_all_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
            where
                T: DeserializeOwned,
            {
                query_all_as(exec, self.returning_all_if_empty().build_pg()?).await
            }
        }
    };
}

impl_execute_write!(InsertBuilder);
impl_execute_write!(InsertQuery);
impl_execute_write!(UpdateBuilder);
impl_execute_write!(UpdateQuery);
impl_execute_write!(DeleteBuilder);
impl_execute_write!(DeleteQuery);
