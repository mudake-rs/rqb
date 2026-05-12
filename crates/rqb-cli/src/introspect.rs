use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use heck::ToShoutySnakeCase;
use sqlx::{PgPool, Row};

use crate::ident::{sanitize_ident, unique_ident_strings};
use crate::model::{Column, GeneratedKind, Relation, RelationKind};
use crate::type_map::map_column_type;

pub(crate) async fn introspect(
    pool: &PgPool,
    schema: &str,
    only_tables: &[String],
) -> Result<Vec<Relation>> {
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

    let column_rows = sqlx::query(
        r#"
        SELECT c.relname AS table_name,
               a.attname AS column_name,
               t.typname AS udt_name,
               pg_catalog.format_type(a.atttypid, a.atttypmod) AS pg_type,
               NOT a.attnotnull AS nullable,
               a.attgenerated::text AS generated,
               a.attidentity::text AS identity_generation
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid
        JOIN pg_catalog.pg_type t ON t.oid = a.atttypid
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
        let udt_name: String = row.try_get("udt_name")?;
        let pg_type: String = row.try_get("pg_type")?;
        let nullable: bool = row.try_get("nullable")?;
        let generated: String = row.try_get("generated")?;
        let identity_generation: String = row.try_get("identity_generation")?;
        let mut ty = map_column_type(&udt_name);
        if let crate::model::ColumnType::RawOnly { pg } = &mut ty {
            *pg = pg_type;
        }
        relation.columns.push(Column {
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            ty,
            nullable,
            generated: generated_kind(&generated, &identity_generation),
            name,
        });
    }

    assign_unique_names(&mut relations);
    Ok(relations.into_values().collect())
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
    use std::collections::BTreeMap;

    use crate::model::{Column, ColumnType, GeneratedKind, KnownType, Relation, RelationKind};

    use super::{assign_unique_names, generated_kind};

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
}
