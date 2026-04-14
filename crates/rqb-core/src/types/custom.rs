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
    Time,
    Timetz,
    Interval,
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

impl ValueRepr {
    pub const fn is_string_backed(self) -> bool {
        matches!(self, Self::String | Self::DecimalString)
    }

    pub const fn is_decimal_string(self) -> bool {
        matches!(self, Self::DecimalString)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectRepr {
    Native,
    Text,
}

impl SelectRepr {
    pub const fn is_text(self) -> bool {
        matches!(self, Self::Text)
    }
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

    pub const fn value_is_string_backed(self) -> bool {
        self.value_repr.is_string_backed()
    }

    pub const fn value_is_decimal_string(self) -> bool {
        self.value_repr.is_decimal_string()
    }

    pub const fn selects_as_text(self) -> bool {
        self.select_repr.is_text()
    }

    pub fn display_name(self) -> String {
        match self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_and_select_repr_helpers_describe_representation() {
        assert!(ValueRepr::String.is_string_backed());
        assert!(ValueRepr::DecimalString.is_string_backed());
        assert!(!ValueRepr::Native.is_string_backed());
        assert!(ValueRepr::DecimalString.is_decimal_string());
        assert!(!ValueRepr::String.is_decimal_string());

        assert!(SelectRepr::Text.is_text());
        assert!(!SelectRepr::Native.is_text());
    }

    #[test]
    fn type_spec_delegates_representation_helpers() {
        const MONEY: TypeSpec = TypeSpec::domain(Some("public"), "money_256")
            .base(TypeFamily::Numeric)
            .value_repr(ValueRepr::DecimalString)
            .select_repr(SelectRepr::Text);

        assert!(MONEY.value_is_string_backed());
        assert!(MONEY.value_is_decimal_string());
        assert!(MONEY.selects_as_text());
    }
}
