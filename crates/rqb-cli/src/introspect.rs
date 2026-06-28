use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use heck::ToShoutySnakeCase;
use sqlx::{PgPool, Row};

use crate::ident::{sanitize_ident, unique_ident_strings};
use crate::model::{
    Column, ColumnType, GeneratedKind, PgEnum, Relation, RelationKind, SchemaModel,
};
use crate::type_map::map_column_type;

pub(crate) async fn introspect(
    pool: &PgPool,
    schema: &str,
    only_tables: &[String],
) -> Result<SchemaModel> {
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

    let column_rows = sqlx::query(
        r#"
        SELECT c.relname AS table_name,
               a.attname AS column_name,
               tn.nspname AS type_schema,
               t.typname AS udt_name,
               pg_catalog.format_type(a.atttypid, a.atttypmod) AS pg_type,
               t.typtype = 'e' AS is_pg_enum,
               elem_t.typtype = 'e' AS is_pg_enum_array,
               NOT a.attnotnull AS nullable,
               a.attgenerated::text AS generated,
               a.attidentity::text AS identity_generation
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid
        JOIN pg_catalog.pg_type t ON t.oid = a.atttypid
        JOIN pg_catalog.pg_namespace tn ON tn.oid = t.typnamespace
        LEFT JOIN pg_catalog.pg_type elem_t ON elem_t.oid = t.typelem
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
        let pg_type: String = row.try_get("pg_type")?;
        let is_pg_enum: bool = row.try_get("is_pg_enum")?;
        let is_pg_enum_array: Option<bool> = row.try_get("is_pg_enum_array")?;
        let nullable: bool = row.try_get("nullable")?;
        let generated: String = row.try_get("generated")?;
        let identity_generation: String = row.try_get("identity_generation")?;
        let ty = map_introspected_column_type(
            &type_schema,
            &udt_name,
            &pg_type,
            is_pg_enum,
            is_pg_enum_array.unwrap_or(false),
            &enum_keys,
            &mut used_enums,
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
    assign_unique_names(&mut relations);
    Ok(SchemaModel {
        enums,
        relations: relations.into_values().collect(),
    })
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

fn map_introspected_column_type(
    type_schema: &str,
    udt_name: &str,
    pg_type: &str,
    is_pg_enum: bool,
    is_pg_enum_array: bool,
    enum_keys: &BTreeSet<(String, String)>,
    used_enums: &mut BTreeSet<(String, String)>,
) -> ColumnType {
    let enum_key = (type_schema.to_owned(), udt_name.to_owned());
    if enum_keys.contains(&enum_key) {
        used_enums.insert(enum_key.clone());
        return ColumnType::PgEnum {
            schema: enum_key.0,
            name: enum_key.1,
            pg: pg_type.to_owned(),
            array: false,
        };
    }

    if let Some(element_udt) = udt_name.strip_prefix('_') {
        let enum_key = (type_schema.to_owned(), element_udt.to_owned());
        if enum_keys.contains(&enum_key) {
            used_enums.insert(enum_key.clone());
            return ColumnType::PgEnum {
                schema: enum_key.0,
                name: enum_key.1,
                pg: pg_type.to_owned(),
                array: true,
            };
        }
    }

    if is_pg_enum || is_pg_enum_array {
        return ColumnType::RawOnly {
            pg: pg_type.to_owned(),
        };
    }

    let mut ty = map_column_type(udt_name);
    if let ColumnType::RawOnly { pg } = &mut ty {
        *pg = pg_type.to_owned();
    }
    ty
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
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::model::{Column, ColumnType, GeneratedKind, KnownType, Relation, RelationKind};

    use super::{assign_unique_names, generated_kind, map_introspected_column_type};

    fn column(name: &str, const_name: &str) -> Column {
        Column {
            name: name.to_owned(),
            const_name: const_name.to_owned(),
            ty: ColumnType::Known(KnownType::Text),
            nullable: false,
            generated: GeneratedKind::None,
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
    fn map_introspected_column_type_prefers_pg_enum_metadata() {
        let enum_keys = BTreeSet::from([("sample".to_owned(), "invoice_state".to_owned())]);
        let mut used_enums = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                "sample",
                "invoice_state",
                "sample.invoice_state",
                true,
                false,
                &enum_keys,
                &mut used_enums,
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
                "sample",
                "_invoice_state",
                "sample.invoice_state[]",
                false,
                true,
                &enum_keys,
                &mut used_enums,
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
    }

    #[test]
    fn cross_schema_pg_enums_remain_raw_only_before_known_type_fallback() {
        let enum_keys = BTreeSet::from([("sample".to_owned(), "invoice_state".to_owned())]);
        let mut used_enums = BTreeSet::new();

        assert_eq!(
            map_introspected_column_type(
                "other",
                "uuid",
                "other.uuid",
                true,
                false,
                &enum_keys,
                &mut used_enums,
            ),
            ColumnType::RawOnly {
                pg: "other.uuid".to_owned()
            }
        );
        assert_eq!(
            map_introspected_column_type(
                "other",
                "_text",
                "other.text[]",
                false,
                true,
                &enum_keys,
                &mut used_enums,
            ),
            ColumnType::RawOnly {
                pg: "other.text[]".to_owned()
            }
        );
        assert!(used_enums.is_empty());
    }
}
