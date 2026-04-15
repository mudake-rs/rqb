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
