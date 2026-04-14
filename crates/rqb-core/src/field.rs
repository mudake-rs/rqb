mod capabilities;
mod reference;
mod resolved;

use crate::expr::{Expr, Sort, SortDir};
use crate::query::QueryExpr;
use crate::sql_expr::SqlExpr;
use crate::types::FieldType;
use crate::value::Value;

pub use capabilities::{Capabilities, JsonPathPolicy, TextSearchConfig};
pub use reference::FieldRef;
pub use resolved::ResolvedField;

macro_rules! delegate_value_ops {
    ($($method:ident),* $(,)?) => {
        $(
            pub fn $method(self, value: impl Into<Value>) -> Expr {
                FieldRef::from(self).$method(value)
            }
        )*
    };
}

macro_rules! delegate_col_ops {
    ($($method:ident),* $(,)?) => {
        $(
            pub fn $method(self, right: impl Into<FieldRef>) -> Expr {
                FieldRef::from(self).$method(right)
            }
        )*
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub api_name: &'static str,
    pub db_name: &'static str,
    pub ty: FieldType,
    pub caps: Capabilities,
}

impl Field {
    pub const fn new(name: &'static str, ty: FieldType) -> Self {
        Self::mapped(name, name, ty)
    }

    pub const fn mapped(api_name: &'static str, db_name: &'static str, ty: FieldType) -> Self {
        Self {
            api_name,
            db_name,
            ty,
            caps: Capabilities::all(),
        }
    }

    pub const fn selectable(mut self, selectable: bool) -> Self {
        self.caps.selectable = selectable;
        self
    }

    pub const fn sortable(mut self, sortable: bool) -> Self {
        self.caps.sortable = sortable;
        self
    }

    pub const fn filterable(mut self, filterable: bool) -> Self {
        self.caps.filterable = filterable;
        self
    }

    pub const fn json_paths(mut self, policy: JsonPathPolicy) -> Self {
        self.caps.json_path = policy;
        self
    }

    pub const fn text_search(mut self, config: &'static str) -> Self {
        self.caps.text_search = TextSearchConfig::Config(config);
        self
    }

    pub fn path(self, path: impl Into<String>) -> FieldRef {
        FieldRef::Known {
            qualifier: None,
            field: self,
            path: vec![path.into()],
            alias: None,
        }
    }

    pub fn on(self, qualifier: impl Into<String>) -> FieldRef {
        FieldRef::Known {
            qualifier: Some(qualifier.into()),
            field: self,
            path: Vec::new(),
            alias: None,
        }
    }

    pub fn alias(self, alias: impl Into<String>) -> FieldRef {
        FieldRef::from(self).alias(alias)
    }

    pub fn expr(self) -> SqlExpr {
        FieldRef::from(self).expr()
    }

    delegate_value_ops!(
        eq,
        ne,
        gt,
        gte,
        lt,
        lte,
        contains,
        not_contains,
        not_in,
        not_starts_with,
        not_ends_with,
        is_distinct_from,
        is_not_distinct_from,
        starts_with,
        ends_with,
        is_in,
        contains_any,
        contains_all,
        elem_match,
        has,
        not_has,
        key_exists,
        keys_exist_any,
        keys_exist_all,
        contained_by,
        overlaps,
        regex,
        not_regex,
        search,
    );

    pub fn between(self, low: impl Into<Value>, high: impl Into<Value>) -> Expr {
        FieldRef::from(self).between(low, high)
    }

    pub fn not_between(self, low: impl Into<Value>, high: impl Into<Value>) -> Expr {
        FieldRef::from(self).not_between(low, high)
    }

    pub fn is_null(self) -> Expr {
        FieldRef::from(self).is_null()
    }

    pub fn is_not_null(self) -> Expr {
        FieldRef::from(self).is_not_null()
    }

    pub fn is_empty(self) -> Expr {
        FieldRef::from(self).is_empty()
    }

    pub fn is_not_empty(self) -> Expr {
        FieldRef::from(self).is_not_empty()
    }

    delegate_col_ops!(eq_col, ne_col, gt_col, gte_col, lt_col, lte_col);

    pub fn in_subquery(self, query: impl Into<QueryExpr>) -> Expr {
        FieldRef::from(self).in_subquery(query)
    }

    pub fn not_in_subquery(self, query: impl Into<QueryExpr>) -> Expr {
        FieldRef::from(self).not_in_subquery(query)
    }

    pub fn asc(self) -> Sort {
        Sort::new(self, SortDir::Asc)
    }

    pub fn desc(self) -> Sort {
        Sort::new(self, SortDir::Desc)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn field_ref_display_and_serde_keep_known_qualified_paths() {
        let field = Field::new("metadata", FieldType::Jsonb)
            .on("o")
            .path("campaign");

        assert_eq!(field.qualifier(), Some("o"));
        assert_eq!(field.display_name(), "o.metadata.campaign");
        assert_eq!(serde_json::to_value(&field).unwrap(), "o.metadata.campaign");

        let decoded = serde_json::from_value::<FieldRef>(serde_json::json!("u.email")).unwrap();
        assert_eq!(decoded.qualifier(), Some("u"));
        assert_eq!(decoded.display_name(), "u.email");
    }

    #[test]
    fn resolved_field_output_alias_prefers_explicit_alias() {
        let field = ResolvedField {
            api_name: "totalCents".to_owned(),
            db_name: "total_cents".to_owned(),
            ty: FieldType::BigInt,
            caps: Capabilities::all(),
            json_path: Vec::new(),
            qualifier: Some("o".to_owned()),
            explicit_qualifier: Some("o".to_owned()),
            alias: None,
        };

        assert_eq!(field.display_name(), "o.totalCents");
        assert_eq!(field.output_alias(), "o_totalCents");
        assert_eq!(field.object_key(), "totalCents");

        let aliased = ResolvedField {
            alias: Some("total".to_owned()),
            ..field
        };
        assert_eq!(aliased.output_alias(), "total");
        assert_eq!(aliased.object_key(), "total");
    }
}
