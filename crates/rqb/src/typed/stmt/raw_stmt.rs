use super::*;

impl RawStmt {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    pub fn bind<T>(mut self, value: T) -> Self
    where
        T: Clone
            + Send
            + Sync
            + 'static
            + for<'q> sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>,
    {
        self.params.push(Param::typed(value));
        self
    }
}
