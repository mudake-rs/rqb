use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeFamily {
    Text,
    Numeric,
    Bool,
    Uuid,
    Timestamp,
    Timestamptz,
    Date,
    Jsonb,
    Bytes,
    Network,
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueRepr {
    Native,
    String,
    DecimalString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectRepr {
    Native,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeSpec {
    pub schema: Option<&'static str>,
    pub name: &'static str,
    pub family: TypeFamily,
    pub value_repr: ValueRepr,
    pub select_repr: SelectRepr,
}

impl TypeSpec {
    pub const fn domain(schema: Option<&'static str>, name: &'static str) -> Self {
        Self {
            schema,
            name,
            family: TypeFamily::Text,
            value_repr: ValueRepr::String,
            select_repr: SelectRepr::Text,
        }
    }

    pub const fn base(mut self, family: TypeFamily) -> Self {
        self.family = family;
        self
    }

    pub const fn value_repr(mut self, value_repr: ValueRepr) -> Self {
        self.value_repr = value_repr;
        self
    }

    pub const fn select_repr(mut self, select_repr: SelectRepr) -> Self {
        self.select_repr = select_repr;
        self
    }

    pub fn display_name(self) -> String {
        match self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.to_owned(),
        }
    }
}
