use serde::{Deserialize, Serialize};

use crate::value::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct RawSql {
    pub sql: String,
    #[serde(default)]
    pub binds: Vec<Value>,
}

pub fn raw(sql: impl Into<String>) -> RawSql {
    RawSql::new(sql)
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
}
