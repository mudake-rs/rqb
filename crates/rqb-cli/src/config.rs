use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use syn::Type;

use crate::model::{FieldJson, FieldOps};

pub(crate) type TypeMappings = BTreeMap<(String, String), TypeMapping>;

#[derive(Debug, Clone, Default)]
pub(crate) struct GeneratorConfig {
    pub(crate) type_map: TypeMappings,
    pub(crate) raw_only: RawOnlyConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct TypeMapping {
    pub(crate) rust: String,
    pub(crate) ops: FieldOps,
    pub(crate) json: Option<FieldJson>,
    pub(crate) array: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RawOnlyConfig {
    pub(crate) allow: BTreeSet<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawConfig {
    type_map: BTreeMap<String, RawTypeMapping>,
    raw_only: RawOnlySection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTypeMapping {
    rust: String,
    #[serde(default)]
    ops: Option<FieldOps>,
    #[serde(default)]
    json: Option<FieldJson>,
    #[serde(default)]
    array: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawOnlySection {
    allow: Vec<String>,
}

impl GeneratorConfig {
    pub(crate) fn load(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };

        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let raw: RawConfig = toml::from_str(&text)
            .with_context(|| format!("failed to parse config {}", path.display()))?;
        raw.try_into()
    }
}

impl TryFrom<RawConfig> for GeneratorConfig {
    type Error = anyhow::Error;

    fn try_from(raw: RawConfig) -> Result<Self> {
        let mut type_map = BTreeMap::new();
        for (key, value) in raw.type_map {
            let parsed_key = parse_type_key(&key)?;
            validate_rust_type(&key, &value.rust)?;
            type_map.insert(
                parsed_key,
                TypeMapping {
                    rust: value.rust,
                    ops: value.ops.unwrap_or(FieldOps::None),
                    json: value.json,
                    array: value.array,
                },
            );
        }
        let mut raw_only_allow = BTreeSet::new();
        for key in raw.raw_only.allow {
            validate_raw_only_key(&key)?;
            raw_only_allow.insert(key);
        }

        Ok(Self {
            type_map,
            raw_only: RawOnlyConfig {
                allow: raw_only_allow,
            },
        })
    }
}

fn parse_type_key(key: &str) -> Result<(String, String)> {
    let parts = key.split('.').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        bail!("type_map key `{key}` must be schema-qualified as `schema.type`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

fn validate_raw_only_key(key: &str) -> Result<()> {
    let parts = key.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        bail!("raw_only.allow entry `{key}` must be `schema.relation.column`");
    }
    Ok(())
}

fn validate_rust_type(key: &str, rust: &str) -> Result<()> {
    let ty = syn::parse_str::<Type>(rust)
        .with_context(|| format!("type_map `{key}` has invalid Rust type `{rust}`"))?;
    let Type::Path(path) = &ty else {
        bail!("type_map `{key}` Rust type `{rust}` must be a qualified path");
    };
    if path.qself.is_some() {
        bail!("type_map `{key}` Rust type `{rust}` must be a qualified path");
    }
    if path.path.leading_colon.is_none() && path.path.segments.len() < 2 {
        bail!(
            "type_map `{key}` Rust type `{rust}` must be qualified, for example `crate::types::PgU256`"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GeneratorConfig, RawConfig};
    use crate::model::{FieldJson, FieldOps};

    #[test]
    fn parses_custom_type_map() {
        let config: GeneratorConfig = toml::from_str::<RawConfig>(
            r#"
            [type_map."bitcoin.uint256"]
            rust = "crate::types::PgU256"
            ops = "ordered"
            json = "text"
            array = true
            "#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        let mapping = config
            .type_map
            .get(&("bitcoin".to_owned(), "uint256".to_owned()))
            .unwrap();
        assert_eq!(mapping.rust, "crate::types::PgU256");
        assert_eq!(mapping.ops, FieldOps::Ordered);
        assert_eq!(mapping.json, Some(FieldJson::Text));
        assert!(mapping.array);
    }

    #[test]
    fn parses_raw_only_allowlist() {
        let config: GeneratorConfig = toml::from_str::<RawConfig>(
            r#"
            [raw_only]
            allow = ["sample.vector_documents.embedding"]
            "#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert!(config.type_map.is_empty());
        assert!(
            config
                .raw_only
                .allow
                .contains("sample.vector_documents.embedding")
        );
    }

    #[test]
    fn custom_type_map_defaults_to_no_json_exposure() {
        let config: GeneratorConfig = toml::from_str::<RawConfig>(
            r#"
            [type_map."public.vector"]
            rust = "pgvector::Vector"
            "#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        let mapping = config
            .type_map
            .get(&("public".to_owned(), "vector".to_owned()))
            .unwrap();
        assert_eq!(mapping.ops, FieldOps::None);
        assert_eq!(mapping.json, None);
        assert!(!mapping.array);
    }

    #[test]
    fn rejects_bare_rust_type_names() {
        let raw = toml::from_str::<RawConfig>(
            r#"
            [type_map."bitcoin.uint256"]
            rust = "PgU256"
            "#,
        )
        .unwrap();

        let err = GeneratorConfig::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("must be qualified"));
    }

    #[test]
    fn rejects_malformed_raw_only_allowlist_entries() {
        let raw = toml::from_str::<RawConfig>(
            r#"
            [raw_only]
            allow = ["documents.embedding"]
            "#,
        )
        .unwrap();

        let err = GeneratorConfig::try_from(raw).unwrap_err();
        assert!(err.to_string().contains("schema.relation.column"));
    }
}
