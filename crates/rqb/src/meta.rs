/// JSON value kind accepted by a field in [`SearchRequest`](crate::SearchRequest).
///
/// This describes the client wire shape, not the SQL type itself. For example,
/// `NumericString` accepts a JSON string so decimal values do not lose
/// precision while being decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum JsonKind {
    /// JSON string value.
    Text,
    /// JSON boolean value.
    Bool,
    /// JSON integer value that fits in `i32`.
    Integer,
    /// JSON integer value that fits in `i64`.
    BigInt,
    /// JSON floating-point value.
    Float,
    /// Decimal value encoded as a JSON string.
    NumericString,
    /// UUID string value.
    Uuid,
    /// Date string value.
    Date,
    /// Time string value.
    Time,
    /// Timestamp without timezone string value.
    Timestamp,
    /// Timestamp with timezone string value.
    Timestamptz,
    /// Arbitrary JSONB value.
    Jsonb,
}

/// Operator capability flags for a field.
///
/// These flags are the source of truth for JSON search operators. They also
/// drive typed builder validation: equality gates equality/list/subquery
/// predicates, ordering gates range predicates and sorting, and pattern gates
/// text pattern predicates. Rust-side null checks remain valid for any field
/// whose inner expression is valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpSet {
    /// Whether equality-style operators are valid.
    pub equality: bool,
    /// Whether ordering-style operators are valid.
    pub ordering: bool,
    /// Whether text pattern operators such as `LIKE`, `ILIKE`, and regex are valid.
    pub pattern: bool,
}

impl OpSet {
    /// No typed comparison or sorting operators are allowed.
    pub const fn none() -> Self {
        Self {
            equality: false,
            ordering: false,
            pattern: false,
        }
    }

    /// Equality operators are allowed, ordering operators are not.
    pub const fn equality() -> Self {
        Self {
            equality: true,
            ordering: false,
            pattern: false,
        }
    }

    /// Equality, ordering, and sorting operators are allowed.
    pub const fn ordered() -> Self {
        Self {
            equality: true,
            ordering: true,
            pattern: false,
        }
    }

    /// Equality, ordering, sorting, and text pattern operators are allowed.
    pub const fn text() -> Self {
        Self {
            equality: true,
            ordering: true,
            pattern: true,
        }
    }
}

/// Field metadata used for validation, rendering, and JSON search exposure.
///
/// Generated schema stores one `Meta` per database column. `ops` controls which
/// operators are valid, while `json` decides whether a field is visible to
/// client-supplied [`SearchRequest`](crate::SearchRequest) values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Meta {
    /// API-facing field name used by JSON search.
    pub api: &'static str,
    /// Database column name rendered into SQL.
    pub db: &'static str,
    /// Postgres type name used by generated schema and diagnostics.
    pub pg: &'static str,
    /// Typed operator capability flags.
    pub ops: OpSet,
    /// JSON search exposure, if the field is visible to `SearchRequest`.
    pub json: Option<JsonKind>,
}

impl Meta {
    /// Creates field metadata with separate API and database names.
    pub const fn new(api: &'static str, db: &'static str, pg: &'static str) -> Self {
        Self {
            api,
            db,
            pg,
            ops: OpSet::none(),
            json: None,
        }
    }

    /// Creates field metadata where API and database names are the same.
    pub const fn col(name: &'static str, pg: &'static str) -> Self {
        Self::new(name, name, pg)
    }

    /// Marks the field as visible to JSON search with the given JSON kind.
    ///
    /// This only controls JSON request decoding/exposure; it does not change
    /// the SQL type rendered for the field.
    pub const fn json(mut self, kind: JsonKind) -> Self {
        self.json = Some(kind);
        self
    }

    /// Sets typed operator capability flags.
    pub const fn ops(mut self, ops: OpSet) -> Self {
        self.ops = ops;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonKind, Meta, OpSet};

    static CONST_META: Meta = Meta::new("email", "email_address", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);

    #[test]
    fn meta_new_defaults_to_no_ops_and_no_json_exposure() {
        let meta = Meta::new("id", "user_id", "uuid");

        assert_eq!(meta.api, "id");
        assert_eq!(meta.db, "user_id");
        assert_eq!(meta.pg, "uuid");
        assert_eq!(meta.ops, OpSet::none());
        assert_eq!(meta.json, None);
    }

    #[test]
    fn meta_col_uses_same_api_and_db_name() {
        let meta = Meta::col("created_at", "timestamptz");

        assert_eq!(meta.api, "created_at");
        assert_eq!(meta.db, "created_at");
        assert_eq!(meta.pg, "timestamptz");
    }

    #[test]
    fn meta_builder_methods_replace_existing_values() {
        let meta = Meta::col("score", "int4")
            .ops(OpSet::equality())
            .ops(OpSet::ordered())
            .json(JsonKind::Integer)
            .json(JsonKind::BigInt);

        assert_eq!(meta.ops, OpSet::ordered());
        assert_eq!(meta.json, Some(JsonKind::BigInt));
    }

    #[test]
    fn op_sets_express_exact_capabilities() {
        assert_eq!(
            OpSet::none(),
            OpSet {
                equality: false,
                ordering: false,
                pattern: false,
            }
        );
        assert_eq!(
            OpSet::equality(),
            OpSet {
                equality: true,
                ordering: false,
                pattern: false,
            }
        );
        assert_eq!(
            OpSet::ordered(),
            OpSet {
                equality: true,
                ordering: true,
                pattern: false,
            }
        );
        assert_eq!(
            OpSet::text(),
            OpSet {
                equality: true,
                ordering: true,
                pattern: true,
            }
        );
    }

    #[test]
    fn all_json_kind_variants_are_static_metadata_friendly() {
        const KINDS: [JsonKind; 12] = [
            JsonKind::Text,
            JsonKind::Bool,
            JsonKind::Integer,
            JsonKind::BigInt,
            JsonKind::Float,
            JsonKind::NumericString,
            JsonKind::Uuid,
            JsonKind::Date,
            JsonKind::Time,
            JsonKind::Timestamp,
            JsonKind::Timestamptz,
            JsonKind::Jsonb,
        ];

        assert_eq!(KINDS.len(), 12);
        assert_eq!(CONST_META.json, Some(JsonKind::Text));
    }
}
