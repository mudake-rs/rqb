use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Row, Transaction, types::ToSql};

use crate::{Error, Result};

#[allow(async_fn_in_trait)]
pub trait PgExecutor {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>>;

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row>;

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>>;

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64>;

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        self.query(sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        self.query_one(sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        self.query_opt(sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        self.execute_sql(sql, params).await
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: u32,
    pub offset: u64,
}

impl<T> Page<T> {
    pub fn map<U, F>(self, f: F) -> Page<U>
    where
        F: FnMut(T) -> U,
    {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            limit: self.limit,
            offset: self.offset,
        }
    }
}

impl PgExecutor for Client {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        Client::query(self, sql, params).await.map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        Client::query_opt(self, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        Client::query_opt(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        Client::execute(self, sql, params)
            .await
            .map_err(Error::from)
    }
}

impl PgExecutor for Transaction<'_> {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        Transaction::query(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        Transaction::query_opt(self, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        Transaction::query_opt(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        Transaction::execute(self, sql, params)
            .await
            .map_err(Error::from)
    }
}

#[cfg(feature = "runtime-deadpool")]
impl PgExecutor for deadpool_postgres::Client {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        deadpool_postgres::GenericClient::query(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        deadpool_postgres::GenericClient::query_opt(self, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        deadpool_postgres::GenericClient::query_opt(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        deadpool_postgres::GenericClient::execute(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        deadpool_query_cached(self, sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        deadpool_query_one_cached(self, sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        deadpool_query_opt_cached(self, sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        deadpool_execute_cached(self, sql, params).await
    }
}

#[cfg(feature = "runtime-deadpool")]
async fn deadpool_query_cached(
    client: &deadpool_postgres::Client,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Row>> {
    let stmt = deadpool_postgres::GenericClient::prepare_cached(client, sql)
        .await
        .map_err(Error::from)?;
    deadpool_postgres::GenericClient::query(client, &stmt, params)
        .await
        .map_err(Error::from)
}

#[cfg(feature = "runtime-deadpool")]
async fn deadpool_query_one_cached(
    client: &deadpool_postgres::Client,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Row> {
    deadpool_query_opt_cached(client, sql, params)
        .await?
        .ok_or(Error::NotFound)
}

#[cfg(feature = "runtime-deadpool")]
async fn deadpool_query_opt_cached(
    client: &deadpool_postgres::Client,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Option<Row>> {
    let stmt = deadpool_postgres::GenericClient::prepare_cached(client, sql)
        .await
        .map_err(Error::from)?;
    deadpool_postgres::GenericClient::query_opt(client, &stmt, params)
        .await
        .map_err(Error::from)
}

#[cfg(feature = "runtime-deadpool")]
async fn deadpool_execute_cached(
    client: &deadpool_postgres::Client,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64> {
    let stmt = deadpool_postgres::GenericClient::prepare_cached(client, sql)
        .await
        .map_err(Error::from)?;
    deadpool_postgres::GenericClient::execute(client, &stmt, params)
        .await
        .map_err(Error::from)
}
