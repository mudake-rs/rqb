use serde::de::DeserializeOwned;
use tokio_postgres::{Row, types::FromSqlOwned};

use crate::{
    BindParam, BuiltQuery, BuiltSelect, Error, PgParams, Result, raw_row_to_json, row_to_json,
};

use super::driver::{Page, PgExecutor, StatementCache};

pub(super) async fn query_all(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Vec<Row>> {
    query_all_parts(exec, &built.sql, &built.params, built.cacheable).await
}

pub(super) async fn query_one(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Row> {
    query_optional(exec, built).await?.ok_or(Error::NotFound)
}

pub(super) async fn query_optional(
    exec: &impl PgExecutor,
    built: BuiltQuery,
) -> Result<Option<Row>> {
    query_optional_parts(exec, &built.sql, &built.params, built.cacheable).await
}

pub(super) async fn execute_query(exec: &impl PgExecutor, built: BuiltQuery) -> Result<u64> {
    execute_parts(exec, &built.sql, &built.params, built.cacheable).await
}

async fn query_all_parts(
    exec: &impl PgExecutor,
    sql: &str,
    params: &[BindParam],
    cacheable: bool,
) -> Result<Vec<Row>> {
    let pg = PgParams::from_binds(params);
    let refs = pg.as_refs();
    exec.query(sql, &refs, StatementCache::from_cacheable(cacheable))
        .await
}

async fn query_optional_parts(
    exec: &impl PgExecutor,
    sql: &str,
    params: &[BindParam],
    cacheable: bool,
) -> Result<Option<Row>> {
    let pg = PgParams::from_binds(params);
    let refs = pg.as_refs();
    exec.query_opt(sql, &refs, StatementCache::from_cacheable(cacheable))
        .await
}

async fn execute_parts(
    exec: &impl PgExecutor,
    sql: &str,
    params: &[BindParam],
    cacheable: bool,
) -> Result<u64> {
    let pg = PgParams::from_binds(params);
    let refs = pg.as_refs();
    exec.execute_sql(sql, &refs, StatementCache::from_cacheable(cacheable))
        .await
}

pub(super) async fn query_count(exec: &impl PgExecutor, built: BuiltQuery) -> Result<i64> {
    let row = query_one(exec, built).await?;
    Ok(row.get::<_, i64>(0))
}

pub(super) async fn query_page_as<T>(
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

pub(super) async fn query_all_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let BuiltQuery {
        sql,
        params,
        columns,
        cacheable,
    } = built;
    let rows = query_all_parts(exec, &sql, &params, cacheable).await?;
    rows.iter()
        .map(|row| {
            let json = row_to_json(row, &columns)?;
            serde_json::from_value(json).map_err(Error::from)
        })
        .collect()
}

pub(super) async fn query_one_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<T>
where
    T: DeserializeOwned,
{
    query_optional_as(exec, built).await?.ok_or(Error::NotFound)
}

pub(super) async fn query_optional_as<T>(
    exec: &impl PgExecutor,
    built: BuiltQuery,
) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let BuiltQuery {
        sql,
        params,
        columns,
        cacheable,
    } = built;
    let row = query_optional_parts(exec, &sql, &params, cacheable).await?;
    row.as_ref()
        .map(|row| {
            let json = row_to_json(row, &columns)?;
            serde_json::from_value(json).map_err(Error::from)
        })
        .transpose()
}

pub(super) async fn raw_query_all_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let rows = query_all(exec, built).await?;
    rows.iter()
        .map(|row| {
            let json = raw_row_to_json(row)?;
            serde_json::from_value(json).map_err(Error::from)
        })
        .collect()
}

pub(super) async fn raw_query_one_as<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<T>
where
    T: DeserializeOwned,
{
    raw_query_optional_as(exec, built)
        .await?
        .ok_or(Error::NotFound)
}

pub(super) async fn raw_query_optional_as<T>(
    exec: &impl PgExecutor,
    built: BuiltQuery,
) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let row = query_optional(exec, built).await?;
    row.as_ref()
        .map(|row| {
            let json = raw_row_to_json(row)?;
            serde_json::from_value(json).map_err(Error::from)
        })
        .transpose()
}

pub(super) async fn query_scalar<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<Vec<T>>
where
    T: FromSqlOwned,
{
    let rows = query_all(exec, built).await?;
    rows.iter()
        .map(|row| row.try_get(0).map_err(Error::from))
        .collect()
}

pub(super) async fn query_one_scalar<T>(exec: &impl PgExecutor, built: BuiltQuery) -> Result<T>
where
    T: FromSqlOwned,
{
    query_optional_scalar(exec, built)
        .await?
        .ok_or(Error::NotFound)
}

pub(super) async fn query_optional_scalar<T>(
    exec: &impl PgExecutor,
    built: BuiltQuery,
) -> Result<Option<T>>
where
    T: FromSqlOwned,
{
    query_optional(exec, built)
        .await?
        .map(|row| row.try_get(0).map_err(Error::from))
        .transpose()
}
