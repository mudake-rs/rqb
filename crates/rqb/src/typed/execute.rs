use sqlx::postgres::PgRow;
use sqlx::{Decode, FromRow, PgExecutor, Postgres, Type};

use crate::Result;
use crate::typed::{
    BuiltQuery, Delete, Insert, Merge, RawStmt, Select, SetQuery, Stmt, Update, count_all, select,
    subquery,
};

/// Rust value that can be decoded from a single Postgres result column.
///
/// This is the read-side pair to [`crate::BindValue`]: it keeps sqlx scalar
/// decode bounds out of user-facing method signatures.
pub trait ScalarValue: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin {}

impl<T> ScalarValue for T where T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin {}

impl BuiltQuery {
    /// Executes the query and returns affected row count.
    pub async fn execute<'e>(&self, executor: impl PgExecutor<'e>) -> Result<u64> {
        let result = sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
    }

    /// Fetches all rows as raw sqlx `PgRow` values.
    pub async fn fetch_all<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Vec<PgRow>> {
        sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches exactly one raw sqlx `PgRow`.
    pub async fn fetch_one<'e>(&self, executor: impl PgExecutor<'e>) -> Result<PgRow> {
        sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches zero or one raw sqlx `PgRow`.
    pub async fn fetch_optional<'e>(&self, executor: impl PgExecutor<'e>) -> Result<Option<PgRow>> {
        sqlx::query_with(&self.sql, self.arguments()?)
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
        sqlx::query_as_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches exactly one row into a `sqlx::FromRow` type.
    pub async fn fetch_one_as<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
    where
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(&self.sql, self.arguments()?)
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
        sqlx::query_as_with::<_, T, _>(&self.sql, self.arguments()?)
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
        sqlx::query_scalar_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    /// Fetches exactly one decoded scalar value.
    pub async fn fetch_one_scalar<'e, T>(&self, executor: impl PgExecutor<'e>) -> Result<T>
    where
        T: ScalarValue,
    {
        sqlx::query_scalar_with::<_, T, _>(&self.sql, self.arguments()?)
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
        sqlx::query_scalar_with::<_, T, _>(&self.sql, self.arguments()?)
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
}

impl Select {
    /// Executes a matching `count(*)` query for this select.
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
