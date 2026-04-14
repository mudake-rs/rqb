use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Row, Transaction, types::ToSql};

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementCache {
    Use,
    Bypass,
}

impl StatementCache {
    pub const fn from_cacheable(cacheable: bool) -> Self {
        if cacheable { Self::Use } else { Self::Bypass }
    }

    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Use)
    }
}

#[allow(async_fn_in_trait)]
pub trait PgExecutor {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Vec<Row>>;

    async fn query_one(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Row> {
        self.query_opt(sql, params, cache)
            .await?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Option<Row>>;

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<u64>;
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
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> Result<Vec<Row>> {
        Client::query(self, sql, params).await.map_err(Error::from)
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> Result<Option<Row>> {
        Client::query_opt(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> Result<u64> {
        Client::execute(self, sql, params)
            .await
            .map_err(Error::from)
    }
}

impl PgExecutor for Transaction<'_> {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> Result<Vec<Row>> {
        Transaction::query(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> Result<Option<Row>> {
        Transaction::query_opt(self, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        _cache: StatementCache,
    ) -> Result<u64> {
        Transaction::execute(self, sql, params)
            .await
            .map_err(Error::from)
    }
}

#[cfg(feature = "runtime-deadpool")]
impl PgExecutor for deadpool_postgres::Client {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Vec<Row>> {
        if cache.is_enabled() {
            deadpool_query_cached(self, sql, params).await
        } else {
            deadpool_postgres::GenericClient::query(self, sql, params)
                .await
                .map_err(Error::from)
        }
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Option<Row>> {
        if cache.is_enabled() {
            deadpool_query_opt_cached(self, sql, params).await
        } else {
            deadpool_postgres::GenericClient::query_opt(self, sql, params)
                .await
                .map_err(Error::from)
        }
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<u64> {
        if cache.is_enabled() {
            deadpool_execute_cached(self, sql, params).await
        } else {
            deadpool_postgres::GenericClient::execute(self, sql, params)
                .await
                .map_err(Error::from)
        }
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
