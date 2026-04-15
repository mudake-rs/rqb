use sqlx::{Encode, Postgres, Type, postgres::PgHasArrayType};

use crate::typed::Param;

use super::{BoolExpr, Field, FieldRef, ValueExpr};

impl<T> Field<Vec<T>>
where
    T: Clone
        + Send
        + Sync
        + 'static
        + for<'q> Encode<'q, Postgres>
        + Type<Postgres>
        + PgHasArrayType,
{
    pub fn contains_any(self, values: Vec<T>) -> BoolExpr {
        self.reference().contains_any(values)
    }

    pub fn contains_all(self, values: Vec<T>) -> BoolExpr {
        self.reference().contains_all(values)
    }

    pub fn contained_by(self, values: Vec<T>) -> BoolExpr {
        self.reference().contained_by(values)
    }

    pub fn has(self, value: T) -> BoolExpr {
        self.reference().has(value)
    }

    pub fn not_has(self, value: T) -> BoolExpr {
        self.reference().not_has(value)
    }

    pub fn is_empty(self) -> BoolExpr {
        self.reference().is_empty()
    }

    pub fn is_not_empty(self) -> BoolExpr {
        self.reference().is_not_empty()
    }
}

impl<T> FieldRef<Vec<T>>
where
    T: Clone
        + Send
        + Sync
        + 'static
        + for<'q> Encode<'q, Postgres>
        + Type<Postgres>
        + PgHasArrayType,
{
    pub fn contains_any(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("&&", values, false)
    }

    pub fn contains_all(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("@>", values, false)
    }

    pub fn contained_by(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("<@", values, false)
    }

    pub fn has(self, value: T) -> BoolExpr {
        self.any_predicate(value, false)
    }

    pub fn not_has(self, value: T) -> BoolExpr {
        self.any_predicate(value, true)
    }

    pub fn is_empty(self) -> BoolExpr {
        BoolExpr::ArrayIsEmpty {
            expr: self.expr(),
            negated: false,
        }
    }

    pub fn is_not_empty(self) -> BoolExpr {
        BoolExpr::ArrayIsEmpty {
            expr: self.expr(),
            negated: true,
        }
    }

    fn array_infix(self, op: &'static str, values: Vec<T>, negated: bool) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(values)),
            negated,
        }
    }

    fn any_predicate(self, value: T, negated: bool) -> BoolExpr {
        BoolExpr::Any {
            value: ValueExpr::Param(Param::typed(value)),
            array: self.expr(),
            negated,
        }
    }
}

impl Field<serde_json::Value> {
    pub fn contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_contains(value)
    }

    pub fn contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_contained_by(value)
    }

    pub fn key_exists(self, key: impl Into<String>) -> BoolExpr {
        self.reference().key_exists(key)
    }

    pub fn keys_exist_any(self, keys: Vec<String>) -> BoolExpr {
        self.reference().keys_exist_any(keys)
    }

    pub fn keys_exist_all(self, keys: Vec<String>) -> BoolExpr {
        self.reference().keys_exist_all(keys)
    }

    pub fn json_contains(self, value: serde_json::Value) -> BoolExpr {
        self.reference().json_contains(value)
    }

    pub fn json_contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.reference().json_contained_by(value)
    }
}

impl<T> Field<sqlx::postgres::types::PgRange<T>>
where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    sqlx::postgres::types::PgRange<T>:
        Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
{
    pub fn contains(self, value: T) -> BoolExpr {
        self.range_contains(value)
    }

    pub fn range_contains(self, value: T) -> BoolExpr {
        self.reference().range_contains(value)
    }

    pub fn contains_range(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().contains_range(value)
    }

    pub fn contained_by(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().contained_by(value)
    }

    pub fn overlaps(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().overlaps(value)
    }
}

impl<T> FieldRef<sqlx::postgres::types::PgRange<T>>
where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    sqlx::postgres::types::PgRange<T>:
        Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
{
    pub fn contains(self, value: T) -> BoolExpr {
        self.range_contains(value)
    }

    pub fn range_contains(self, value: T) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op: "@>",
            right: ValueExpr::Param(Param::typed(value)),
            negated: false,
        }
    }

    pub fn contains_range(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("@>", value)
    }

    pub fn contained_by(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("<@", value)
    }

    pub fn overlaps(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("&&", value)
    }

    fn range_infix(self, op: &'static str, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(value)),
            negated: false,
        }
    }
}

impl FieldRef<serde_json::Value> {
    pub fn contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_contains(value)
    }

    pub fn contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_contained_by(value)
    }

    pub fn key_exists(self, key: impl Into<String>) -> BoolExpr {
        self.json_infix("?", Param::typed(key.into()))
    }

    pub fn keys_exist_any(self, keys: Vec<String>) -> BoolExpr {
        self.json_infix("?|", Param::typed(keys))
    }

    pub fn keys_exist_all(self, keys: Vec<String>) -> BoolExpr {
        self.json_infix("?&", Param::typed(keys))
    }

    pub fn json_contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_infix("@>", Param::typed(value))
    }

    pub fn json_contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_infix("<@", Param::typed(value))
    }

    fn json_infix(self, op: &'static str, param: Param) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(param),
            negated: false,
        }
    }
}
