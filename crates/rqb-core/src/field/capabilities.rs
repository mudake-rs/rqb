use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonPathPolicy {
    Deny,
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub selectable: bool,
    pub sortable: bool,
    pub filterable: bool,
    pub json_path: JsonPathPolicy,
    pub text_search: TextSearchConfig,
}

impl Capabilities {
    pub const fn all() -> Self {
        Self {
            selectable: true,
            sortable: true,
            filterable: true,
            json_path: JsonPathPolicy::Deny,
            text_search: TextSearchConfig::None,
        }
    }

    pub const fn hidden() -> Self {
        Self {
            selectable: false,
            sortable: false,
            filterable: false,
            json_path: JsonPathPolicy::Deny,
            text_search: TextSearchConfig::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSearchConfig {
    None,
    Config(&'static str),
}
