use serde::{Deserialize, Serialize};

use crate::aggregate::Aggregate;
use crate::dataset::{Cte, Dataset, Join};
use crate::error::Error;
use crate::expr::{Expr, Sort};
use crate::field::FieldRef;
use crate::sql_expr::SelectItem;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[must_use]
pub struct SearchRequest {
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub sort: Vec<Sort>,
    #[serde(default)]
    pub filter: Option<Expr>,
}

impl SearchRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(mut self, request: Self) -> Self {
        self.merge_in(request);
        self
    }

    pub fn merge_in(&mut self, request: Self) {
        if !request.sort.is_empty() {
            self.sort = request.sort;
        }
        if request.limit.is_some() {
            self.limit = request.limit;
        }
        if request.offset.is_some() {
            self.offset = request.offset;
        }
        self.filter = match (self.filter.take(), request.filter) {
            (Some(existing), Some(incoming)) => Some(existing.and(incoming)),
            (Some(existing), None) => Some(existing),
            (None, Some(incoming)) => Some(incoming),
            (None, None) => None,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    Update,
    NoKeyUpdate,
    Share,
    KeyShare,
}

impl LockMode {
    pub fn as_sql(self) -> &'static str {
        match self {
            Self::Update => "FOR UPDATE",
            Self::NoKeyUpdate => "FOR NO KEY UPDATE",
            Self::Share => "FOR SHARE",
            Self::KeyShare => "FOR KEY SHARE",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockWait {
    #[default]
    Wait,
    NoWait,
    SkipLocked,
}

impl LockWait {
    pub fn as_sql(self) -> Option<&'static str> {
        match self {
            Self::Wait => None,
            Self::NoWait => Some("NOWAIT"),
            Self::SkipLocked => Some("SKIP LOCKED"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct RowLock {
    pub mode: LockMode,
    pub wait: LockWait,
}

impl RowLock {
    pub const fn new(mode: LockMode) -> Self {
        Self {
            mode,
            wait: LockWait::Wait,
        }
    }

    pub const fn nowait(mut self) -> Self {
        self.wait = LockWait::NoWait;
        self
    }

    pub const fn skip_locked(mut self) -> Self {
        self.wait = LockWait::SkipLocked;
        self
    }
}

impl Default for RowLock {
    fn default() -> Self {
        Self::new(LockMode::Update)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct SelectQuery {
    pub dataset: Dataset,
    pub joins: Vec<Join>,
    pub projection: Vec<FieldRef>,
    pub request: SearchRequest,
    pub cacheable: bool,
    pub ctes: Vec<Cte>,
    pub distinct: bool,
    pub distinct_on: Vec<FieldRef>,
    pub group_by: Vec<FieldRef>,
    pub aggregates: Vec<Aggregate>,
    pub select_items: Vec<SelectItem>,
    pub having: Option<Expr>,
    pub lock: Option<RowLock>,
    pub builder_errors: Vec<Error>,
}

impl SelectQuery {
    pub fn new(dataset: impl Into<Dataset>) -> Self {
        Self {
            dataset: dataset.into(),
            joins: Vec::new(),
            projection: Vec::new(),
            request: SearchRequest::new(),
            cacheable: true,
            ctes: Vec::new(),
            distinct: false,
            distinct_on: Vec::new(),
            group_by: Vec::new(),
            aggregates: Vec::new(),
            select_items: Vec::new(),
            having: None,
            lock: None,
            builder_errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{LogicalExpr, LogicalOp, SortDir, field};

    use super::*;

    #[test]
    fn search_request_merge_replaces_page_sort_and_ands_filters() {
        let base = SearchRequest {
            offset: Some(10),
            limit: Some(20),
            sort: vec![field("createdAt").desc()],
            filter: Some(field("status").eq("paid")),
        };
        let incoming = SearchRequest {
            offset: Some(0),
            limit: Some(5),
            sort: vec![field("email").asc()],
            filter: Some(field("email").contains("@example.com")),
        };

        let merged = base.merge(incoming);

        assert_eq!(merged.offset, Some(0));
        assert_eq!(merged.limit, Some(5));
        assert_eq!(merged.sort[0].field, field("email"));
        assert_eq!(merged.sort[0].dir, SortDir::Asc);
        assert!(matches!(
            merged.filter,
            Some(Expr::Logical(LogicalExpr {
                logical: LogicalOp::And,
                predicates,
            })) if predicates.len() == 2
        ));
    }

    #[test]
    fn search_request_merge_keeps_existing_values_when_incoming_is_empty() {
        let base = SearchRequest {
            offset: Some(10),
            limit: Some(20),
            sort: vec![field("createdAt").desc()],
            filter: Some(field("status").eq("paid")),
        };

        let merged = base.clone().merge(SearchRequest::new());

        assert_eq!(merged, base);
    }

    #[test]
    fn search_request_json_uses_filter_and_lowercase_sort_direction() {
        let request: SearchRequest = serde_json::from_value(serde_json::json!({
            "sort": [{ "field": "createdAt", "dir": "desc" }],
            "filter": {
                "and": [
                    { "field": "status", "operator": "equals", "value": "paid" },
                    { "field": "email", "operator": "contains", "value": "@example.com" }
                ]
            }
        }))
        .unwrap();

        assert!(request.filter.is_some());
        assert_eq!(request.sort[0].dir, SortDir::Desc);
    }

    #[test]
    fn search_request_json_rejects_legacy_query_and_uppercase_sort_direction() {
        let legacy_query = serde_json::from_value::<SearchRequest>(serde_json::json!({
            "query": { "field": "status", "operator": "equals", "value": "paid" }
        }));
        assert!(legacy_query.is_err());

        let uppercase_dir = serde_json::from_value::<SearchRequest>(serde_json::json!({
            "sort": [{ "field": "createdAt", "dir": "DESC" }]
        }));
        assert!(uppercase_dir.is_err());
    }

    #[test]
    fn search_request_json_rejects_fields() {
        let result = serde_json::from_value::<SearchRequest>(serde_json::json!({
            "fields": ["id"]
        }));
        assert!(result.is_err());
    }
}
