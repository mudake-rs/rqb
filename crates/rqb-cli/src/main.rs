use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span};
use quote::quote;
use rqb_core::{ElemType, FieldType, SelectRepr, TypeFamily, ValueRepr};
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
    Domain(PgDomain),
}

impl ColumnType {
    fn is_jsonb(&self) -> bool {
        matches!(self, Self::Core(FieldType::Jsonb))
            || matches!(self, Self::Domain(domain) if domain.family == TypeFamily::Jsonb)
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

#[derive(Debug, Clone)]
struct PgDomain {
    schema: String,
    name: String,
    const_name: String,
    family: TypeFamily,
    value_repr: ValueRepr,
    select_repr: SelectRepr,
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
    let domains = introspect_domains(&client, schema).await?;
    let mut relations = introspect(&client, schema, only_tables, &enums, &domains).await?;
    relations.sort_by(|a, b| a.name.cmp(&b.name));

    let code = render(&relations, &enums, &domains)?;
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

async fn introspect_domains(
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
                select_repr: SelectRepr::Text,
                name,
            },
        );
    }
    Ok(domains)
}

fn render(
    relations: &[Relation],
    enums: &BTreeMap<String, PgEnum>,
    domains: &BTreeMap<String, PgDomain>,
) -> Result<String> {
    let enum_module = render_enum_module(enums);
    let type_module = render_type_module(domains);
    let relation_wrapper = render_relation_wrapper();
    let modules = relations.iter().map(render_relation);
    let tokens = quote! {
        #![allow(dead_code)]
        #![allow(clippy::module_inception)]

        #relation_wrapper
        #enum_module
        #type_module
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

fn render_type_module(domains: &BTreeMap<String, PgDomain>) -> proc_macro2::TokenStream {
    let domain_defs = domains.values().map(render_domain_def);
    quote! {
        pub mod types {
            use rqb::prelude::*;

            #(#domain_defs)*
        }
    }
}

fn render_domain_def(domain: &PgDomain) -> proc_macro2::TokenStream {
    let const_name = Ident::new(&domain.const_name, Span::call_site());
    let schema = &domain.schema;
    let name = &domain.name;
    let family = type_family_tokens(domain.family);
    let value_repr = value_repr_tokens(domain.value_repr);
    let select_repr = select_repr_tokens(domain.select_repr);

    quote! {
        pub const #const_name: TypeSpec = TypeSpec::domain(Some(#schema), #name)
            .base(#family)
            .value_repr(#value_repr)
            .select_repr(#select_repr);
    }
}

fn render_enum_def(pg_enum: &PgEnum) -> proc_macro2::TokenStream {
    let const_name = Ident::new(&pg_enum.const_name, Span::call_site());
    let rust_name = Ident::new(&pg_enum.rust_name, Span::call_site());
    let schema = &pg_enum.schema;
    let name = &pg_enum.name;
    let variants = &pg_enum.variants;
    let variant_idents = unique_enum_variant_idents(&pg_enum.variants);

    let variant_defs = variants
        .iter()
        .zip(variant_idents.iter())
        .map(|(variant, ident)| {
            quote! {
                #[serde(rename = #variant)]
                #ident
            }
        });

    quote! {
        pub const #const_name: EnumType = EnumType::new(
            Some(#schema),
            #name,
            &[#(#variants),*],
        );

        #[derive(Clone, Copy, Debug, PartialEq, Eq, rqb::serde::Serialize, rqb::serde::Deserialize)]
        #[serde(crate = "rqb::serde")]
        pub enum #rust_name {
            #(#variant_defs),*
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
        ColumnType::Domain(domain) => {
            let const_name = Ident::new(&domain.const_name, Span::call_site());
            quote! { FieldType::Custom(&super::types::#const_name) }
        }
    }
}

fn core_field_type_tokens(field_type: FieldType) -> proc_macro2::TokenStream {
    match field_type {
        FieldType::Text => quote! { FieldType::Text },
        FieldType::Citext => quote! { FieldType::Citext },
        FieldType::Integer => quote! { FieldType::Integer },
        FieldType::BigInt => quote! { FieldType::BigInt },
        FieldType::Float => quote! { FieldType::Float },
        FieldType::Numeric => quote! { FieldType::Numeric },
        FieldType::Bool => quote! { FieldType::Bool },
        FieldType::Uuid => quote! { FieldType::Uuid },
        FieldType::Timestamp => quote! { FieldType::Timestamp },
        FieldType::Timestamptz => quote! { FieldType::Timestamptz },
        FieldType::Date => quote! { FieldType::Date },
        FieldType::Jsonb => quote! { FieldType::Jsonb },
        FieldType::Bytea => quote! { FieldType::Bytea },
        FieldType::Inet => quote! { FieldType::Inet },
        FieldType::Cidr => quote! { FieldType::Cidr },
        FieldType::Custom(_) => unreachable!("custom types are rendered from ColumnType::Domain"),
        FieldType::Range(elem_type) => {
            let elem_type = elem_type_tokens(elem_type);
            quote! { FieldType::Range(#elem_type) }
        }
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
        ElemType::Citext => quote! { ElemType::Citext },
        ElemType::Int => quote! { ElemType::Int },
        ElemType::BigInt => quote! { ElemType::BigInt },
        ElemType::Float => quote! { ElemType::Float },
        ElemType::Numeric => quote! { ElemType::Numeric },
        ElemType::Bool => quote! { ElemType::Bool },
        ElemType::Uuid => quote! { ElemType::Uuid },
        ElemType::Timestamp => quote! { ElemType::Timestamp },
        ElemType::Timestamptz => quote! { ElemType::Timestamptz },
        ElemType::Date => quote! { ElemType::Date },
        ElemType::Enum(_) => {
            unreachable!("enum element types are rendered from ColumnType::ArrayEnum")
        }
    }
}

fn map_field_type(
    data_type: &str,
    udt_name: &str,
    domain_schema: Option<&str>,
    domain_name: Option<&str>,
    enums: &BTreeMap<String, PgEnum>,
    domains: &BTreeMap<String, PgDomain>,
) -> ColumnType {
    if let Some(domain_name) = domain_name
        && let Some(domain) = domains.get(domain_name)
        && domain_schema.is_none_or(|schema| schema == domain.schema)
    {
        return ColumnType::Domain(domain.clone());
    }

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
        ("ARRAY", "_text" | "_varchar") => FieldType::Array(ElemType::Text),
        ("ARRAY", "_citext") => FieldType::Array(ElemType::Citext),
        ("ARRAY", "_int2" | "_int4") => FieldType::Array(ElemType::Int),
        ("ARRAY", "_int8") => FieldType::Array(ElemType::BigInt),
        ("ARRAY", "_float4" | "_float8") => FieldType::Array(ElemType::Float),
        ("ARRAY", "_numeric") => FieldType::Array(ElemType::Numeric),
        ("ARRAY", "_bool") => FieldType::Array(ElemType::Bool),
        ("ARRAY", "_uuid") => FieldType::Array(ElemType::Uuid),
        ("ARRAY", "_timestamp") => FieldType::Array(ElemType::Timestamp),
        ("ARRAY", "_timestamptz") => FieldType::Array(ElemType::Timestamptz),
        ("ARRAY", "_date") => FieldType::Array(ElemType::Date),
        (_, "int4range") => FieldType::Range(ElemType::Int),
        (_, "int8range") => FieldType::Range(ElemType::BigInt),
        (_, "numrange") => FieldType::Range(ElemType::Numeric),
        (_, "tsrange") => FieldType::Range(ElemType::Timestamp),
        (_, "tstzrange") => FieldType::Range(ElemType::Timestamptz),
        (_, "daterange") => FieldType::Range(ElemType::Date),
        (_, "uuid") => FieldType::Uuid,
        (_, "bool") => FieldType::Bool,
        (_, "bytea") => FieldType::Bytea,
        (_, "citext") => FieldType::Citext,
        (_, "inet") => FieldType::Inet,
        (_, "cidr") => FieldType::Cidr,
        (_, "int2" | "int4") => FieldType::Integer,
        (_, "int8") => FieldType::BigInt,
        (_, "float4" | "float8") => FieldType::Float,
        (_, "numeric") => FieldType::Numeric,
        (_, "date") => FieldType::Date,
        (_, "timestamp") => FieldType::Timestamp,
        (_, "timestamptz") => FieldType::Timestamptz,
        (_, "json" | "jsonb") => FieldType::Jsonb,
        _ => FieldType::Text,
    })
}

fn type_family_for_udt(udt_name: &str) -> TypeFamily {
    match udt_name {
        "bool" => TypeFamily::Bool,
        "int2" | "int4" | "int8" | "float4" | "float8" | "numeric" => TypeFamily::Numeric,
        "uuid" => TypeFamily::Uuid,
        "date" => TypeFamily::Date,
        "timestamp" => TypeFamily::Timestamp,
        "timestamptz" => TypeFamily::Timestamptz,
        "json" | "jsonb" => TypeFamily::Jsonb,
        "bytea" => TypeFamily::Bytes,
        "inet" | "cidr" => TypeFamily::Network,
        "int4range" | "int8range" | "numrange" | "tsrange" | "tstzrange" | "daterange" => {
            TypeFamily::Range
        }
        _ => TypeFamily::Text,
    }
}

fn value_repr_for_family(family: TypeFamily) -> ValueRepr {
    match family {
        TypeFamily::Numeric => ValueRepr::DecimalString,
        TypeFamily::Bool | TypeFamily::Jsonb | TypeFamily::Bytes => ValueRepr::Native,
        TypeFamily::Text
        | TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Network
        | TypeFamily::Range => ValueRepr::String,
    }
}

fn type_family_tokens(family: TypeFamily) -> proc_macro2::TokenStream {
    match family {
        TypeFamily::Text => quote! { TypeFamily::Text },
        TypeFamily::Numeric => quote! { TypeFamily::Numeric },
        TypeFamily::Bool => quote! { TypeFamily::Bool },
        TypeFamily::Uuid => quote! { TypeFamily::Uuid },
        TypeFamily::Timestamp => quote! { TypeFamily::Timestamp },
        TypeFamily::Timestamptz => quote! { TypeFamily::Timestamptz },
        TypeFamily::Date => quote! { TypeFamily::Date },
        TypeFamily::Jsonb => quote! { TypeFamily::Jsonb },
        TypeFamily::Bytes => quote! { TypeFamily::Bytes },
        TypeFamily::Network => quote! { TypeFamily::Network },
        TypeFamily::Range => quote! { TypeFamily::Range },
    }
}

fn value_repr_tokens(value_repr: ValueRepr) -> proc_macro2::TokenStream {
    match value_repr {
        ValueRepr::Native => quote! { ValueRepr::Native },
        ValueRepr::String => quote! { ValueRepr::String },
        ValueRepr::DecimalString => quote! { ValueRepr::DecimalString },
    }
}

fn select_repr_tokens(select_repr: SelectRepr) -> proc_macro2::TokenStream {
    match select_repr {
        SelectRepr::Native => quote! { SelectRepr::Native },
        SelectRepr::Text => quote! { SelectRepr::Text },
    }
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
        let uint_256 = PgDomain {
            schema: "public".to_owned(),
            name: "uint_256".to_owned(),
            const_name: "UINT_256".to_owned(),
            family: TypeFamily::Numeric,
            value_repr: ValueRepr::DecimalString,
            select_repr: SelectRepr::Text,
        };
        let mut domains = BTreeMap::new();
        domains.insert(uint_256.name.clone(), uint_256);

        for (udt_name, expected) in [
            ("_text", ElemType::Text),
            ("_varchar", ElemType::Text),
            ("_citext", ElemType::Citext),
            ("_int2", ElemType::Int),
            ("_int4", ElemType::Int),
            ("_int8", ElemType::BigInt),
            ("_float4", ElemType::Float),
            ("_float8", ElemType::Float),
            ("_numeric", ElemType::Numeric),
            ("_bool", ElemType::Bool),
            ("_uuid", ElemType::Uuid),
            ("_timestamp", ElemType::Timestamp),
            ("_timestamptz", ElemType::Timestamptz),
            ("_date", ElemType::Date),
        ] {
            assert!(
                matches!(
                    map_field_type(
                        "ARRAY",
                        udt_name,
                        None,
                        None,
                        &BTreeMap::new(),
                        &BTreeMap::new()
                    ),
                    ColumnType::Core(FieldType::Array(actual)) if actual == expected
                ),
                "{udt_name} should map to {expected:?}"
            );
        }
        assert!(matches!(
            map_field_type(
                "USER-DEFINED",
                "uuid",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Uuid)
        ));
        assert!(matches!(
            map_field_type(
                "jsonb",
                "jsonb",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Jsonb)
        ));
        assert!(matches!(
            map_field_type(
                "timestamp with time zone",
                "timestamptz",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Timestamptz)
        ));
        for (udt_name, expected) in [
            ("bytea", FieldType::Bytea),
            ("citext", FieldType::Citext),
            ("inet", FieldType::Inet),
            ("cidr", FieldType::Cidr),
            ("int4range", FieldType::Range(ElemType::Int)),
            ("tstzrange", FieldType::Range(ElemType::Timestamptz)),
        ] {
            assert!(
                matches!(
                    map_field_type("USER-DEFINED", udt_name, None, None, &BTreeMap::new(), &BTreeMap::new()),
                    ColumnType::Core(actual) if actual == expected
                ),
                "{udt_name} should map to {expected:?}"
            );
        }
        assert!(matches!(
            map_field_type(
                "unknown",
                "ltree",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Text)
        ));
        assert!(matches!(
            map_field_type(
                "numeric",
                "numeric",
                Some("public"),
                Some("uint_256"),
                &BTreeMap::new(),
                &domains
            ),
            ColumnType::Domain(domain)
                if domain.name == "uint_256"
                    && domain.family == TypeFamily::Numeric
                    && domain.value_repr == ValueRepr::DecimalString
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
            &BTreeMap::new(),
        )
        .unwrap();

        assert!(code.contains("pub mod enums"));
        assert!(code.contains("pub const ORDER_STATUS: EnumType"));
        assert!(code.contains("pub enum OrderStatus"));
        assert!(code.contains("rqb::serde::Serialize"));
        assert!(code.contains("rqb::serde::Deserialize"));
        assert!(code.contains("#[serde(crate = \"rqb::serde\")]"));
        assert!(code.contains("#[serde(rename = \"draft\")]"));
        assert!(code.contains("impl DbEnum for OrderStatus"));
        assert!(code.contains("impl std::str::FromStr for OrderStatus"));
        assert!(code.contains("FieldType::Enum(super::enums::ORDER_STATUS)"));
        assert!(code.contains("FieldType::Array(ElemType::Enum(super::enums::ORDER_STATUS))"));
    }

    #[test]
    fn renders_postgres_domain_module_and_fields() {
        let domain = PgDomain {
            schema: "public".to_owned(),
            name: "uint_256".to_owned(),
            const_name: "UINT_256".to_owned(),
            family: TypeFamily::Numeric,
            value_repr: ValueRepr::DecimalString,
            select_repr: SelectRepr::Text,
        };
        let mut domains = BTreeMap::new();
        domains.insert(domain.name.clone(), domain.clone());

        let code = render(
            &[Relation {
                name: "withdrawals".to_owned(),
                kind: RelationKind::Table,
                columns: vec![Column {
                    name: "amount".to_owned(),
                    api_name: "amount".to_owned(),
                    rust_name: "amount".to_owned(),
                    const_name: "AMOUNT".to_owned(),
                    field_type: ColumnType::Domain(domain),
                }],
            }],
            &BTreeMap::new(),
            &domains,
        )
        .unwrap();

        assert!(code.contains("pub mod types"));
        assert!(code.contains("pub const UINT_256: TypeSpec"));
        assert!(code.contains("TypeSpec::domain(Some(\"public\"), \"uint_256\")"));
        assert!(code.contains(".base(TypeFamily::Numeric)"));
        assert!(code.contains(".value_repr(ValueRepr::DecimalString)"));
        assert!(code.contains(".select_repr(SelectRepr::Text)"));
        assert!(code.contains("FieldType::Custom(&super::types::UINT_256)"));
    }

    #[test]
    fn sanitizes_identifiers_and_disambiguates_enum_variants() {
        assert_eq!(sanitize_ident("type"), "type_");
        assert_eq!(sanitize_ident("123bad-name"), "_123bad_name");
        assert_eq!(sanitize_ident(""), "_");

        let variants = vec![
            "foo-bar".to_owned(),
            "foo_bar".to_owned(),
            "foo bar".to_owned(),
        ];
        let names = unique_enum_variant_idents(&variants)
            .into_iter()
            .map(|ident| ident.to_string())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["FooBar", "FooBar_1", "FooBar_2"]);
    }
}
