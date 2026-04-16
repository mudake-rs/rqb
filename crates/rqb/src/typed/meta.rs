#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonKind {
    Text,
    Bool,
    Integer,
    BigInt,
    Float,
    NumericString,
    Uuid,
    Date,
    Time,
    Timestamp,
    Timestamptz,
    Jsonb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpSet {
    pub equality: bool,
    pub ordering: bool,
}

impl OpSet {
    pub const fn none() -> Self {
        Self {
            equality: false,
            ordering: false,
        }
    }

    pub const fn equality() -> Self {
        Self {
            equality: true,
            ordering: false,
        }
    }

    pub const fn ordered() -> Self {
        Self {
            equality: true,
            ordering: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meta {
    pub api: &'static str,
    pub db: &'static str,
    pub pg: &'static str,
    pub ops: OpSet,
    pub json: Option<JsonKind>,
}

impl Meta {
    pub const fn new(api: &'static str, db: &'static str, pg: &'static str) -> Self {
        Self {
            api,
            db,
            pg,
            ops: OpSet::none(),
            json: None,
        }
    }

    pub const fn col(name: &'static str, pg: &'static str) -> Self {
        Self::new(name, name, pg)
    }

    pub const fn json(mut self, kind: JsonKind) -> Self {
        self.json = Some(kind);
        self
    }

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
            }
        );
        assert_eq!(
            OpSet::equality(),
            OpSet {
                equality: true,
                ordering: false,
            }
        );
        assert_eq!(
            OpSet::ordered(),
            OpSet {
                equality: true,
                ordering: true,
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
