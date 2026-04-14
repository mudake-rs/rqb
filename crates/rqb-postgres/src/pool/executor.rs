use deadpool_postgres::GenericClient;
use tokio_postgres::{Row, types::ToSql};

use crate::{Error, PgExecutor, Result, StatementCache};

use super::{Db, Savepoint, Tx};

impl PgExecutor for Db {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Vec<Row>> {
        let client = self.get().await?;
        query_with_cache(&client, sql, params, cache).await
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Option<Row>> {
        let client = self.get().await?;
        query_opt_with_cache(&client, sql, params, cache).await
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<u64> {
        let client = self.get().await?;
        execute_with_cache(&client, sql, params, cache).await
    }
}

impl PgExecutor for Tx {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Vec<Row>> {
        query_with_cache(self.client()?, sql, params, cache).await
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Option<Row>> {
        query_opt_with_cache(self.client()?, sql, params, cache).await
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<u64> {
        execute_with_cache(self.client()?, sql, params, cache).await
    }
}

impl PgExecutor for Savepoint<'_> {
    async fn query(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Vec<Row>> {
        self.tx().query(sql, params, cache).await
    }

    async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<Option<Row>> {
        self.tx().query_opt(sql, params, cache).await
    }

    async fn execute_sql(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        cache: StatementCache,
    ) -> Result<u64> {
        self.tx().execute_sql(sql, params, cache).await
    }
}

async fn query_with_cache(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    cache: StatementCache,
) -> Result<Vec<Row>> {
    if cache.is_enabled() {
        let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
        client.query(&stmt, params).await.map_err(Error::from)
    } else {
        client.query(sql, params).await.map_err(Error::from)
    }
}

async fn query_opt_with_cache(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    cache: StatementCache,
) -> Result<Option<Row>> {
    if cache.is_enabled() {
        let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
        client.query_opt(&stmt, params).await.map_err(Error::from)
    } else {
        client.query_opt(sql, params).await.map_err(Error::from)
    }
}

async fn execute_with_cache(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    cache: StatementCache,
) -> Result<u64> {
    if cache.is_enabled() {
        let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
        client.execute(&stmt, params).await.map_err(Error::from)
    } else {
        client.execute(sql, params).await.map_err(Error::from)
    }
}
