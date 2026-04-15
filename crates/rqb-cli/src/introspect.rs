use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};

use crate::ident::{sanitize_ident, unique_ident_strings};
use crate::model::{
    Column, ColumnType, PgDomain, PgDomainSource, PgEnum, Relation, RelationKind, SchemaTypeKey,
};
use crate::type_map::map_field_type;

pub(crate) async fn introspect(
    client: &tokio_postgres::Client,
    schema: &str,
    only_tables: &[String],
    enums: &BTreeMap<SchemaTypeKey, PgEnum>,
    domains: &BTreeMap<SchemaTypeKey, PgDomainSource>,
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

    let column_rows = client
        .query(
            r#"
            SELECT table_name,
                   column_name,
                   data_type,
                   udt_schema,
                   udt_name,
                   domain_schema,
                   domain_name
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
        let udt_schema: String = row.get("udt_schema");
        let udt_name: String = row.get("udt_name");
        let domain_schema: Option<String> = row.get("domain_schema");
        let domain_name: Option<String> = row.get("domain_name");
        relation.columns.push(Column {
            api_name: name.to_lower_camel_case(),
            rust_name: sanitize_ident(&name.to_snake_case()),
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            field_type: map_field_type(
                &data_type,
                &udt_schema,
                &udt_name,
                domain_schema.as_deref(),
                domain_name.as_deref(),
                enums,
                domains,
            )
            .with_context(|| {
                format!(
                    "failed to map type for column `{schema}`.`{table_name}`.`{name}` \
                     (data_type `{data_type}`, udt_schema `{udt_schema}`, udt_name `{udt_name}`)"
                )
            })?,
            name,
        });
    }

    Ok(relations.into_values().collect())
}

pub(crate) async fn introspect_enums(
    client: &tokio_postgres::Client,
) -> Result<BTreeMap<SchemaTypeKey, PgEnum>> {
    let rows = client
        .query(
            r#"
            SELECT n.nspname, t.typname, e.enumlabel
            FROM pg_type t
            JOIN pg_enum e ON e.enumtypid = t.oid
            JOIN pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
              AND n.nspname NOT LIKE 'pg_toast%'
            ORDER BY n.nspname, t.typname, e.enumsortorder
            "#,
            &[],
        )
        .await?;

    let mut enums = BTreeMap::<SchemaTypeKey, PgEnum>::new();
    for row in rows {
        let schema: String = row.get("nspname");
        let name: String = row.get("typname");
        let variant: String = row.get("enumlabel");
        let key = (schema.clone(), name.clone());
        enums
            .entry(key)
            .or_insert_with(|| PgEnum {
                schema,
                const_name: sanitize_ident(&name.to_shouty_snake_case()),
                rust_name: sanitize_ident(&name.to_upper_camel_case()),
                name,
                variants: Vec::new(),
            })
            .variants
            .push(variant);
    }
    assign_enum_names(&mut enums);
    Ok(enums)
}

pub(crate) async fn introspect_domains(
    client: &tokio_postgres::Client,
) -> Result<BTreeMap<SchemaTypeKey, PgDomainSource>> {
    let rows = client
        .query(
            r#"
            SELECT n.nspname, t.typname, bt.typname AS base_udt_name
            FROM pg_type t
            JOIN pg_namespace n ON n.oid = t.typnamespace
            JOIN pg_type bt ON bt.oid = t.typbasetype
            WHERE t.typtype = 'd'
              AND n.nspname NOT IN ('pg_catalog', 'information_schema')
              AND n.nspname NOT LIKE 'pg_toast%'
            ORDER BY n.nspname, t.typname
            "#,
            &[],
        )
        .await?;

    let mut domains = BTreeMap::<SchemaTypeKey, PgDomainSource>::new();
    for row in rows {
        let schema: String = row.get("nspname");
        let name: String = row.get("typname");
        let base_udt_name: String = row.get("base_udt_name");
        let key = (schema.clone(), name.clone());
        domains.insert(
            key,
            PgDomainSource {
                schema,
                name,
                base_udt_name,
            },
        );
    }
    Ok(domains)
}

pub(crate) fn collect_used_schema_types(
    relations: &mut [Relation],
) -> (
    BTreeMap<SchemaTypeKey, PgEnum>,
    BTreeMap<SchemaTypeKey, PgDomain>,
) {
    let mut enums = BTreeMap::<SchemaTypeKey, PgEnum>::new();
    let mut domains = BTreeMap::<SchemaTypeKey, PgDomain>::new();

    for relation in relations.iter() {
        for column in &relation.columns {
            match &column.field_type {
                ColumnType::Enum(pg_enum) | ColumnType::ArrayEnum(pg_enum) => {
                    enums
                        .entry(type_key(pg_enum))
                        .or_insert_with(|| pg_enum.clone());
                }
                ColumnType::Domain(domain) | ColumnType::ArrayDomain(domain) => {
                    domains
                        .entry(domain_key(domain))
                        .or_insert_with(|| domain.clone());
                }
                ColumnType::Core(_) => {}
            }
        }
    }

    assign_enum_names(&mut enums);
    assign_domain_names(&mut domains);

    for relation in relations {
        for column in &mut relation.columns {
            match &mut column.field_type {
                ColumnType::Enum(pg_enum) | ColumnType::ArrayEnum(pg_enum) => {
                    if let Some(canonical) = enums.get(&type_key(pg_enum)) {
                        *pg_enum = canonical.clone();
                    }
                }
                ColumnType::Domain(domain) | ColumnType::ArrayDomain(domain) => {
                    if let Some(canonical) = domains.get(&domain_key(domain)) {
                        *domain = canonical.clone();
                    }
                }
                ColumnType::Core(_) => {}
            }
        }
    }

    (enums, domains)
}

fn type_key(pg_enum: &PgEnum) -> SchemaTypeKey {
    (pg_enum.schema.clone(), pg_enum.name.clone())
}

fn domain_key(domain: &PgDomain) -> SchemaTypeKey {
    (domain.schema.clone(), domain.name.clone())
}

fn assign_enum_names(enums: &mut BTreeMap<SchemaTypeKey, PgEnum>) {
    let const_names = unique_ident_strings(
        enums.values().map(|pg_enum| pg_enum.const_name.clone()),
        &[],
    );
    let rust_names =
        unique_ident_strings(enums.values().map(|pg_enum| pg_enum.rust_name.clone()), &[]);

    for ((pg_enum, const_name), rust_name) in enums
        .values_mut()
        .zip(const_names.into_iter())
        .zip(rust_names.into_iter())
    {
        pg_enum.const_name = const_name;
        pg_enum.rust_name = rust_name;
    }
}

fn assign_domain_names(domains: &mut BTreeMap<SchemaTypeKey, PgDomain>) {
    let const_names = unique_ident_strings(
        domains.values().map(|domain| domain.const_name.clone()),
        &[],
    );

    for (domain, const_name) in domains.values_mut().zip(const_names.into_iter()) {
        domain.const_name = const_name;
    }
}
