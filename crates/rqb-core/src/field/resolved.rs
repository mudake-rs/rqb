use std::borrow::Cow;

use crate::types::FieldType;

use super::Capabilities;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedField {
    pub api_name: Cow<'static, str>,
    pub db_name: Cow<'static, str>,
    pub ty: FieldType,
    pub caps: Capabilities,
    pub json_path: Vec<String>,
    pub qualifier: Option<String>,
    pub explicit_qualifier: Option<String>,
    pub alias: Option<String>,
}

impl ResolvedField {
    pub fn display_name(&self) -> String {
        let name = if self.json_path.is_empty() {
            self.api_name.to_string()
        } else {
            format!("{}.{}", self.api_name, self.json_path.join("."))
        };
        match &self.explicit_qualifier {
            Some(qualifier) => format!("{qualifier}.{name}"),
            None => name,
        }
    }

    #[inline]
    pub fn is_json_path(&self) -> bool {
        !self.json_path.is_empty()
    }

    pub fn output_alias(&self) -> String {
        if let Some(alias) = &self.alias {
            return alias.clone();
        }
        match &self.explicit_qualifier {
            Some(qualifier) => format!("{qualifier}_{}", self.api_name),
            None => self.api_name.to_string(),
        }
    }

    pub fn object_key(&self) -> &str {
        self.alias.as_deref().unwrap_or(self.api_name.as_ref())
    }
}
