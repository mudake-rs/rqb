use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};

use crate::ident::sanitize_ident;
use crate::model::{Column, PgDomain, PgEnum, Relation, RelationKind};
use crate::type_map::{map_field_type, type_family_for_udt, value_repr_for_family};

pub(crate) async fn introspect(
    client: &tokio_postgres::Client,
    schema: &str,
    only_tables: &[String],
    enums: &BTreeMap<String, PgEnum>,
    domains: &BTreeMap<String, PgDomain>,
) -> Result<Vec<Relation>> {
    let rows = client
        .query(
            r#"
            SELECT table_name, table_type
            FROM information_schema.tables
            WHERE table_schema = $1
              AND table_type IN ('BASE TABLE', 'VIEW')
            ORDER BY table_name
            "#,
            &[&schema],
        )
        .await?;

    let only = only_tables.iter().cloned().collect::<BTreeSet<_>>();
    let mut relations = BTreeMap::<String, Relation>::new();
    for row in rows {
        let name: String = row.get("table_name");
        if !only.is_empty() && !only.contains(&name) {
            continue;
        }
        let table_type: String = row.get("table_type");
        let kind = if table_type == "VIEW" {
            RelationKind::View
        } else {
            RelationKind::Table
        };
        relations.insert(
            name.clone(),
            Relation {
                name,
                kind,
                columns: Vec::new(),
            },
        );
    }

    let column_rows = client
        .query(
            r#"
            SELECT table_name, column_name, data_type, udt_name, domain_schema, domain_name
            FROM information_schema.columns
            WHERE table_schema = $1
            ORDER BY table_name, ordinal_position
            "#,
            &[&schema],
        )
        .await?;

    for row in column_rows {
        let table_name: String = row.get("table_name");
        let Some(relation) = relations.get_mut(&table_name) else {
            continue;
        };
        let name: String = row.get("column_name");
        let data_type: String = row.get("data_type");
        let udt_name: String = row.get("udt_name");
        let domain_schema: Option<String> = row.get("domain_schema");
        let domain_name: Option<String> = row.get("domain_name");
        relation.columns.push(Column {
            api_name: name.to_lower_camel_case(),
            rust_name: sanitize_ident(&name.to_snake_case()),
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            field_type: map_field_type(
                &data_type,
                &udt_name,
                domain_schema.as_deref(),
                domain_name.as_deref(),
                enums,
                domains,
            ),
            name,
        });
    }

    Ok(relations.into_values().collect())
}

pub(crate) async fn introspect_enums(
    client: &tokio_postgres::Client,
    schema: &str,
) -> Result<BTreeMap<String, PgEnum>> {
    let rows = client
        .query(
            r#"
            SELECT t.typname, e.enumlabel
            FROM pg_type t
            JOIN pg_enum e ON e.enumtypid = t.oid
            JOIN pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname = $1
            ORDER BY t.typname, e.enumsortorder
            "#,
            &[&schema],
        )
        .await?;

    let mut enums = BTreeMap::<String, PgEnum>::new();
    for row in rows {
        let name: String = row.get("typname");
        let variant: String = row.get("enumlabel");
        enums
            .entry(name.clone())
            .or_insert_with(|| PgEnum {
                schema: schema.to_owned(),
                const_name: sanitize_ident(&name.to_shouty_snake_case()),
                rust_name: sanitize_ident(&name.to_upper_camel_case()),
                name,
                variants: Vec::new(),
            })
            .variants
            .push(variant);
    }
    Ok(enums)
}

pub(crate) async fn introspect_domains(
    client: &tokio_postgres::Client,
    schema: &str,
) -> Result<BTreeMap<String, PgDomain>> {
    let rows = client
        .query(
            r#"
            SELECT t.typname, bt.typname AS base_udt_name
            FROM pg_type t
            JOIN pg_namespace n ON n.oid = t.typnamespace
            JOIN pg_type bt ON bt.oid = t.typbasetype
            WHERE t.typtype = 'd'
              AND n.nspname = $1
            ORDER BY t.typname
            "#,
            &[&schema],
        )
        .await?;

    let mut domains = BTreeMap::<String, PgDomain>::new();
    for row in rows {
        let name: String = row.get("typname");
        let base_udt_name: String = row.get("base_udt_name");
        let family = type_family_for_udt(&base_udt_name);
        domains.insert(
            name.clone(),
            PgDomain {
                schema: schema.to_owned(),
                const_name: sanitize_ident(&name.to_shouty_snake_case()),
                family,
                value_repr: value_repr_for_family(family),
                select_repr: rqb_core::SelectRepr::Text,
                name,
            },
        );
    }
    Ok(domains)
}
