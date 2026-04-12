use rqb_core::{
    DeleteBuilder, DeleteQuery, InsertBuilder, InsertQuery, SelectBuilder, SelectQuery,
    UpdateBuilder, UpdateQuery,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Row, Transaction, types::ToSql};

use crate::{BuildPostgres, BuiltQuery, BuiltSelect, Error, Postgres, Result, row_to_json};

#[allow(async_fn_in_trait)]
pub trait PgExecutor {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>>;

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row>;

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>>;

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64>;
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
}

#[allow(async_fn_in_trait)]
pub trait ExecutePostgres {
    async fn fetch_all(self, exec: &impl PgExecutor) -> Result<Vec<Row>>;
    async fn fetch_one(self, exec: &impl PgExecutor) -> Result<Row>;
    async fn fetch_optional(self, exec: &impl PgExecutor) -> Result<Option<Row>>;
    async fn fetch_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
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

    async fn fetch_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
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

    async fn fetch_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
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
    async fn fetch_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
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

            async fn fetch_as<T>(self, exec: &impl PgExecutor) -> Result<Vec<T>>
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

async fn query_all(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Vec<Row>> {
    let pg = built.params();
    exec.query(&built.sql, &pg.as_refs()).await
}

async fn query_one(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Row> {
    query_optional(exec, built).await?.ok_or(Error::NotFound)
}

async fn query_optional(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Option<Row>> {
    let pg = built.params();
    exec.query_opt(&built.sql, &pg.as_refs()).await
}

async fn execute_query(exec: &impl PgExecutor, built: BuiltQuery) -> Result<u64> {
    let pg = built.params();
    exec.execute_sql(&built.sql, &pg.as_refs()).await
}

async fn query_count(exec: &impl PgExecutor, built: BuiltQuery) -> Result<i64> {
    let row = query_one(exec, built).await?;
    Ok(row.get::<_, i64>(0))
}

async fn query_page_as<T>(
    exec: &impl PgExecutor,
    built: BuiltSelect,
    limit: u32,
    offset: u64,
) -> Result<Page<T>>
where
    T: DeserializeOwned,
{
    let rows = query_all_as(exec, built.rows);
    let count = query_count(exec, built.count);
    let (items, total) = tokio::try_join!(rows, count)?;
    Ok(Page {
        items,
        total,
        limit,
        offset,
    })
}

async fn query_all_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let columns = built.columns.clone();
    let rows = query_all(exec, built).await?;
    rows.iter()
        .map(|row| {
            let json = row_to_json(row, &columns)?;
            serde_json::from_value(json).map_err(Error::from)
        })
        .collect()
}

async fn query_one_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<T>
where
    T: DeserializeOwned,
{
    query_optional_as(exec, built).await?.ok_or(Error::NotFound)
}

async fn query_optional_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let columns = built.columns.clone();
    let row = query_optional(exec, built).await?;
    row.as_ref()
        .map(|row| {
            let json = row_to_json(row, &columns)?;
            serde_json::from_value(json).map_err(Error::from)
        })
        .transpose()
}
