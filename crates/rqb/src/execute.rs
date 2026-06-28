use async_stream::try_stream;
use futures_core::stream::BoxStream;
use futures_util::{StreamExt, TryStreamExt};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, Decode, FromRow, PgExecutor, PgPool, Postgres, Type};

use crate::Result;
use crate::{
    BuiltQuery, Delete, Insert, Merge, RawStmt, Select, SetQuery, Stmt, Update, count_all, select,
    subquery,
};

/// Rust value that can be decoded from a single Postgres result column.
///
/// This is the read-side pair to [`crate::BindValue`]: it keeps sqlx scalar
/// decode bounds out of user-facing method signatures.
pub trait ScalarValue: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin {}

impl<T> ScalarValue for T where T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin {}

fn safe_sql(sql: &str) -> AssertSqlSafe<&str> {
    // rqb reaches execution only after validation and rendering have produced
    // parameterized Postgres SQL; raw fragments are server-owned with bind
    // counts validated, and user values are carried separately as binds.
    AssertSqlSafe(sql)
}

impl BuiltQuery {
    /// Executes the query and returns affected row count.
    pub async fn execute<'e>(&self, executor: impl PgExecutor<'e>) -> Result<u64> {
        let result = sqlx::query_with(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
    }

    /// Fetches all rows as raw sqlx `PgRow` values.
    pub async fn fetch_all<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<PgRow>> {
        sqlx::query_with(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    /// Streams raw sqlx `PgRow` values.
    ///
    /// The returned stream borrows this [`BuiltQuery`], so keep the built query
    /// value alive until the stream is fully consumed.
    pub fn fetch_stream<'q, 'e>(
        &'q self,
        executor: impl PgExecutor<'e> + 'q,
    ) -> Result<BoxStream<'q, Result<PgRow>>>
    where
        'e: 'q,
    {
        Ok(sqlx::query_with(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch(executor)
            .map_err(Into::into)
            .boxed())
    }

    /// Streams raw sqlx `PgRow` values from an owned pool-backed query.
    ///
    /// The returned stream owns this built query and a cloneable [`PgPool`]
    /// handle, so it can outlive the call frame that created it.
    pub fn fetch_stream_pool(self, pool: PgPool) -> Result<BoxStream<'static, Result<PgRow>>> {
        let Self {
            sql,
            params,
            cacheable,
        } = self;
        let arguments = params.arguments()?;

        Ok(try_stream! {
            let mut rows = sqlx::query_with(safe_sql(&sql), arguments)
                .persistent(cacheable)
                .fetch(&pool);
            while let Some(row) = rows.try_next().await.map_err(crate::Error::from)? {
                yield row;
            }
        }
        .boxed())
    }

    /// Fetches exactly one raw sqlx `PgRow`.
    pub async fn fetch_one<'e>(&self, executor: impl PgExecutor<'e>) -> Result<PgRow> {
        sqlx::query_with(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches zero or one raw sqlx `PgRow`.
    pub async fn fetch_optional<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Option<PgRow>> {
        sqlx::query_with(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches all rows into a `sqlx::FromRow` type.
    pub async fn fetch_all_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<T>>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    /// Streams rows into a `sqlx::FromRow` type.
    ///
    /// Streaming avoids materializing large exports in memory. Keep the
    /// [`BuiltQuery`] value alive while consuming the stream.
    pub fn fetch_stream_as<'q, 'e, T>(
        &'q self,
        executor: impl PgExecutor<'e> + 'q,
    ) -> Result<BoxStream<'q, Result<T>>>
    where
        'e: 'q,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin + 'q,
    {
        Ok(
            sqlx::query_as_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
                .persistent(self.cacheable)
                .fetch(executor)
                .map_err(Into::into)
                .boxed(),
        )
    }

    /// Streams rows into a `sqlx::FromRow` type from an owned pool-backed query.
    ///
    /// The returned stream owns this built query and a cloneable [`PgPool`]
    /// handle, so it can outlive the call frame that created it.
    pub fn fetch_stream_pool_as<T>(self, pool: PgPool) -> Result<BoxStream<'static, Result<T>>>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin + 'static,
    {
        let Self {
            sql,
            params,
            cacheable,
        } = self;
        let arguments = params.arguments()?;

        Ok(try_stream! {
            let mut rows = sqlx::query_as_with::<_, T, _>(safe_sql(&sql), arguments)
                .persistent(cacheable)
                .fetch(&pool);
            while let Some(row) = rows.try_next().await.map_err(crate::Error::from)? {
                yield row;
            }
        }
        .boxed())
    }

    /// Fetches exactly one row into a `sqlx::FromRow` type.
    pub async fn fetch_one_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches zero or one row into a `sqlx::FromRow` type.
    pub async fn fetch_optional_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Option<T>>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches all rows as a single decoded scalar column.
    pub async fn fetch_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<T>>
    where
        T: ScalarValue,
    {
        sqlx::query_scalar_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    /// Streams rows as a single decoded scalar column.
    ///
    /// Keep the [`BuiltQuery`] value alive while consuming the stream.
    pub fn fetch_stream_scalar<'q, 'e, T>(
        &'q self,
        executor: impl PgExecutor<'e> + 'q,
    ) -> Result<BoxStream<'q, Result<T>>>
    where
        'e: 'q,
        T: ScalarValue + 'q,
    {
        Ok(
            sqlx::query_scalar_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
                .persistent(self.cacheable)
                .fetch(executor)
                .map_err(Into::into)
                .boxed(),
        )
    }

    /// Streams rows as a single decoded scalar column from an owned pool-backed query.
    ///
    /// The returned stream owns this built query and a cloneable [`PgPool`]
    /// handle, so it can outlive the call frame that created it.
    pub fn fetch_stream_pool_scalar<T>(self, pool: PgPool) -> Result<BoxStream<'static, Result<T>>>
    where
        T: ScalarValue + 'static,
    {
        let Self {
            sql,
            params,
            cacheable,
        } = self;
        let arguments = params.arguments()?;

        Ok(try_stream! {
            let mut rows = sqlx::query_scalar_with::<_, T, _>(safe_sql(&sql), arguments)
                .persistent(cacheable)
                .fetch(&pool);
            while let Some(row) = rows.try_next().await.map_err(crate::Error::from)? {
                yield row;
            }
        }
        .boxed())
    }

    /// Fetches exactly one decoded scalar value.
    pub async fn fetch_one_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
    where
        T: ScalarValue,
    {
        sqlx::query_scalar_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches zero or one decoded scalar value.
    pub async fn fetch_optional_scalar<'e, T>(
        &self,
        executor: impl PgExecutor<'e>,
    ) -> Result<Option<T>>
    where
        T: ScalarValue,
    {
        sqlx::query_scalar_with::<_, T, _>(safe_sql(&self.sql), self.arguments()?)
            .persistent(self.cacheable)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }
}

impl Stmt {
    /// Builds and executes the statement.
    pub async fn execute<'e>(&self, executor: impl PgExecutor<'e>) -> Result<u64> {
        self.build()?.execute(executor).await
    }

    /// Builds the statement and fetches all raw rows.
    pub async fn fetch_all<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<PgRow>> {
        self.build()?.fetch_all(executor).await
    }

    /// Builds the statement and fetches one raw row.
    pub async fn fetch_one<'e>(&self, executor: impl PgExecutor<'e>) -> Result<PgRow> {
        self.build()?.fetch_one(executor).await
    }

    /// Builds the statement and fetches an optional raw row.
    pub async fn fetch_optional<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Option<PgRow>> {
        self.build()?.fetch_optional(executor).await
    }

    /// Builds the statement and streams raw rows from an owned pool-backed query.
    pub fn fetch_stream_pool(self, pool: PgPool) -> Result<BoxStream<'static, Result<PgRow>>> {
        self.build()?.fetch_stream_pool(pool)
    }

    /// Builds the statement and fetches all rows into a `sqlx::FromRow` type.
    pub async fn fetch_all_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<T>>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        self.build()?.fetch_all_as(executor).await
    }

    /// Builds the statement and fetches one row into a `sqlx::FromRow` type.
    pub async fn fetch_one_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        self.build()?.fetch_one_as(executor).await
    }

    /// Builds the statement and fetches an optional row into a `sqlx::FromRow` type.
    pub async fn fetch_optional_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Option<T>>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        self.build()?.fetch_optional_as(executor).await
    }

    /// Builds the statement and streams rows into a `sqlx::FromRow` type from an owned pool-backed query.
    pub fn fetch_stream_pool_as<T>(self, pool: PgPool) -> Result<BoxStream<'static, Result<T>>>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin + 'static,
    {
        self.build()?.fetch_stream_pool_as(pool)
    }

    /// Builds the statement and fetches all rows as a single scalar column.
    pub async fn fetch_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<T>>
    where
        T: ScalarValue,
    {
        self.build()?.fetch_scalar(executor).await
    }

    /// Builds the statement and fetches one scalar value.
    pub async fn fetch_one_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
    where
        T: ScalarValue,
    {
        self.build()?.fetch_one_scalar(executor).await
    }

    /// Builds the statement and fetches an optional scalar value.
    pub async fn fetch_optional_scalar<'e, T>(
        &self,
        executor: impl PgExecutor<'e>,
    ) -> Result<Option<T>>
    where
        T: ScalarValue,
    {
        self.build()?.fetch_optional_scalar(executor).await
    }

    /// Builds the statement and streams scalar values from an owned pool-backed query.
    pub fn fetch_stream_pool_scalar<T>(self, pool: PgPool) -> Result<BoxStream<'static, Result<T>>>
    where
        T: ScalarValue + 'static,
    {
        self.build()?.fetch_stream_pool_scalar(pool)
    }
}

impl Select {
    /// Executes a matching `count(*)` query for this select.
    ///
    /// The count query strips `ORDER BY`, `LIMIT`, `OFFSET`, `FETCH`, and row
    /// locks before wrapping the select as a subquery.
    pub async fn count<'e>(&self, executor: impl PgExecutor<'e>) -> Result<i64> {
        self.build_count()?.fetch_one_scalar(executor).await
    }

    pub(crate) fn build_count(&self) -> Result<BuiltQuery> {
        let mut count = self.clone();
        count.order.clear();
        count.limit = None;
        count.offset = None;
        count.fetch = None;
        count.lock = None;
        select(subquery(count, "rqb_count", ()))
            .expr(count_all())
            .build()
    }
}

macro_rules! impl_statement_execute {
    ($ty:ty) => {
        impl $ty {
            /// Builds and executes the statement.
            pub async fn execute<'e>(&self, executor: impl PgExecutor<'e>) -> Result<u64> {
                self.build()?.execute(executor).await
            }

            /// Builds the statement and fetches all raw rows.
            pub async fn fetch_all<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<PgRow>> {
                self.build()?.fetch_all(executor).await
            }

            /// Builds the statement and fetches one raw row.
            pub async fn fetch_one<'e>(&self, executor: impl PgExecutor<'e>) -> Result<PgRow> {
                self.build()?.fetch_one(executor).await
            }

            /// Builds the statement and fetches an optional raw row.
            pub async fn fetch_optional<'e>(
                &self,
                executor: impl PgExecutor<'e>,
            ) -> Result<Option<PgRow>> {
                self.build()?.fetch_optional(executor).await
            }

            /// Builds the statement and streams raw rows from an owned pool-backed query.
            pub fn fetch_stream_pool(
                self,
                pool: PgPool,
            ) -> Result<BoxStream<'static, Result<PgRow>>> {
                self.build()?.fetch_stream_pool(pool)
            }

            /// Builds the statement and fetches all rows into a `sqlx::FromRow` type.
            pub async fn fetch_all_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<T>>
            where
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
            {
                self.build()?.fetch_all_as(executor).await
            }

            /// Builds the statement and fetches one row into a `sqlx::FromRow` type.
            pub async fn fetch_one_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
            where
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
            {
                self.build()?.fetch_one_as(executor).await
            }

            /// Builds the statement and fetches an optional row into a `sqlx::FromRow` type.
            pub async fn fetch_optional_as<'e, T>(
                &self,
                executor: impl PgExecutor<'e>,
            ) -> Result<Option<T>>
            where
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
            {
                self.build()?.fetch_optional_as(executor).await
            }

            /// Builds the statement and streams rows into a `sqlx::FromRow` type from an owned pool-backed query.
            pub fn fetch_stream_pool_as<T>(
                self,
                pool: PgPool,
            ) -> Result<BoxStream<'static, Result<T>>>
            where
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin + 'static,
            {
                self.build()?.fetch_stream_pool_as(pool)
            }

            /// Builds the statement and fetches all rows as a single scalar column.
            pub async fn fetch_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<T>>
            where
                T: ScalarValue,
            {
                self.build()?.fetch_scalar(executor).await
            }

            /// Builds the statement and fetches one scalar value.
            pub async fn fetch_one_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
            where
                T: ScalarValue,
            {
                self.build()?.fetch_one_scalar(executor).await
            }

            /// Builds the statement and fetches an optional scalar value.
            pub async fn fetch_optional_scalar<'e, T>(
                &self,
                executor: impl PgExecutor<'e>,
            ) -> Result<Option<T>>
            where
                T: ScalarValue,
            {
                self.build()?.fetch_optional_scalar(executor).await
            }

            /// Builds the statement and streams scalar values from an owned pool-backed query.
            pub fn fetch_stream_pool_scalar<T>(
                self,
                pool: PgPool,
            ) -> Result<BoxStream<'static, Result<T>>>
            where
                T: ScalarValue + 'static,
            {
                self.build()?.fetch_stream_pool_scalar(pool)
            }
        }
    };
}

impl_statement_execute!(Select);
impl_statement_execute!(SetQuery);
impl_statement_execute!(Insert);
impl_statement_execute!(Update);
impl_statement_execute!(Delete);
impl_statement_execute!(Merge);
impl_statement_execute!(RawStmt);
