use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use heck::{ToShoutySnakeCase, ToSnakeCase};
use sqlx::{PgPool, Row};

use crate::ident::{sanitize_ident, unique_ident_strings};
use crate::model::{Column, Relation, RelationKind};
use crate::type_map::map_column_type;

pub(crate) async fn introspect(
    pool: &PgPool,
    schema: &str,
    only_tables: &[String],
) -> Result<Vec<Relation>> {
    let rows = sqlx::query(
        r#"
        SELECT table_name, table_type
        FROM information_schema.tables
        WHERE table_schema = $1
          AND table_type IN ('BASE TABLE', 'VIEW')
        ORDER BY table_name
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let only = only_tables.iter().cloned().collect::<BTreeSet<_>>();
    let mut relations = BTreeMap::<String, Relation>::new();
    for row in rows {
        let name: String = row.try_get("table_name")?;
        if !only.is_empty() && !only.contains(&name) {
            continue;
        }
        let table_type: String = row.try_get("table_type")?;
        let kind = if table_type == "VIEW" {
            RelationKind::View
        } else {
            RelationKind::Table
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
        SELECT table_name,
               column_name,
               data_type,
               udt_name
        FROM information_schema.columns
        WHERE table_schema = $1
        ORDER BY table_name, ordinal_position
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
        let data_type: String = row.try_get("data_type")?;
        let udt_name: String = row.try_get("udt_name")?;
        relation.columns.push(Column {
            api_name: name.clone(),
            rust_name: sanitize_ident(&name.to_snake_case()),
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            meta_name: sanitize_ident(&format!("{}_META", name.to_shouty_snake_case())),
            ty: map_column_type(&data_type, &udt_name),
            name,
        });
    }

    assign_unique_names(&mut relations);
    Ok(relations.into_values().collect())
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
        let meta_names = unique_ident_strings(
            relation
                .columns
                .iter()
                .map(|column| column.meta_name.clone()),
            &["FIELDS"],
        );
        let rust_names = unique_ident_strings(
            relation
                .columns
                .iter()
                .map(|column| column.rust_name.clone()),
            &["source", "table", "view"],
        );

        for (((column, const_name), meta_name), rust_name) in relation
            .columns
            .iter_mut()
            .zip(const_names)
            .zip(meta_names)
            .zip(rust_names)
        {
            column.const_name = const_name;
            column.meta_name = meta_name;
            column.rust_name = rust_name;
        }
    }
}
