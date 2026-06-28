use super::*;

impl RawStmt {
    /// Creates a server-owned raw SQL statement.
    ///
    /// Use `?` for rqb bind placeholders and `??` for a literal question mark,
    /// for example PostgreSQL JSONB `?` operators. Bind-count mismatches fail
    /// validation before rendering or execution.
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    /// Adds one bind value for a `?` placeholder in the raw SQL text.
    ///
    /// Placeholders inside SQL strings, quoted identifiers, dollar quotes, and
    /// comments are ignored by the raw placeholder scanner.
    pub fn bind<T>(mut self, value: T) -> Self
    where
        T: BindValue,
    {
        self.params.push(Param::typed(value));
        self
    }
}
