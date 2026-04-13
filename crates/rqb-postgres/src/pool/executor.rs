use deadpool_postgres::GenericClient;
use tokio_postgres::{Row, types::ToSql};

use crate::{Error, PgExecutor, Result};

use super::{Db, Savepoint, Tx};

impl PgExecutor for Db {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let client = self.get().await?;
        GenericClient::query(&client, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let client = self.get().await?;
        GenericClient::query_opt(&client, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        let client = self.get().await?;
        GenericClient::query_opt(&client, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        GenericClient::execute(&client, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let client = self.get().await?;
        query_cached(&client, sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let client = self.get().await?;
        query_one_cached(&client, sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        let client = self.get().await?;
        query_opt_cached(&client, sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        execute_cached(&client, sql, params).await
    }
}

impl PgExecutor for Tx {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        GenericClient::query(self.client()?, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        GenericClient::query_opt(self.client()?, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        GenericClient::query_opt(self.client()?, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        GenericClient::execute(self.client()?, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        query_cached(self.client()?, sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        query_one_cached(self.client()?, sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        query_opt_cached(self.client()?, sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        execute_cached(self.client()?, sql, params).await
    }
}

impl PgExecutor for Savepoint<'_> {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        self.tx().query(sql, params).await
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        self.tx().query_one(sql, params).await
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        self.tx().query_opt(sql, params).await
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        self.tx().execute_sql(sql, params).await
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        self.tx().query_cached(sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        self.tx().query_one_cached(sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        self.tx().query_opt_cached(sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        self.tx().execute_sql_cached(sql, params).await
    }
}

async fn query_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Row>> {
    let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
    client.query(&stmt, params).await.map_err(Error::from)
}

async fn query_one_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Row> {
    query_opt_cached(client, sql, params)
        .await?
        .ok_or(Error::NotFound)
}

async fn query_opt_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Option<Row>> {
    let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
    client.query_opt(&stmt, params).await.map_err(Error::from)
}

async fn execute_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64> {
    let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
    client.execute(&stmt, params).await.map_err(Error::from)
}
