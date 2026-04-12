use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span};
use quote::quote;
use rqb_core::{ElemType, FieldType};
use tokio_postgres::NoTls;

#[derive(Parser)]
#[command(name = "rqb")]
#[command(about = "Schema introspection and code generation for rqb")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Generate {
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
        #[arg(long, default_value = "public")]
        schema: String,
        #[arg(long)]
        table: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Clone)]
struct Relation {
    name: String,
    kind: RelationKind,
    columns: Vec<Column>,
}

#[derive(Debug, Clone, Copy)]
enum RelationKind {
    Table,
    View,
}

#[derive(Debug, Clone)]
struct Column {
    name: String,
    api_name: String,
    rust_name: String,
    const_name: String,
    field_type: ColumnType,
}

#[derive(Debug, Clone)]
enum ColumnType {
    Core(FieldType),
    Enum(PgEnum),
    ArrayEnum(PgEnum),
}

impl ColumnType {
    fn is_jsonb(&self) -> bool {
        matches!(self, Self::Core(FieldType::Jsonb))
    }

    fn is_array(&self) -> bool {
        matches!(self, Self::Core(field_type) if field_type.is_array())
            || matches!(self, Self::ArrayEnum(_))
    }
}

#[derive(Debug, Clone)]
struct PgEnum {
    schema: String,
    name: String,
    const_name: String,
    rust_name: String,
    variants: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            database_url,
            schema,
            table,
            out,
        } => generate(&database_url, &schema, &table, out).await,
    }
}

async fn generate(
    database_url: &str,
    schema: &str,
    only_tables: &[String],
    out: PathBuf,
) -> Result<()> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .context("failed to connect to Postgres")?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("postgres connection error: {error}");
        }
    });

    let enums = introspect_enums(&client, schema).await?;
    let mut relations = introspect(&client, schema, only_tables, &enums).await?;
    relations.sort_by(|a, b| a.name.cmp(&b.name));

    let code = render(&relations, &enums)?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&out, code).with_context(|| format!("failed to write {}", out.display()))?;
    Ok(())
}

async fn introspect(
    client: &tokio_postgres::Client,
    schema: &str,
    only_tables: &[String],
    enums: &BTreeMap<String, PgEnum>,
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
            SELECT table_name, column_name, data_type, udt_name
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
        relation.columns.push(Column {
            api_name: name.to_lower_camel_case(),
            rust_name: sanitize_ident(&name.to_snake_case()),
            const_name: sanitize_ident(&name.to_shouty_snake_case()),
            field_type: map_field_type(&data_type, &udt_name, enums),
            name,
        });
    }

    Ok(relations.into_values().collect())
}

async fn introspect_enums(
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

fn render(relations: &[Relation], enums: &BTreeMap<String, PgEnum>) -> Result<String> {
    let enum_module = render_enum_module(enums);
    let relation_wrapper = render_relation_wrapper();
    let modules = relations.iter().map(render_relation);
    let tokens = quote! {
        #![allow(dead_code)]
        #![allow(clippy::module_inception)]

        #relation_wrapper
        #enum_module
        #(#modules)*
    };
    let syntax = syn::parse2(tokens).context("generated schema code did not parse")?;
    Ok(prettyplease::unparse(&syntax))
}

fn render_relation(relation: &Relation) -> proc_macro2::TokenStream {
    let module = Ident::new(
        &sanitize_ident(&relation.name.to_snake_case()),
        Span::call_site(),
    );
    let db_name = &relation.name;
    let dataset_ctor = match relation.kind {
        RelationKind::Table => quote! { Dataset::table(#db_name) },
        RelationKind::View => quote! { Dataset::view(#db_name) },
    };
    let fields = relation
        .columns
        .iter()
        .map(render_field_const)
        .collect::<Vec<_>>();
    let field_names = relation
        .columns
        .iter()
        .map(|column| Ident::new(&column.const_name, Span::call_site()));
    let relation_ctor = match relation.kind {
        RelationKind::Table => quote! {
            pub fn table() -> Relation {
                Relation::new(dataset())
            }
        },
        RelationKind::View => quote! {
            pub fn view() -> Relation {
                Relation::new(dataset())
            }
        },
    };
    let field_methods = relation.columns.iter().map(render_relation_field_method);

    quote! {
        pub mod #module {
            use rqb::prelude::*;

            #(#fields)*

            pub fn dataset() -> Dataset {
                #dataset_ctor.fields([#(#field_names),*])
            }

            #relation_ctor

            __rqb_relation_wrapper!();

            impl Relation {
                #(#field_methods)*
            }
        }
    }
}

fn render_relation_wrapper() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! __rqb_relation_wrapper {
            () => {
                #[derive(Clone, Debug)]
                pub struct Relation {
                    inner: rqb::prelude::Relation,
                }

                impl Relation {
                    fn new(dataset: Dataset) -> Self {
                        Self {
                            inner: rqb::prelude::Relation::new(dataset),
                        }
                    }

                    pub fn alias(mut self, alias: impl Into<String>) -> Self {
                        self.inner = self.inner.alias(alias);
                        self
                    }

                    pub fn dataset(&self) -> Dataset {
                        self.inner.dataset().clone()
                    }
                }

                impl From<Relation> for Dataset {
                    fn from(value: Relation) -> Self {
                        value.inner.into()
                    }
                }

                impl From<&Relation> for Dataset {
                    fn from(value: &Relation) -> Self {
                        value.dataset()
                    }
                }
            };
        }
    }
}

fn render_enum_module(enums: &BTreeMap<String, PgEnum>) -> proc_macro2::TokenStream {
    let enum_defs = enums.values().map(render_enum_def);
    quote! {
        pub mod enums {
            use rqb::prelude::*;

            #(#enum_defs)*
        }
    }
}

fn render_enum_def(pg_enum: &PgEnum) -> proc_macro2::TokenStream {
    let const_name = Ident::new(&pg_enum.const_name, Span::call_site());
    let rust_name = Ident::new(&pg_enum.rust_name, Span::call_site());
    let schema = &pg_enum.schema;
    let name = &pg_enum.name;
    let variants = &pg_enum.variants;
    let variant_idents = unique_enum_variant_idents(&pg_enum.variants);

    quote! {
        pub const #const_name: EnumType = EnumType::new(
            Some(#schema),
            #name,
            &[#(#variants),*],
        );

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum #rust_name {
            #(#variant_idents),*
        }

        impl #rust_name {
            pub const fn as_db_str(self) -> &'static str {
                match self {
                    #(Self::#variant_idents => #variants),*
                }
            }
        }

        impl DbEnum for #rust_name {
            const TYPE: EnumType = #const_name;

            fn as_db_str(self) -> &'static str {
                #rust_name::as_db_str(self)
            }
        }

        impl std::fmt::Display for #rust_name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_db_str())
            }
        }

        impl std::str::FromStr for #rust_name {
            type Err = &'static str;

            fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
                match value {
                    #(#variants => Ok(Self::#variant_idents),)*
                    _ => Err("unknown enum variant"),
                }
            }
        }
    }
}

fn unique_enum_variant_idents(variants: &[String]) -> Vec<Ident> {
    let mut seen = BTreeMap::<String, usize>::new();
    variants
        .iter()
        .map(|variant| {
            let mut name = sanitize_ident(&variant.to_upper_camel_case());
            let count = seen.entry(name.clone()).or_insert(0);
            if *count > 0 {
                name.push('_');
                name.push_str(&count.to_string());
            }
            *count += 1;
            Ident::new(&name, Span::call_site())
        })
        .collect()
}

fn render_relation_field_method(column: &Column) -> proc_macro2::TokenStream {
    let rust_name = Ident::new(&column.rust_name, Span::call_site());
    let const_name = Ident::new(&column.const_name, Span::call_site());
    quote! {
        pub fn #rust_name(&self) -> FieldRef {
            self.inner.field(#const_name)
        }
    }
}

fn render_field_const(column: &Column) -> proc_macro2::TokenStream {
    let const_name = Ident::new(&column.const_name, Span::call_site());
    let api_name = &column.api_name;
    let db_name = &column.name;
    let field_type = field_type_tokens(&column.field_type);
    let mut expr = if column.api_name == column.name {
        quote! { Field::new(#api_name, #field_type) }
    } else {
        quote! { Field::mapped(#api_name, #db_name, #field_type) }
    };

    if column.field_type.is_jsonb() {
        expr = quote! { #expr.sortable(false).json_paths(JsonPathPolicy::Dynamic) };
    }
    if column.field_type.is_array() {
        expr = quote! { #expr.sortable(false) };
    }

    quote! {
        pub const #const_name: Field = #expr;
    }
}

fn field_type_tokens(field_type: &ColumnType) -> proc_macro2::TokenStream {
    match field_type {
        ColumnType::Core(field_type) => core_field_type_tokens(*field_type),
        ColumnType::Enum(pg_enum) => {
            let const_name = Ident::new(&pg_enum.const_name, Span::call_site());
            quote! { FieldType::Enum(super::enums::#const_name) }
        }
        ColumnType::ArrayEnum(pg_enum) => {
            let const_name = Ident::new(&pg_enum.const_name, Span::call_site());
            quote! { FieldType::Array(ElemType::Enum(super::enums::#const_name)) }
        }
    }
}

fn core_field_type_tokens(field_type: FieldType) -> proc_macro2::TokenStream {
    match field_type {
        FieldType::Text => quote! { FieldType::Text },
        FieldType::Integer => quote! { FieldType::Integer },
        FieldType::BigInt => quote! { FieldType::BigInt },
        FieldType::Float => quote! { FieldType::Float },
        FieldType::Numeric => quote! { FieldType::Numeric },
        FieldType::Bool => quote! { FieldType::Bool },
        FieldType::Uuid => quote! { FieldType::Uuid },
        FieldType::Timestamp => quote! { FieldType::Timestamp },
        FieldType::Date => quote! { FieldType::Date },
        FieldType::Jsonb => quote! { FieldType::Jsonb },
        FieldType::Array(elem_type) => {
            let elem_type = elem_type_tokens(elem_type);
            quote! { FieldType::Array(#elem_type) }
        }
        FieldType::Enum(_) => unreachable!("core enum types are rendered from ColumnType::Enum"),
    }
}

fn elem_type_tokens(elem_type: ElemType) -> proc_macro2::TokenStream {
    match elem_type {
        ElemType::Text => quote! { ElemType::Text },
        ElemType::Int => quote! { ElemType::Int },
        ElemType::BigInt => quote! { ElemType::BigInt },
        ElemType::Float => quote! { ElemType::Float },
        ElemType::Numeric => quote! { ElemType::Numeric },
        ElemType::Bool => quote! { ElemType::Bool },
        ElemType::Uuid => quote! { ElemType::Uuid },
        ElemType::Timestamp => quote! { ElemType::Timestamp },
        ElemType::Date => quote! { ElemType::Date },
        ElemType::Enum(_) => {
            unreachable!("enum element types are rendered from ColumnType::ArrayEnum")
        }
    }
}

fn map_field_type(data_type: &str, udt_name: &str, enums: &BTreeMap<String, PgEnum>) -> ColumnType {
    if data_type == "USER-DEFINED"
        && let Some(pg_enum) = enums.get(udt_name)
    {
        return ColumnType::Enum(pg_enum.clone());
    }
    if data_type == "ARRAY"
        && let Some(enum_name) = udt_name.strip_prefix('_')
        && let Some(pg_enum) = enums.get(enum_name)
    {
        return ColumnType::ArrayEnum(pg_enum.clone());
    }

    ColumnType::Core(match (data_type, udt_name) {
        ("ARRAY", "_text" | "_varchar" | "_citext") => FieldType::Array(ElemType::Text),
        ("ARRAY", "_int2" | "_int4") => FieldType::Array(ElemType::Int),
        ("ARRAY", "_int8") => FieldType::Array(ElemType::BigInt),
        ("ARRAY", "_float4" | "_float8") => FieldType::Array(ElemType::Float),
        ("ARRAY", "_numeric") => FieldType::Array(ElemType::Numeric),
        ("ARRAY", "_bool") => FieldType::Array(ElemType::Bool),
        ("ARRAY", "_uuid") => FieldType::Array(ElemType::Uuid),
        ("ARRAY", "_timestamp" | "_timestamptz") => FieldType::Array(ElemType::Timestamp),
        ("ARRAY", "_date") => FieldType::Array(ElemType::Date),
        (_, "uuid") => FieldType::Uuid,
        (_, "bool") => FieldType::Bool,
        (_, "int2" | "int4") => FieldType::Integer,
        (_, "int8") => FieldType::BigInt,
        (_, "float4" | "float8") => FieldType::Float,
        (_, "numeric") => FieldType::Numeric,
        (_, "date") => FieldType::Date,
        (_, "timestamp" | "timestamptz") => FieldType::Timestamp,
        (_, "json" | "jsonb") => FieldType::Jsonb,
        _ => FieldType::Text,
    })
}

fn sanitize_ident(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if out.is_empty() {
        out.push('_');
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if is_rust_keyword(&out) {
        out.push('_');
    }
    out
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_ergonomic_generated_module() {
        let code = render(
            &[Relation {
                name: "order_search_view".to_owned(),
                kind: RelationKind::View,
                columns: vec![
                    Column {
                        name: "created_at".to_owned(),
                        api_name: "createdAt".to_owned(),
                        rust_name: "created_at".to_owned(),
                        const_name: "CREATED_AT".to_owned(),
                        field_type: ColumnType::Core(FieldType::Timestamp),
                    },
                    Column {
                        name: "metadata".to_owned(),
                        api_name: "metadata".to_owned(),
                        rust_name: "metadata".to_owned(),
                        const_name: "METADATA".to_owned(),
                        field_type: ColumnType::Core(FieldType::Jsonb),
                    },
                ],
            }],
            &BTreeMap::new(),
        )
        .unwrap();

        assert!(code.contains("#![allow(dead_code)]"));
        assert!(code.contains("pub mod order_search_view"));
        assert!(code.contains("pub const CREATED_AT: Field"));
        assert!(code.contains("\"createdAt\""));
        assert!(code.contains("\"created_at\""));
        assert!(code.contains("Field::mapped"));
        assert!(code.contains("FieldType::Timestamp"));
        assert!(code.contains("Dataset::view(\"order_search_view\")"));
        assert!(code.contains("pub fn view() -> Relation"));
        assert!(!code.contains("pub fn relation()"));
        assert!(code.contains("pub fn created_at(&self) -> FieldRef"));
        assert!(code.contains("macro_rules! __rqb_relation_wrapper"));
        assert!(code.contains("__rqb_relation_wrapper!();"));
        assert!(code.contains("self.inner.field(CREATED_AT)"));
        assert!(code.contains(".json_paths(JsonPathPolicy::Dynamic)"));
    }

    #[test]
    fn maps_postgres_types() {
        assert!(matches!(
            map_field_type("ARRAY", "_text", &BTreeMap::new()),
            ColumnType::Core(FieldType::Array(ElemType::Text))
        ));
        assert!(matches!(
            map_field_type("USER-DEFINED", "uuid", &BTreeMap::new()),
            ColumnType::Core(FieldType::Uuid)
        ));
        assert!(matches!(
            map_field_type("jsonb", "jsonb", &BTreeMap::new()),
            ColumnType::Core(FieldType::Jsonb)
        ));
        assert!(matches!(
            map_field_type("timestamp with time zone", "timestamptz", &BTreeMap::new()),
            ColumnType::Core(FieldType::Timestamp)
        ));
    }

    #[test]
    fn renders_postgres_enum_module_and_fields() {
        let pg_enum = PgEnum {
            schema: "public".to_owned(),
            name: "order_status".to_owned(),
            const_name: "ORDER_STATUS".to_owned(),
            rust_name: "OrderStatus".to_owned(),
            variants: vec!["draft".to_owned(), "paid".to_owned()],
        };
        let mut enums = BTreeMap::new();
        enums.insert(pg_enum.name.clone(), pg_enum.clone());

        let code = render(
            &[Relation {
                name: "orders".to_owned(),
                kind: RelationKind::Table,
                columns: vec![
                    Column {
                        name: "status".to_owned(),
                        api_name: "status".to_owned(),
                        rust_name: "status".to_owned(),
                        const_name: "STATUS".to_owned(),
                        field_type: ColumnType::Enum(pg_enum.clone()),
                    },
                    Column {
                        name: "status_history".to_owned(),
                        api_name: "statusHistory".to_owned(),
                        rust_name: "status_history".to_owned(),
                        const_name: "STATUS_HISTORY".to_owned(),
                        field_type: ColumnType::ArrayEnum(pg_enum),
                    },
                ],
            }],
            &enums,
        )
        .unwrap();

        assert!(code.contains("pub mod enums"));
        assert!(code.contains("pub const ORDER_STATUS: EnumType"));
        assert!(code.contains("pub enum OrderStatus"));
        assert!(code.contains("impl DbEnum for OrderStatus"));
        assert!(code.contains("impl std::str::FromStr for OrderStatus"));
        assert!(code.contains("FieldType::Enum(super::enums::ORDER_STATUS)"));
        assert!(code.contains("FieldType::Array(ElemType::Enum(super::enums::ORDER_STATUS))"));
    }
}
