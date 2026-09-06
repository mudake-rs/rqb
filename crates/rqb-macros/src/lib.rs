//! Procedural macros used by the `rqb` facade crate.
//!
//! Applications normally use these macros through `rqb::schema!`,
//! `#[derive(rqb::Insertable)]`, and `#[derive(rqb::Changeset)]`.

use heck::ToShoutySnakeCase;
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, Span};
use quote::{quote, quote_spanned};
use std::collections::HashSet;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, GenericArgument, LitBool, LitStr, Path,
    PathArguments, Result, Token, Type, braced, parse_macro_input, token,
};

/// Generates compact rqb schema modules.
///
/// Grammar:
///
/// ```text
/// (table|view) <schema>.<relation> [as <module>] {
///     [#[rqb(ops = none|equality|ordered|text, json = none|text|...)]]
///     <db_column> [as <CONST>]: <pg_type> [= <rust_type>],
///     ...
///     constraints {
///         <CONST>: <constraint_name>,
///     }
/// }
/// ```
///
/// A column with `= <rust_type>` emits both metadata and a typed
/// `pub const NAME: Field<T>`. A column without a Rust type emits metadata
/// only, which is useful for extension or user-defined PostgreSQL types that
/// should remain raw-only.
///
/// Each generated relation module contains:
/// - `*_META` values for all columns;
/// - typed field constants for columns with a Rust type;
/// - `FIELDS`, used by default projection and JSON search metadata lookup;
/// - `table()` or `view()` source constructors;
/// - `alias("u")` handles with alias-bound field methods;
/// - optional `constraints::*` string constants for unique constraints.
///
/// `ops = ...` controls which typed/search operators are accepted for a field.
/// `json = ...` exposes the field to `rqb::SearchRequest` with that client
/// wire shape; `json = none` keeps it hidden from client search even if it
/// remains usable from Rust builder code.
///
/// Generated metadata intentionally uses database column names as public API
/// names. HTTP/JSON casing belongs in application DTOs, not generated schema.
/// Rename columns that collide with generated helpers, for example
/// `fields as FIELDS_1: int4 = i32`. The CLI does this automatically.
///
/// ```rust,ignore
/// rqb::schema! {
///     table public.users {
///         id: uuid = uuid::Uuid,
///         email: text = String,
///         embedding: vector,
///     }
/// }
/// ```
///
/// Each generated relation also exposes `alias("u")`, which returns an
/// alias-bound handle for join-heavy code:
///
/// ```rust,ignore
/// let u = users::alias("u");
/// select(&u).column(u.email());
/// ```
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as SchemaInput).expand().into()
}

/// Derives `rqb::Insertable` for a DTO.
///
/// Required container attribute:
///
/// ```rust,ignore
/// #[rqb(table = crate::schema::users)]
/// ```
///
/// By default, a Rust field named `display_name` maps to the generated schema
/// constant `DISPLAY_NAME` in the configured table module.
///
/// Builder assignments use replacement semantics, so application code can call
/// `insert(table).values(&dto).set(server_field.set(value))` to let
/// server-owned values override DTO fields.
///
/// For DTO batches, use `insert(table).values_many(&rows)`.
/// Each row must produce the same fields in the same order.
///
/// Field attributes:
/// - `#[rqb(field = TABLE::FIELD)]` maps a Rust field to a differently named
///   generated schema field.
/// - `#[rqb(skip)]` omits a local-only field.
/// - `#[rqb(skip_none)]` omits `None` for `Option<T>` fields. Without this
///   attribute, `Some(T)` writes the typed value and `None` writes SQL NULL.
///   Generated nullable columns still use `Field<T>`, not `Field<Option<T>>`.
///
/// ```rust,ignore
/// #[derive(rqb::Insertable)]
/// #[rqb(table = crate::schema::users)]
/// struct NewUser {
///     #[rqb(field = crate::schema::users::EMAIL)]
///     login_email: String,
///     display_name: String,
///     #[rqb(skip_none)]
///     invited_by: Option<uuid::Uuid>,
///     #[rqb(skip)]
///     request_id: uuid::Uuid,
/// }
/// ```
#[proc_macro_derive(Insertable, attributes(rqb))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_write_record(
        parse_macro_input!(input as DeriveInput),
        WriteKind::Insertable,
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

/// Derives `rqb::Changeset` for patch DTOs.
///
/// `Option<T>` fields naturally model PATCH semantics: `Some(value)` sets the
/// column and `None` leaves it unchanged. Non-optional fields always produce an
/// assignment.
/// `Option<Option<T>>` adds explicit NULL: outer `None` omits, `Some(None)`
/// clears, and `Some(Some(T))` sets. For HTTP JSON, use a present-field
/// deserializer: serde's default nested Option handling collapses missing and null.
///
/// Builder assignments use replacement semantics, so call `patch(&dto)` before
/// `set(...)` when authenticated/server-owned values must override request
/// fields.
///
/// The same `#[rqb(table = ...)]`, `#[rqb(field = ...)]`, and `#[rqb(skip)]`
/// attributes supported by [`Insertable`] are available here.
///
/// ```rust,ignore
/// #[derive(rqb::Changeset)]
/// #[rqb(table = crate::schema::users)]
/// struct PatchUser {
///     display_name: Option<String>,
///     active: Option<bool>,
///     #[rqb(skip)]
///     actor_id: uuid::Uuid,
/// }
/// ```
#[proc_macro_derive(Changeset, attributes(rqb))]
pub fn derive_changeset(input: TokenStream) -> TokenStream {
    expand_write_record(
        parse_macro_input!(input as DeriveInput),
        WriteKind::Changeset,
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

#[derive(Clone, Copy)]
enum WriteKind {
    Insertable,
    Changeset,
}

enum RelationKind {
    Table,
    View,
}

struct SchemaInput {
    relations: Vec<RelationInput>,
}

struct RelationInput {
    kind: RelationKind,
    qualified_name: String,
    module: Ident,
    columns: Vec<ColumnInput>,
    constraints: Vec<ConstraintInput>,
}

struct ColumnInput {
    attrs: SchemaFieldAttrs,
    db: String,
    const_ident: Ident,
    pg: String,
    rust_ty: Option<Type>,
}

struct ConstraintInput {
    const_ident: Ident,
    name: String,
}

#[derive(Default)]
struct ContainerAttrs {
    table: Option<Path>,
}

#[derive(Default)]
struct FieldAttrs {
    field: Option<Path>,
    skip: bool,
    skip_none: bool,
}

#[derive(Default)]
struct SchemaFieldAttrs {
    ops: Option<SchemaOps>,
    json: Option<SchemaJson>,
}

#[derive(Clone, Copy)]
enum SchemaOps {
    None,
    Equality,
    Ordered,
    Text,
}

#[derive(Clone, Copy)]
enum SchemaJson {
    None,
    Text,
    Bool,
    Integer,
    BigInt,
    Float,
    NumericString,
    Uuid,
    Date,
    Time,
    Timestamp,
    Timestamptz,
    Jsonb,
}

mod kw {
    syn::custom_keyword!(constraints);
    syn::custom_keyword!(table);
    syn::custom_keyword!(view);
}

impl Parse for SchemaInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut relations = Vec::new();
        while !input.is_empty() {
            relations.push(input.parse()?);
        }
        Ok(Self { relations })
    }
}

impl SchemaInput {
    fn expand(self) -> proc_macro2::TokenStream {
        let relations = self.relations.into_iter().map(RelationInput::expand);
        quote! {
            #(#relations)*
        }
    }
}

impl Parse for RelationInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let kind = if input.peek(kw::table) {
            input.parse::<kw::table>()?;
            RelationKind::Table
        } else if input.peek(kw::view) {
            input.parse::<kw::view>()?;
            RelationKind::View
        } else {
            return Err(input.error("expected `table` or `view`"));
        };

        let name = parse_relation_name(input)?;
        let module = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            input.call(Ident::parse_any)?
        } else {
            Ident::new(&sanitize_ident(&name.default_module), Span::call_site())
        };

        let content;
        braced!(content in input);
        let mut columns = Vec::new();
        let mut constraints = Vec::new();
        while !content.is_empty() {
            if starts_constraints_block(&content) {
                if !constraints.is_empty() {
                    return Err(content.error("duplicate constraints block"));
                }
                constraints = parse_constraints_block(&content)?;
            } else {
                columns.push(content.parse()?);
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        }

        Ok(Self {
            kind,
            qualified_name: name.qualified,
            module,
            columns,
            constraints,
        })
    }
}

impl RelationInput {
    fn expand(self) -> proc_macro2::TokenStream {
        let module = self.module;
        let qualified_name = self.qualified_name;
        let module_doc = Literal::string(&format!("Generated rqb schema for `{qualified_name}`."));
        let fields_doc = Literal::string(&format!(
            "Root field metadata for the default `{qualified_name}` projection."
        ));
        let alias_doc = Literal::string(&format!(
            "Creates an alias-bound handle for `{qualified_name}`."
        ));
        let alias_struct_doc = Literal::string(&format!(
            "Alias-bound accessors for `{qualified_name}` fields."
        ));
        let source_doc = Literal::string(&format!(
            "Returns `{qualified_name}` as a source with this alias."
        ));
        let constructor = match self.kind {
            RelationKind::Table => quote! { ::rqb::table(#qualified_name, &FIELDS) },
            RelationKind::View => quote! { ::rqb::view(#qualified_name, &FIELDS) },
        };
        let constructor_fn = match self.kind {
            RelationKind::Table => quote! {
                #[doc = "Creates the table source for this relation."]
                pub fn table() -> ::rqb::Source {
                    #constructor
                }
            },
            RelationKind::View => quote! {
                #[doc = "Creates the view source for this relation."]
                pub fn view() -> ::rqb::Source {
                    #constructor
                }
            },
        };

        let mut names = self
            .columns
            .iter()
            .filter(|column| column.rust_ty.is_some())
            .map(|column| column.const_ident.to_string())
            .collect::<HashSet<_>>();
        let mut methods = HashSet::new();
        let columns = self
            .columns
            .into_iter()
            .map(|column| {
                let ident = &column.const_ident;
                let meta = unique_ident(format!("{ident}_META"), ident.span(), &mut names);
                column.expand(meta, &mut methods)
            })
            .collect::<Vec<_>>();
        let metas = columns.iter().map(|column| &column.meta);
        let fields = columns.iter().filter_map(|column| column.field.as_ref());
        let alias_methods = columns
            .iter()
            .filter_map(|column| column.alias_method.as_ref());
        let meta_idents = columns.iter().map(|column| &column.meta_ident);
        let field_count = Literal::usize_unsuffixed(columns.len());
        let constraints = expand_constraints(self.constraints);

        quote! {
            #[doc = #module_doc]
            pub mod #module {
                // Bring caller-scope Rust types into the generated module.
                #[allow(unused_imports)]
                use super::*;

                #(#metas)*
                #(#fields)*

                #[doc = #fields_doc]
                pub static FIELDS: [&'static ::rqb::Meta; #field_count] = [#(&#meta_idents),*];

                #constructor_fn
                #constraints

                #[doc = #alias_doc]
                pub fn alias(alias: impl Into<String>) -> Alias {
                    Alias {
                        alias: alias.into(),
                    }
                }

                #[doc = #alias_struct_doc]
                #[derive(Clone, Debug)]
                pub struct Alias {
                    alias: String,
                }

                impl Alias {
                    #[doc = #source_doc]
                    pub fn source(&self) -> ::rqb::Source {
                        (#constructor).alias(self.alias.clone())
                    }

                    #(#alias_methods)*
                }

                impl From<&Alias> for ::rqb::Source {
                    fn from(alias: &Alias) -> Self {
                        alias.source()
                    }
                }

                impl From<Alias> for ::rqb::Source {
                    fn from(alias: Alias) -> Self {
                        (#constructor).alias(alias.alias)
                    }
                }
            }
        }
    }
}

struct ExpandedColumn {
    meta_ident: Ident,
    meta: proc_macro2::TokenStream,
    field: Option<proc_macro2::TokenStream>,
    alias_method: Option<proc_macro2::TokenStream>,
}

impl Parse for ColumnInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = parse_schema_field_attrs(&Attribute::parse_outer(input)?)?;
        let db = parse_name(input)?;
        let const_ident = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            input.call(Ident::parse_any)?
        } else {
            Ident::new(
                &sanitize_ident(&db.to_shouty_snake_case()),
                Span::call_site(),
            )
        };

        input.parse::<Token![:]>()?;
        let pg = parse_pg_name(input)?;
        if pg.ends_with("[]") && !matches!(attrs.json, None | Some(SchemaJson::None)) {
            return Err(Error::new_spanned(
                &const_ident,
                "array fields do not support JSON search exposure; use json = none",
            ));
        }
        let rust_ty = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        if rust_ty.is_some()
            && matches!(
                const_ident.to_string().as_str(),
                "FIELDS" | "table" | "view" | "alias"
            )
        {
            return Err(Error::new_spanned(
                &const_ident,
                "field constant conflicts with a generated relation helper; choose another name with `as FIELD_NAME`",
            ));
        }

        Ok(Self {
            attrs,
            db,
            const_ident,
            pg,
            rust_ty,
        })
    }
}

impl ColumnInput {
    fn expand(self, meta_ident: Ident, methods: &mut HashSet<String>) -> ExpandedColumn {
        let db = self.db;
        let pg = self.pg;
        let ops = ops_tokens(&pg, self.rust_ty.is_some(), self.attrs.ops);
        let json = json_kind_tokens(&pg, self.rust_ty.is_some(), self.attrs.json);
        let const_ident = self.const_ident;
        let meta_doc = Literal::string(&format!("Metadata for `{db}` (`{pg}`)."));
        let field_doc = Literal::string(&format!("Typed field for `{db}` (`{pg}`)."));
        let alias_method_doc = Literal::string(&format!("Returns `{db}` bound to this alias."));

        let mut meta_expr = quote! { ::rqb::Meta::col(#db, #pg).ops(#ops) };
        if let Some(json) = json {
            meta_expr = quote! { #meta_expr.json(#json) };
        }
        let meta = quote! {
            #[doc = #meta_doc]
            pub static #meta_ident: ::rqb::Meta = #meta_expr;
        };

        let (field, alias_method) = match self.rust_ty {
            Some(rust_ty) => {
                let method_ident = unique_ident(
                    sanitize_alias_method_ident(&const_ident.to_string().to_snake_case()),
                    const_ident.span(),
                    methods,
                );
                let field = quote! {
                    #[doc = #field_doc]
                    pub const #const_ident: ::rqb::Field<#rust_ty> = ::rqb::Field::new(&#meta_ident);
                };
                let alias_method = quote! {
                    #[doc = #alias_method_doc]
                    pub fn #method_ident(&self) -> ::rqb::FieldRef<#rust_ty> {
                        #const_ident.at(self.alias.clone())
                    }
                };
                (Some(field), Some(alias_method))
            }
            None => (None, None),
        };

        ExpandedColumn {
            meta_ident,
            meta,
            field,
            alias_method,
        }
    }
}

fn starts_constraints_block(input: ParseStream<'_>) -> bool {
    if !input.peek(kw::constraints) {
        return false;
    }
    let fork = input.fork();
    fork.parse::<kw::constraints>().is_ok() && fork.peek(token::Brace)
}

fn parse_constraints_block(input: ParseStream<'_>) -> Result<Vec<ConstraintInput>> {
    input.parse::<kw::constraints>()?;
    let content;
    braced!(content in input);
    let mut constraints = Vec::new();
    while !content.is_empty() {
        constraints.push(content.parse()?);
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(constraints)
}

impl Parse for ConstraintInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let const_ident = input.call(Ident::parse_any)?;
        input.parse::<Token![:]>()?;
        let name = parse_name(input)?;

        Ok(Self { const_ident, name })
    }
}

fn expand_constraints(constraints: Vec<ConstraintInput>) -> proc_macro2::TokenStream {
    if constraints.is_empty() {
        return quote! {};
    }

    let items = constraints.into_iter().map(|constraint| {
        let const_ident = constraint.const_ident;
        let name = Literal::string(&constraint.name);
        let doc = Literal::string(&format!(
            "Database unique constraint `{}`.",
            constraint.name
        ));
        quote! {
            #[doc = #doc]
            pub const #const_ident: &str = #name;
        }
    });

    quote! {
        #[doc = "Database unique constraint names for this relation."]
        pub mod constraints {
            #(#items)*
        }
    }
}

struct ParsedRelationName {
    qualified: String,
    default_module: String,
}

fn parse_relation_name(input: ParseStream<'_>) -> Result<ParsedRelationName> {
    if input.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        let qualified = lit.value();
        let default_module = qualified
            .rsplit('.')
            .next()
            .map(ToSnakeCase::to_snake_case)
            .unwrap_or_else(|| qualified.to_snake_case());
        return Ok(ParsedRelationName {
            qualified,
            default_module,
        });
    }

    let schema: Ident = input.call(Ident::parse_any)?;
    input.parse::<Token![.]>()?;
    let relation: Ident = input.call(Ident::parse_any)?;
    let schema = schema.to_string();
    let relation = relation.to_string();
    Ok(ParsedRelationName {
        qualified: format!("{schema}.{relation}"),
        default_module: relation.to_snake_case(),
    })
}

fn parse_name(input: ParseStream<'_>) -> Result<String> {
    let db = if input.peek(LitStr) {
        input.parse::<LitStr>()?.value()
    } else {
        input.call(Ident::parse_any)?.to_string()
    };
    Ok(db)
}

fn parse_pg_name(input: ParseStream<'_>) -> Result<String> {
    if input.peek(LitStr) {
        return Ok(input.parse::<LitStr>()?.value());
    }
    Ok(input.call(Ident::parse_any)?.to_string())
}

fn ops_tokens(pg: &str, typed: bool, override_ops: Option<SchemaOps>) -> proc_macro2::TokenStream {
    if let Some(ops) = override_ops {
        return schema_ops_tokens(ops);
    }
    if !typed || pg == "json" {
        return quote! { ::rqb::OpSet::none() };
    }
    if is_text_pattern_pg(pg) {
        return quote! { ::rqb::OpSet::text() };
    }
    if is_equality_only_pg(pg) {
        quote! { ::rqb::OpSet::equality() }
    } else {
        quote! { ::rqb::OpSet::ordered() }
    }
}

fn schema_ops_tokens(ops: SchemaOps) -> proc_macro2::TokenStream {
    match ops {
        SchemaOps::None => quote! { ::rqb::OpSet::none() },
        SchemaOps::Equality => quote! { ::rqb::OpSet::equality() },
        SchemaOps::Ordered => quote! { ::rqb::OpSet::ordered() },
        SchemaOps::Text => quote! { ::rqb::OpSet::text() },
    }
}

fn is_text_pattern_pg(pg: &str) -> bool {
    matches!(pg, "text" | "varchar" | "bpchar" | "citext")
}

fn is_equality_only_pg(pg: &str) -> bool {
    matches!(
        pg,
        "bool"
            | "jsonb"
            | "bytea"
            | "int4range"
            | "int8range"
            | "numrange"
            | "daterange"
            | "tsrange"
            | "tstzrange"
    ) || pg.ends_with("[]")
}

fn json_kind_tokens(
    pg: &str,
    typed: bool,
    override_json: Option<SchemaJson>,
) -> Option<proc_macro2::TokenStream> {
    if pg.ends_with("[]") {
        return None;
    }
    if let Some(json) = override_json {
        return schema_json_tokens(json);
    }
    if !typed {
        return None;
    }
    let kind = match pg {
        "text" | "varchar" | "bpchar" | "citext" => {
            quote! { ::rqb::JsonKind::Text }
        }
        "bool" => quote! { ::rqb::JsonKind::Bool },
        "int2" | "int4" => quote! { ::rqb::JsonKind::Integer },
        "int8" => quote! { ::rqb::JsonKind::BigInt },
        "float4" | "float8" => quote! { ::rqb::JsonKind::Float },
        "numeric" => quote! { ::rqb::JsonKind::NumericString },
        "uuid" => quote! { ::rqb::JsonKind::Uuid },
        "date" => quote! { ::rqb::JsonKind::Date },
        "time" => quote! { ::rqb::JsonKind::Time },
        "timestamp" => quote! { ::rqb::JsonKind::Timestamp },
        "timestamptz" => quote! { ::rqb::JsonKind::Timestamptz },
        "jsonb" => quote! { ::rqb::JsonKind::Jsonb },
        _ => return None,
    };
    Some(kind)
}

fn schema_json_tokens(json: SchemaJson) -> Option<proc_macro2::TokenStream> {
    let kind = match json {
        SchemaJson::None => return None,
        SchemaJson::Text => quote! { ::rqb::JsonKind::Text },
        SchemaJson::Bool => quote! { ::rqb::JsonKind::Bool },
        SchemaJson::Integer => quote! { ::rqb::JsonKind::Integer },
        SchemaJson::BigInt => quote! { ::rqb::JsonKind::BigInt },
        SchemaJson::Float => quote! { ::rqb::JsonKind::Float },
        SchemaJson::NumericString => quote! { ::rqb::JsonKind::NumericString },
        SchemaJson::Uuid => quote! { ::rqb::JsonKind::Uuid },
        SchemaJson::Date => quote! { ::rqb::JsonKind::Date },
        SchemaJson::Time => quote! { ::rqb::JsonKind::Time },
        SchemaJson::Timestamp => quote! { ::rqb::JsonKind::Timestamp },
        SchemaJson::Timestamptz => quote! { ::rqb::JsonKind::Timestamptz },
        SchemaJson::Jsonb => quote! { ::rqb::JsonKind::Jsonb },
    };
    Some(kind)
}

fn expand_write_record(
    input: DeriveInput,
    kind: WriteKind,
) -> syn::Result<proc_macro2::TokenStream> {
    let attrs = parse_container_attrs(&input.attrs)?;
    let Some(table) = attrs.table else {
        return Err(Error::new_spanned(
            &input.ident,
            "missing #[rqb(table = path::to::table_module)]",
        ));
    };

    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "rqb write derives only support structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(
            &input.ident,
            "rqb write derives require named fields",
        ));
    };

    let pushes = fields
        .named
        .iter()
        .map(|field| expand_field(field, &table, kind))
        .collect::<syn::Result<Vec<_>>>()?;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let (trait_path, method) = match kind {
        WriteKind::Insertable => (
            quote! { ::rqb::Insertable },
            Ident::new("insert_assignments", Span::call_site()),
        ),
        WriteKind::Changeset => (
            quote! { ::rqb::Changeset },
            Ident::new("changeset_assignments", Span::call_site()),
        ),
    };

    Ok(quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn #method(&self) -> ::std::vec::Vec<::rqb::Assignment> {
                let mut __rqb_assignments = ::std::vec::Vec::new();
                #(#pushes)*
                __rqb_assignments
            }
        }
    })
}

fn expand_field(
    field: &Field,
    table: &Path,
    kind: WriteKind,
) -> syn::Result<proc_macro2::TokenStream> {
    let attrs = parse_field_attrs(&field.attrs)?;
    if attrs.skip {
        return Ok(quote! {});
    }

    let Some(ident) = &field.ident else {
        return Err(Error::new_spanned(
            field,
            "rqb write derives require named fields",
        ));
    };
    let field_path = field_path(table, attrs.field.as_ref(), ident);
    let value = quote_spanned! {field.span()=> &self.#ident };

    match kind {
        WriteKind::Insertable if attrs.skip_none => {
            ensure_option_field(field)?;
            Ok(quote_spanned! {field.span()=>
                if let ::std::option::Option::Some(__rqb_value) = self.#ident.as_ref() {
                    __rqb_assignments.push(#field_path.set_ref(__rqb_value));
                }
            })
        }
        WriteKind::Insertable if is_option(&field.ty) => Ok(quote_spanned! {field.span()=>
            __rqb_assignments.push(match self.#ident.as_ref() {
                ::std::option::Option::Some(__rqb_value) => #field_path.set_ref(__rqb_value),
                ::std::option::Option::None => #field_path.set_null(),
            });
        }),
        WriteKind::Insertable => Ok(quote_spanned! {field.span()=>
            __rqb_assignments.push(#field_path.set_ref(#value));
        }),
        WriteKind::Changeset => {
            if option_inner(&field.ty).is_some_and(is_option) {
                Ok(quote_spanned! {field.span()=>
                    if let ::std::option::Option::Some(__rqb_value) = self.#ident.as_ref() {
                        __rqb_assignments.push(match __rqb_value.as_ref() {
                            ::std::option::Option::Some(__rqb_value) => #field_path.set_ref(__rqb_value),
                            ::std::option::Option::None => #field_path.set_null(),
                        });
                    }
                })
            } else if is_option(&field.ty) {
                Ok(quote_spanned! {field.span()=>
                    if let ::std::option::Option::Some(__rqb_value) = self.#ident.as_ref() {
                        __rqb_assignments.push(#field_path.set_ref(__rqb_value));
                    }
                })
            } else {
                Ok(quote_spanned! {field.span()=>
                    __rqb_assignments.push(#field_path.set_ref(#value));
                })
            }
        }
    }
}

fn field_path(
    table: &Path,
    override_path: Option<&Path>,
    field_ident: &Ident,
) -> proc_macro2::TokenStream {
    match override_path {
        Some(path) if is_single_segment_path(path) => quote! { #table::#path },
        Some(path) => quote! { #path },
        None => {
            let const_ident = const_ident_for_field(field_ident);
            quote! { #table::#const_ident }
        }
    }
}

fn const_ident_for_field(field_ident: &Ident) -> Ident {
    let name = field_ident.to_string();
    let name = name.strip_prefix("r#").unwrap_or(&name);
    Ident::new(&name.to_shouty_snake_case(), field_ident.span())
}

fn parse_container_attrs(attrs: &[Attribute]) -> syn::Result<ContainerAttrs> {
    let mut parsed = ContainerAttrs::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("rqb")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("table") {
                parsed.table = Some(meta.value()?.parse()?);
                return Ok(());
            }
            Err(meta.error("unsupported rqb container attribute"))
        })?;
    }
    Ok(parsed)
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut parsed = FieldAttrs::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("rqb")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("field") {
                parsed.field = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("skip") {
                parsed.skip = parse_optional_bool(&meta)?;
                return Ok(());
            }
            if meta.path.is_ident("skip_none") {
                parsed.skip_none = parse_optional_bool(&meta)?;
                return Ok(());
            }
            Err(meta.error("unsupported rqb field attribute"))
        })?;
    }
    Ok(parsed)
}

fn parse_schema_field_attrs(attrs: &[Attribute]) -> syn::Result<SchemaFieldAttrs> {
    let mut parsed = SchemaFieldAttrs::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("rqb")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("ops") {
                parsed.ops = Some(parse_schema_ops(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("json") {
                parsed.json = Some(parse_schema_json(&meta)?);
                return Ok(());
            }
            Err(meta.error("unsupported rqb schema field attribute"))
        })?;
    }
    Ok(parsed)
}

fn parse_schema_ops(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<SchemaOps> {
    let ident: Ident = meta.value()?.call(Ident::parse_any)?;
    match ident.to_string().as_str() {
        "none" => Ok(SchemaOps::None),
        "equality" => Ok(SchemaOps::Equality),
        "ordered" => Ok(SchemaOps::Ordered),
        "text" => Ok(SchemaOps::Text),
        _ => Err(Error::new_spanned(
            ident,
            "expected one of: none, equality, ordered, text",
        )),
    }
}

fn parse_schema_json(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<SchemaJson> {
    let ident: Ident = meta.value()?.call(Ident::parse_any)?;
    match ident.to_string().as_str() {
        "none" => Ok(SchemaJson::None),
        "text" => Ok(SchemaJson::Text),
        "bool" => Ok(SchemaJson::Bool),
        "integer" => Ok(SchemaJson::Integer),
        "big_int" => Ok(SchemaJson::BigInt),
        "float" => Ok(SchemaJson::Float),
        "numeric_string" => Ok(SchemaJson::NumericString),
        "uuid" => Ok(SchemaJson::Uuid),
        "date" => Ok(SchemaJson::Date),
        "time" => Ok(SchemaJson::Time),
        "timestamp" => Ok(SchemaJson::Timestamp),
        "timestamptz" => Ok(SchemaJson::Timestamptz),
        "jsonb" => Ok(SchemaJson::Jsonb),
        _ => Err(Error::new_spanned(
            ident,
            "expected one of: none, text, bool, integer, big_int, float, numeric_string, uuid, date, time, timestamp, timestamptz, jsonb",
        )),
    }
}

fn parse_optional_bool(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<bool> {
    if meta.input.peek(syn::Token![=]) {
        return Ok(meta.value()?.parse::<LitBool>()?.value);
    }
    Ok(true)
}

fn ensure_option_field(field: &Field) -> syn::Result<()> {
    if is_option(&field.ty) {
        Ok(())
    } else {
        Err(Error::new_spanned(
            &field.ty,
            "#[rqb(skip_none)] can only be used on Option<T> fields",
        ))
    }
}

fn is_option(ty: &Type) -> bool {
    option_inner(ty).is_some()
}

fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(ty) if args.args.len() == 1 => Some(ty),
        _ => None,
    }
}

fn is_single_segment_path(path: &Path) -> bool {
    path.leading_colon.is_none() && path.segments.len() == 1
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

fn unique_ident(base: String, span: Span, used: &mut HashSet<String>) -> Ident {
    let mut name = base.clone();
    let mut suffix = 2;
    while !used.insert(name.clone()) {
        name = format!("{}_{suffix}", base.trim_end_matches('_'));
        suffix += 1;
    }
    Ident::new(&name, span)
}

fn sanitize_alias_method_ident(value: &str) -> String {
    let ident = sanitize_ident(value);
    if matches!(
        ident.as_str(),
        "as_ref" | "clone" | "eq" | "from" | "hash" | "into" | "ne" | "source"
    ) {
        format!("{ident}_")
    } else {
        ident
    }
}

fn is_rust_keyword(value: &str) -> bool {
    // This mostly matters for generated module names. Column const names are
    // SHOUTY_SNAKE, but quoted column names can still pass through the same
    // sanitizer before case conversion.
    matches!(
        value,
        "abstract"
            | "alignof"
            | "as"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "gen"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "offsetof"
            | "override"
            | "priv"
            | "proc"
            | "pure"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "sizeof"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "yield"
    )
}

#[cfg(test)]
mod tests {
    use super::SchemaInput;

    #[test]
    fn schema_accepts_cli_ops_and_json_contract_tokens() {
        let input = r#"
            table public.contract_tokens {
                #[rqb(ops = none, json = none)]
                none_token: text = String,
                #[rqb(ops = equality, json = text)]
                text_token: text = String,
                #[rqb(ops = ordered, json = bool)]
                bool_token: bool = bool,
                #[rqb(ops = text, json = integer)]
                integer_token: int4 = i32,
                #[rqb(ops = ordered, json = big_int)]
                big_int_token: int8 = i64,
                #[rqb(ops = ordered, json = float)]
                float_token: float8 = f64,
                #[rqb(ops = ordered, json = numeric_string)]
                numeric_string_token: numeric = sqlx::types::BigDecimal,
                #[rqb(ops = ordered, json = uuid)]
                uuid_token: uuid = uuid::Uuid,
                #[rqb(ops = ordered, json = date)]
                date_token: date = chrono::NaiveDate,
                #[rqb(ops = ordered, json = time)]
                time_token: time = chrono::NaiveTime,
                #[rqb(ops = ordered, json = timestamp)]
                timestamp_token: timestamp = chrono::NaiveDateTime,
                #[rqb(ops = ordered, json = timestamptz)]
                timestamptz_token: timestamptz = chrono::DateTime<chrono::Utc>,
                #[rqb(ops = equality, json = jsonb)]
                jsonb_token: jsonb = serde_json::Value,
            }
        "#;

        let schema = syn::parse_str::<SchemaInput>(input).unwrap();
        let expanded = schema.expand().to_string();

        assert!(expanded.contains("contract_tokens"));
    }

    #[test]
    fn schema_reports_field_constant_conflicts_with_helpers() {
        let err = syn::parse_str::<SchemaInput>("table public.t { fields: int4 = i32 }")
            .err()
            .unwrap();
        assert!(
            err.to_string()
                .contains("choose another name with `as FIELD_NAME`")
        );
        assert!(
            syn::parse_str::<SchemaInput>("table public.t { fields as FIELDS_1: int4 = i32 }")
                .is_ok()
        );
    }

    #[test]
    fn schema_rejects_unsupported_array_json_override() {
        let err = syn::parse_str::<SchemaInput>(
            r#"table public.t {
            #[rqb(json = text)] tags: "text[]" = Vec<String>,
        }"#,
        )
        .err()
        .unwrap();
        assert!(err.to_string().contains("array fields do not support JSON"));
    }
}
