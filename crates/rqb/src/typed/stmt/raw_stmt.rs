use super::*;

impl RawStmt {
    /// Creates a server-owned raw SQL statement.
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    /// Adds one bind value for a `?` placeholder in the raw SQL text.
    pub fn bind<T>(mut self, value: T) -> Self
    where
        T: BindValue,
    {
        self.params.push(Param::typed(value));
        self
    }
}
