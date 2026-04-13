use serde::{Deserialize, Serialize};

use crate::value::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct RawSql {
    pub sql: String,
    #[serde(default)]
    pub binds: Vec<Value>,
}

/// Top-level raw SQL query with bind parameters.
///
/// Placeholders use `?` syntax. Postgres rendering converts them to `$1`,
/// `$2`, and so on. Use `??` for a literal question mark.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct RawQuery {
    inner: RawSql,
}

pub fn raw(sql: impl Into<String>) -> RawSql {
    RawSql::new(sql)
}

pub fn raw_query(sql: impl Into<String>) -> RawQuery {
    RawQuery::new(sql)
}

impl RawSql {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            binds: Vec::new(),
        }
    }

    pub fn bind(mut self, value: impl Into<Value>) -> Self {
        self.binds.push(value.into());
        self
    }

    pub fn placeholder_count(&self) -> usize {
        count_placeholders(&self.sql)
    }
}

impl RawQuery {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            inner: RawSql::new(sql),
        }
    }

    pub fn bind(mut self, value: impl Into<Value>) -> Self {
        self.inner.binds.push(value.into());
        self
    }

    pub fn sql(&self) -> &str {
        &self.inner.sql
    }

    pub fn binds(&self) -> &[Value] {
        &self.inner.binds
    }

    pub fn as_raw_sql(&self) -> &RawSql {
        &self.inner
    }
}

pub(crate) fn count_placeholders(sql: &str) -> usize {
    let mut count = 0;
    let mut chars = sql.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '?' {
            if chars.peek() == Some(&'?') {
                chars.next();
            } else {
                count += 1;
            }
        }
    }
    count
}
