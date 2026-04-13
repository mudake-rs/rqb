use serde::Serialize;

use crate::value::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumType {
    pub schema: Option<&'static str>,
    pub name: &'static str,
    pub variants: &'static [&'static str],
}

impl EnumType {
    pub const fn new(
        schema: Option<&'static str>,
        name: &'static str,
        variants: &'static [&'static str],
    ) -> Self {
        Self {
            schema,
            name,
            variants,
        }
    }

    pub fn contains(self, value: &str) -> bool {
        self.variants.contains(&value)
    }

    pub fn display_name(self) -> String {
        match self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.to_owned(),
        }
    }

    pub fn allowed_values(self) -> String {
        self.variants.join(", ")
    }
}

pub trait DbEnum: Copy {
    const TYPE: EnumType;

    fn as_db_str(self) -> &'static str;
}

impl<T> From<T> for Value
where
    T: DbEnum,
{
    fn from(value: T) -> Self {
        Self::String(value.as_db_str().to_owned())
    }
}
