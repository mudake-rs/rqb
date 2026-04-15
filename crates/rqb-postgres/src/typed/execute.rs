use sqlx::postgres::PgRow;
use sqlx::{Decode, Executor, FromRow, Postgres, Type};

use crate::Result;
use crate::typed::{BuiltQuery, Delete, Insert, RawStmt, Select, Stmt, Update};

impl BuiltQuery {
    pub async fn execute<'e, E>(&self, executor: E) -> Result<u64>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn fetch_all<'e, E>(&self, executor: E) -> Result<Vec<PgRow>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_one<'e, E>(&self, executor: E) -> Result<PgRow>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_optional<'e, E>(&self, executor: E) -> Result<Option<PgRow>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_with(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_all_as<'e, E, T>(&self, executor: E) -> Result<Vec<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_one_as<'e, E, T>(&self, executor: E) -> Result<T>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_optional_as<'e, E, T>(&self, executor: E) -> Result<Option<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        sqlx::query_as_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_scalar<'e, E, T>(&self, executor: E) -> Result<Vec<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
    {
        sqlx::query_scalar_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_one_scalar<'e, E, T>(&self, executor: E) -> Result<T>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
    {
        sqlx::query_scalar_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_one(executor)
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_optional_scalar<'e, E, T>(&self, executor: E) -> Result<Option<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
    {
        sqlx::query_scalar_with::<_, T, _>(&self.sql, self.arguments()?)
            .persistent(self.cacheable)
            .fetch_optional(executor)
            .await
            .map_err(Into::into)
    }
}

impl Stmt {
    pub async fn execute<'e, E>(&self, executor: E) -> Result<u64>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.build()?.execute(executor).await
    }

    pub async fn fetch_all<'e, E>(&self, executor: E) -> Result<Vec<PgRow>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.build()?.fetch_all(executor).await
    }

    pub async fn fetch_one<'e, E>(&self, executor: E) -> Result<PgRow>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.build()?.fetch_one(executor).await
    }

    pub async fn fetch_optional<'e, E>(&self, executor: E) -> Result<Option<PgRow>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.build()?.fetch_optional(executor).await
    }

    pub async fn fetch_all_as<'e, E, T>(&self, executor: E) -> Result<Vec<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        self.build()?.fetch_all_as(executor).await
    }

    pub async fn fetch_one_as<'e, E, T>(&self, executor: E) -> Result<T>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        self.build()?.fetch_one_as(executor).await
    }

    pub async fn fetch_optional_as<'e, E, T>(&self, executor: E) -> Result<Option<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    {
        self.build()?.fetch_optional_as(executor).await
    }

    pub async fn fetch_scalar<'e, E, T>(&self, executor: E) -> Result<Vec<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
    {
        self.build()?.fetch_scalar(executor).await
    }

    pub async fn fetch_one_scalar<'e, E, T>(&self, executor: E) -> Result<T>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
    {
        self.build()?.fetch_one_scalar(executor).await
    }

    pub async fn fetch_optional_scalar<'e, E, T>(&self, executor: E) -> Result<Option<T>>
    where
        E: Executor<'e, Database = Postgres>,
        T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
    {
        self.build()?.fetch_optional_scalar(executor).await
    }
}

macro_rules! impl_statement_execute {
    ($ty:ty) => {
        impl $ty {
            pub async fn execute<'e, E>(&self, executor: E) -> Result<u64>
            where
                E: Executor<'e, Database = Postgres>,
            {
                self.build()?.execute(executor).await
            }

            pub async fn fetch_all<'e, E>(&self, executor: E) -> Result<Vec<PgRow>>
            where
                E: Executor<'e, Database = Postgres>,
            {
                self.build()?.fetch_all(executor).await
            }

            pub async fn fetch_one<'e, E>(&self, executor: E) -> Result<PgRow>
            where
                E: Executor<'e, Database = Postgres>,
            {
                self.build()?.fetch_one(executor).await
            }

            pub async fn fetch_optional<'e, E>(&self, executor: E) -> Result<Option<PgRow>>
            where
                E: Executor<'e, Database = Postgres>,
            {
                self.build()?.fetch_optional(executor).await
            }

            pub async fn fetch_all_as<'e, E, T>(&self, executor: E) -> Result<Vec<T>>
            where
                E: Executor<'e, Database = Postgres>,
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
            {
                self.build()?.fetch_all_as(executor).await
            }

            pub async fn fetch_one_as<'e, E, T>(&self, executor: E) -> Result<T>
            where
                E: Executor<'e, Database = Postgres>,
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
            {
                self.build()?.fetch_one_as(executor).await
            }

            pub async fn fetch_optional_as<'e, E, T>(&self, executor: E) -> Result<Option<T>>
            where
                E: Executor<'e, Database = Postgres>,
                T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
            {
                self.build()?.fetch_optional_as(executor).await
            }

            pub async fn fetch_scalar<'e, E, T>(&self, executor: E) -> Result<Vec<T>>
            where
                E: Executor<'e, Database = Postgres>,
                T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
            {
                self.build()?.fetch_scalar(executor).await
            }

            pub async fn fetch_one_scalar<'e, E, T>(&self, executor: E) -> Result<T>
            where
                E: Executor<'e, Database = Postgres>,
                T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
            {
                self.build()?.fetch_one_scalar(executor).await
            }

            pub async fn fetch_optional_scalar<'e, E, T>(&self, executor: E) -> Result<Option<T>>
            where
                E: Executor<'e, Database = Postgres>,
                T: for<'r> Decode<'r, Postgres> + Type<Postgres> + Send + Unpin,
            {
                self.build()?.fetch_optional_scalar(executor).await
            }
        }
    };
}

impl_statement_execute!(Select);
impl_statement_execute!(Insert);
impl_statement_execute!(Update);
impl_statement_execute!(Delete);
impl_statement_execute!(RawStmt);
