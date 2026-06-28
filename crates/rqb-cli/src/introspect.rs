use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use heck::ToShoutySnakeCase;
use sqlx::{PgPool, Row};

use crate::config::{TypeMapping, TypeMappings};
use crate::ident::{sanitize_ident, unique_ident_strings};
use crate::model::{
    Column, ColumnType, GeneratedKind, PgEnum, Relation, RelationKind, SchemaModel,
    UniqueConstraint,
};
use crate::type_map::map_column_type;

pub(crate) async fn introspect(
    pool: &PgPool,
    schema: &str,
    only_tables: &[String],
    type_mappings: &TypeMappings,
) -> Result<Introspection> {
    let rows = sqlx::query(
        r#"
        SELECT c.relname AS relation_name,
               c.relkind::text AS relation_kind
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1
          AND c.relkind IN ('r', 'p', 'v', 'm')
        ORDER BY c.relname
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let only = only_tables.iter().cloned().collect::<BTreeSet<_>>();
    let mut relations = BTreeMap::<String, Relation>::new();
    for row in rows {
        let name: String = row.try_get("relation_name")?;
        if !only.is_empty() && !only.contains(&name) {
            continue;
        }
        let relation_kind: String = row.try_get("relation_kind")?;
        let kind = match relation_kind.as_str() {
            "v" => RelationKind::View,
            "m" => RelationKind::MaterializedView,
            _ => RelationKind::Table,
        };
        relations.insert(
            name.clone(),
            Relation {
                schema: schema.to_owned(),
                name,
                kind,
                columns: Vec::new(),
                constraints: Vec::new(),
            },
        );
    }

    if !only.is_empty() {
        let found = relations.keys().cloned().collect::<BTreeSet<_>>();
        let missing = only.difference(&found).cloned().collect::<Vec<_>>();
        if !missing.is_empty() {
            bail!(
                "requested relation(s) not found in schema `{schema}`: {}",
                missing.join(", ")
            );
        }
    }

    let mut enums = introspect_enums(pool, schema).await?;
    let enum_keys = enums
        .iter()
        .map(|enum_type| (enum_type.schema.clone(), enum_type.name.clone()))
        .collect::<BTreeSet<_>>();
    let mut used_enums = BTreeSet::<(String, String)>::new();
    let mut used_type_mappings = BTreeSet::<(String, String)>::new();

    let column_rows = sqlx::query(
        r#"
        SELECT c.relname AS table_name,
               a.attname AS column_name,
               tn.nspname AS type_schema,
               t.typname AS udt_name,
               elem_n.nspname AS element_type_schema,
               elem_t.typname AS element_udt_name,
               pg_catalog.format_type(a.atttypid, a.atttypmod) AS pg_type,
               t.typtype = 'e' AS is_pg_enum,
               elem_t.typtype = 'e' AS is_pg_enum_array,
               t.typnamespace = 'pg_catalog'::regnamespace AS is_pg_catalog_type,
               NOT a.attnotnull AS nullable,
               a.attgenerated::text AS generated,
               a.attidentity::text AS identity_generation
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid
        JOIN pg_catalog.pg_type t ON t.oid = a.atttypid
        JOIN pg_catalog.pg_namespace tn ON tn.oid = t.typnamespace
        LEFT JOIN pg_catalog.pg_type elem_t ON elem_t.oid = t.typelem
        LEFT JOIN pg_catalog.pg_namespace elem_n ON elem_n.oid = elem_t.typnamespace
        WHERE n.nspname = $1
          AND c.relkind IN ('r', 'p', 'v', 'm')
          AND a.attnum > 0
          AND NOT a.attisdropped
        ORDER BY c.relname, a.attnum
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    for row in column_rows {
        let table_name: String = row.try_get("table_name")?;
        let Some(relation) = relations.get_mut(&table_name) else {
            continue;
        };
        let name: String = row.try_get("column_name")?;
        let type_schema: String = row.try_get("type_schema")?;
        let udt_name: String = row.try_get("udt_name")?;
        let element_type_schema: Option<String> = row.try_get("element_type_schema")?;
        let element_udt_name: Option<String> = row.try_get("element_udt_name")?;
        let pg_type: String = row.try_get("pg_type")?;
        let is_pg_enum: bool = row.try_get("is_pg_enum")?;
        let is_pg_enum_array: Option<bool> = row.try_get("is_pg_enum_array")?;
        let is_pg_catalog_type: bool = row.try_get("is_pg_catalog_type")?;
        let nullable: bool = row.try_get("nullable")?;
        let generated: String = row.try_get("generated")?;
        let identity_generation: String = row.try_get("identity_generation")?;
        let ty = map_introspected_column_type(
            IntrospectedType {
                schema: &type_schema,
                udt_name: &udt_name,
                element_schema: element_type_schema.as_deref(),
                element_udt_name: element_udt_name.as_deref(),
                pg: &pg_type,
                is_pg_enum,
                is_pg_enum_array: is_pg_enum_array.unwrap_or(false),
                is_pg_catalog_type,
            },
            &enum_keys,
            &mut used_enums,
            &mut used_type_mappings,
            type_mappings,
        );
        relation.columns.push(Column {
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            ty,
            nullable,
            generated: generated_kind(&generated, &identity_generation),
            name,
        });
    }

    enums.retain(|enum_type| {
        used_enums.contains(&(enum_type.schema.clone(), enum_type.name.clone()))
    });
    introspect_unique_constraints(pool, schema, &mut relations).await?;
    assign_unique_names(&mut relations);
    Ok(Introspection {
        schema: SchemaModel {
            enums,
            relations: relations.into_values().collect(),
        },
        used_type_mappings,
    })
}

pub(crate) struct Introspection {
    pub(crate) schema: SchemaModel,
    pub(crate) used_type_mappings: BTreeSet<(String, String)>,
}

async fn introspect_enums(pool: &PgPool, schema: &str) -> Result<Vec<PgEnum>> {
    let enum_rows = sqlx::query(
        r#"
        SELECT n.nspname AS type_schema,
               t.typname AS type_name,
               e.enumlabel AS enum_label
        FROM pg_catalog.pg_type t
        JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
        JOIN pg_catalog.pg_enum e ON e.enumtypid = t.oid
        WHERE n.nspname = $1
        ORDER BY t.typname, e.enumsortorder
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let mut enums = BTreeMap::<(String, String), PgEnum>::new();
    for row in enum_rows {
        let schema: String = row.try_get("type_schema")?;
        let name: String = row.try_get("type_name")?;
        let label: String = row.try_get("enum_label")?;
        enums
            .entry((schema.clone(), name.clone()))
            .or_insert_with(|| PgEnum {
                schema,
                name,
                variants: Vec::new(),
            })
            .variants
            .push(label);
    }

    Ok(enums.into_values().collect())
}

async fn introspect_unique_constraints(
    pool: &PgPool,
    schema: &str,
    relations: &mut BTreeMap<String, Relation>,
) -> Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT c.relname AS relation_name,
               con.conname AS constraint_name
        FROM pg_catalog.pg_constraint con
        JOIN pg_catalog.pg_class c ON c.oid = con.conrelid
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1
          AND c.relkind IN ('r', 'p')
          AND con.contype IN ('p', 'u')
          AND NOT con.condeferrable
        ORDER BY c.relname, con.conname
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let relation_name: String = row.try_get("relation_name")?;
        let Some(relation) = relations.get_mut(&relation_name) else {
            continue;
        };
        let name: String = row.try_get("constraint_name")?;
        relation.constraints.push(UniqueConstraint {
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            name,
        });
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct IntrospectedType<'a> {
    schema: &'a str,
    udt_name: &'a str,
    element_schema: Option<&'a str>,
    element_udt_name: Option<&'a str>,
    pg: &'a str,
    is_pg_enum: bool,
    is_pg_enum_array: bool,
    is_pg_catalog_type: bool,
}

fn map_introspected_column_type(
    ty: IntrospectedType<'_>,
    enum_keys: &BTreeSet<(String, String)>,
    used_enums: &mut BTreeSet<(String, String)>,
    used_type_mappings: &mut BTreeSet<(String, String)>,
    type_mappings: &TypeMappings,
) -> ColumnType {
    let type_key = (ty.schema.to_owned(), ty.udt_name.to_owned());
    if let Some(mapping) = type_mappings.get(&type_key) {
        used_type_mappings.insert(type_key);
        return custom_column_type(ty.pg, mapping, false);
    }

    if let (Some(element_schema), Some(element_udt)) = (ty.element_schema, ty.element_udt_name) {
        let element_key = (element_schema.to_owned(), element_udt.to_owned());
        if let Some(mapping) = type_mappings.get(&element_key) {
            if mapping.array {
                used_type_mappings.insert(element_key);
                return custom_column_type(ty.pg, mapping, true);
            }
            return ColumnType::RawOnly {
                pg: ty.pg.to_owned(),
            };
        }
    }

    let enum_key = (ty.schema.to_owned(), ty.udt_name.to_owned());
    if enum_keys.contains(&enum_key) {
        used_enums.insert(enum_key.clone());
        return ColumnType::PgEnum {
            schema: enum_key.0,
            name: enum_key.1,
            pg: ty.pg.to_owned(),
            array: false,
        };
    }

    if let Some(element_udt) = ty.udt_name.strip_prefix('_') {
        let enum_key = (ty.schema.to_owned(), element_udt.to_owned());
        if enum_keys.contains(&enum_key) {
            used_enums.insert(enum_key.clone());
            return ColumnType::PgEnum {
                schema: enum_key.0,
                name: enum_key.1,
                pg: ty.pg.to_owned(),
                array: true,
            };
        }
    }

    if ty.is_pg_enum || ty.is_pg_enum_array {
        return ColumnType::RawOnly {
            pg: ty.pg.to_owned(),
        };
    }

    if !ty.is_pg_catalog_type {
        return ColumnType::RawOnly {
            pg: ty.pg.to_owned(),
        };
    }

    let mut column_type = map_column_type(ty.udt_name);
    if let ColumnType::RawOnly { pg } = &mut column_type {
        *pg = ty.pg.to_owned();
    }
    column_type
}

fn custom_column_type(pg_type: &str, mapping: &TypeMapping, array: bool) -> ColumnType {
    ColumnType::Custom {
        pg: pg_type.to_owned(),
        rust: mapping.rust.clone(),
        array,
        ops: mapping.ops,
        json: (!array).then_some(mapping.json).flatten(),
    }
}

fn generated_kind(generated: &str, identity_generation: &str) -> GeneratedKind {
    match (generated, identity_generation) {
        ("s", _) => GeneratedKind::Stored,
        (_, "a") => GeneratedKind::IdentityAlways,
        (_, "d") => GeneratedKind::IdentityByDefault,
        _ => GeneratedKind::None,
    }
}

fn assign_unique_names(relations: &mut BTreeMap<String, Relation>) {
    for relation in relations.values_mut() {
        let const_names = unique_ident_strings(
            relation
                .columns
                .iter()
                .map(|column| column.const_name.clone()),
            &["FIELDS"],
        );

        for (column, const_name) in relation.columns.iter_mut().zip(const_names) {
            column.const_name = const_name;
        }

        let constraint_names = unique_ident_strings(
            relation
                .constraints
                .iter()
                .map(|constraint| constraint.const_name.clone()),
            &[],
        );
        for (constraint, const_name) in relation.constraints.iter_mut().zip(constraint_names) {
            constraint.const_name = const_name;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::config::TypeMapping;
    use crate::model::{
        Column, ColumnType, FieldJson, FieldOps, GeneratedKind, KnownType, Relation, RelationKind,
        UniqueConstraint,
    };

    use super::{
        IntrospectedType, assign_unique_names, generated_kind, map_introspected_column_type,
    };

    fn column(name: &str, const_name: &str) -> Column {
        Column {
            name: name.to_owned(),
            const_name: const_name.to_owned(),
            ty: ColumnType::Known(KnownType::Text),
            nullable: false,
            generated: GeneratedKind::None,
        }
    }

    fn introspected_type<'a>(
        schema: &'a str,
        udt_name: &'a str,
        pg: &'a str,
    ) -> IntrospectedType<'a> {
        IntrospectedType {
            schema,
            udt_name,
            element_schema: None,
            element_udt_name: None,
            pg,
            is_pg_enum: false,
            is_pg_enum_array: false,
            is_pg_catalog_type: schema == "pg_catalog",
        }
    }

    fn introspected_array<'a>(
        schema: &'a str,
        udt_name: &'a str,
        element_schema: &'a str,
        element_udt_name: &'a str,
        pg: &'a str,
    ) -> IntrospectedType<'a> {
        IntrospectedType {
            schema,
            udt_name,
            element_schema: Some(element_schema),
            element_udt_name: Some(element_udt_name),
            pg,
            is_pg_enum: false,
            is_pg_enum_array: false,
            is_pg_catalog_type: schema == "pg_catalog",
        }
    }

    #[test]
    fn assign_unique_names_disambiguates_columns_after_sanitizing() {
        let mut relations = BTreeMap::from([(
            "events".to_owned(),
            Relation {
                schema: "public".to_owned(),
                name: "events".to_owned(),
                kind: RelationKind::Table,
                columns: vec![column("source", "SOURCE"), column("source_", "SOURCE")],
                constraints: Vec::new(),
            },
        )]);

        assign_unique_names(&mut relations);

        let relation = relations.get("events").unwrap();
        assert_eq!(relation.columns[0].const_name, "SOURCE");
        assert_eq!(relation.columns[1].const_name, "SOURCE_1");
    }

    #[test]
    fn assign_unique_names_reserves_fields_array_name() {
        let mut relations = BTreeMap::from([(
            "events".to_owned(),
            Relation {
                schema: "public".to_owned(),
                name: "events".to_owned(),
                kind: RelationKind::Table,
                columns: vec![column("fields", "FIELDS")],
                constraints: Vec::new(),
            },
        )]);

        assign_unique_names(&mut relations);

        let relation = relations.get("events").unwrap();
        assert_eq!(relation.columns[0].const_name, "FIELDS_1");
    }

    #[test]
    fn generated_kind_maps_pg_catalog_markers() {
        assert_eq!(generated_kind("s", ""), GeneratedKind::Stored);
        assert_eq!(generated_kind("", "a"), GeneratedKind::IdentityAlways);
        assert_eq!(generated_kind("", "d"), GeneratedKind::IdentityByDefault);
        assert_eq!(generated_kind("", ""), GeneratedKind::None);
    }

    #[test]
    fn assign_unique_names_disambiguates_constraints_after_sanitizing() {
        let mut relations = BTreeMap::from([(
            "events".to_owned(),
            Relation {
                schema: "public".to_owned(),
                name: "events".to_owned(),
                kind: RelationKind::Table,
                columns: vec![column("id", "ID")],
                constraints: vec![
                    UniqueConstraint {
                        name: "events-key".to_owned(),
                        const_name: "EVENTS_KEY".to_owned(),
                    },
                    UniqueConstraint {
                        name: "events key".to_owned(),
                        const_name: "EVENTS_KEY".to_owned(),
                    },
                ],
            },
        )]);

        assign_unique_names(&mut relations);

        let relation = relations.get("events").unwrap();
        assert_eq!(relation.constraints[0].const_name, "EVENTS_KEY");
        assert_eq!(relation.constraints[1].const_name, "EVENTS_KEY_1");
    }

    #[test]
    fn map_introspected_column_type_prefers_pg_enum_metadata() {
        let enum_keys = BTreeSet::from([("sample".to_owned(), "invoice_state".to_owned())]);
        let mut used_enums = BTreeSet::new();
        let mut used_type_mappings = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                IntrospectedType {
                    is_pg_enum: true,
                    ..introspected_type("sample", "invoice_state", "sample.invoice_state")
                },
                &enum_keys,
                &mut used_enums,
                &mut used_type_mappings,
                &BTreeMap::new(),
            ),
            ColumnType::PgEnum {
                schema: "sample".to_owned(),
                name: "invoice_state".to_owned(),
                pg: "sample.invoice_state".to_owned(),
                array: false,
            }
        );
        assert_eq!(
            map_introspected_column_type(
                IntrospectedType {
                    is_pg_enum_array: true,
                    ..introspected_array(
                        "sample",
                        "_invoice_state",
                        "sample",
                        "invoice_state",
                        "sample.invoice_state[]",
                    )
                },
                &enum_keys,
                &mut used_enums,
                &mut used_type_mappings,
                &BTreeMap::new(),
            ),
            ColumnType::PgEnum {
                schema: "sample".to_owned(),
                name: "invoice_state".to_owned(),
                pg: "sample.invoice_state[]".to_owned(),
                array: true,
            }
        );
        assert_eq!(
            used_enums,
            BTreeSet::from([("sample".to_owned(), "invoice_state".to_owned())])
        );
        assert!(used_type_mappings.is_empty());
    }

    #[test]
    fn cross_schema_pg_enums_remain_raw_only_before_known_type_fallback() {
        let enum_keys = BTreeSet::from([("sample".to_owned(), "invoice_state".to_owned())]);
        let mut used_enums = BTreeSet::new();
        let mut used_type_mappings = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                IntrospectedType {
                    is_pg_enum: true,
                    ..introspected_type("other", "uuid", "other.uuid")
                },
                &enum_keys,
                &mut used_enums,
                &mut used_type_mappings,
                &BTreeMap::new(),
            ),
            ColumnType::RawOnly {
                pg: "other.uuid".to_owned()
            }
        );
        assert_eq!(
            map_introspected_column_type(
                IntrospectedType {
                    is_pg_enum_array: true,
                    ..introspected_array("other", "_text", "other", "text", "other.text[]")
                },
                &enum_keys,
                &mut used_enums,
                &mut used_type_mappings,
                &BTreeMap::new(),
            ),
            ColumnType::RawOnly {
                pg: "other.text[]".to_owned()
            }
        );
        assert!(used_enums.is_empty());
        assert!(used_type_mappings.is_empty());
    }

    #[test]
    fn custom_type_mapping_wins_before_builtin_type_mapping() {
        let type_mappings = BTreeMap::from([(
            ("pg_catalog".to_owned(), "numeric".to_owned()),
            TypeMapping {
                rust: "rust_decimal::Decimal".to_owned(),
                ops: FieldOps::Ordered,
                json: Some(FieldJson::NumericString),
                array: true,
            },
        )]);
        let mut used_enums = BTreeSet::new();
        let mut used_type_mappings = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                introspected_type("pg_catalog", "numeric", "numeric"),
                &BTreeSet::new(),
                &mut used_enums,
                &mut used_type_mappings,
                &type_mappings,
            ),
            ColumnType::Custom {
                pg: "numeric".to_owned(),
                rust: "rust_decimal::Decimal".to_owned(),
                array: false,
                ops: FieldOps::Ordered,
                json: Some(FieldJson::NumericString),
            }
        );
        assert_eq!(
            used_type_mappings,
            BTreeSet::from([("pg_catalog".to_owned(), "numeric".to_owned())])
        );
    }

    #[test]
    fn custom_type_arrays_require_explicit_array_mapping() {
        let type_mappings = BTreeMap::from([(
            ("bitcoin".to_owned(), "uint256".to_owned()),
            TypeMapping {
                rust: "crate::types::PgU256".to_owned(),
                ops: FieldOps::Ordered,
                json: Some(FieldJson::Text),
                array: false,
            },
        )]);
        let mut used_enums = BTreeSet::new();
        let mut used_type_mappings = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                introspected_array(
                    "bitcoin",
                    "_uint256",
                    "bitcoin",
                    "uint256",
                    "bitcoin.uint256[]",
                ),
                &BTreeSet::new(),
                &mut used_enums,
                &mut used_type_mappings,
                &type_mappings,
            ),
            ColumnType::RawOnly {
                pg: "bitcoin.uint256[]".to_owned(),
            }
        );
        assert!(used_type_mappings.is_empty());

        let mut type_mappings = type_mappings;
        type_mappings
            .get_mut(&("bitcoin".to_owned(), "uint256".to_owned()))
            .unwrap()
            .array = true;

        assert_eq!(
            map_introspected_column_type(
                introspected_array(
                    "bitcoin",
                    "_uint256",
                    "bitcoin",
                    "uint256",
                    "bitcoin.uint256[]",
                ),
                &BTreeSet::new(),
                &mut used_enums,
                &mut used_type_mappings,
                &type_mappings,
            ),
            ColumnType::Custom {
                pg: "bitcoin.uint256[]".to_owned(),
                rust: "crate::types::PgU256".to_owned(),
                array: true,
                ops: FieldOps::Ordered,
                json: None,
            }
        );
        assert_eq!(
            used_type_mappings,
            BTreeSet::from([("bitcoin".to_owned(), "uint256".to_owned())])
        );
    }

    #[test]
    fn custom_type_arrays_do_not_inherit_scalar_json_exposure() {
        let type_mappings = BTreeMap::from([(
            ("bitcoin".to_owned(), "uint256".to_owned()),
            TypeMapping {
                rust: "crate::types::PgU256".to_owned(),
                ops: FieldOps::Ordered,
                json: Some(FieldJson::Text),
                array: true,
            },
        )]);
        let mut used_enums = BTreeSet::new();
        let mut used_type_mappings = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                introspected_array(
                    "bitcoin",
                    "_uint256",
                    "bitcoin",
                    "uint256",
                    "bitcoin.uint256[]",
                ),
                &BTreeSet::new(),
                &mut used_enums,
                &mut used_type_mappings,
                &type_mappings,
            ),
            ColumnType::Custom {
                pg: "bitcoin.uint256[]".to_owned(),
                rust: "crate::types::PgU256".to_owned(),
                array: true,
                ops: FieldOps::Ordered,
                json: None,
            }
        );
        assert_eq!(
            used_type_mappings,
            BTreeSet::from([("bitcoin".to_owned(), "uint256".to_owned())])
        );
    }

    #[test]
    fn unmapped_non_catalog_types_remain_raw_only_before_builtin_fallback() {
        let mut used_enums = BTreeSet::new();
        let mut used_type_mappings = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                introspected_type("bitcoin", "numeric", "bitcoin.numeric"),
                &BTreeSet::new(),
                &mut used_enums,
                &mut used_type_mappings,
                &BTreeMap::new(),
            ),
            ColumnType::RawOnly {
                pg: "bitcoin.numeric".to_owned(),
            }
        );
        assert_eq!(
            map_introspected_column_type(
                introspected_array("bitcoin", "_uuid", "bitcoin", "uuid", "bitcoin.uuid[]"),
                &BTreeSet::new(),
                &mut used_enums,
                &mut used_type_mappings,
                &BTreeMap::new(),
            ),
            ColumnType::RawOnly {
                pg: "bitcoin.uuid[]".to_owned(),
            }
        );
        assert!(used_type_mappings.is_empty());
    }
}
