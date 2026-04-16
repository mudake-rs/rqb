use heck::ToShoutySnakeCase;
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, Span};
use quote::{quote, quote_spanned};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, GenericArgument, LitBool, LitStr, Path,
    PathArguments, Result, Token, Type, braced, parse_macro_input,
};

/// Generates compact rqb schema modules.
///
/// Grammar:
///
/// ```text
/// (table|view) <schema>.<relation> [as <module>] {
///     <db_column> [as <CONST>]: <pg_type> [= <rust_type>],
///     ...
/// }
/// ```
///
/// A column with `= <rust_type>` emits both metadata and a typed
/// `pub const NAME: Field<T>`. A column without a Rust type emits metadata
/// only, which is useful for extension or user-defined PostgreSQL types that
/// should remain raw-only.
///
/// Generated metadata intentionally uses database column names as public API
/// names. HTTP/JSON casing belongs in application DTOs, not generated schema.
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

#[proc_macro_derive(Insertable, attributes(rqb))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_write_record(
        parse_macro_input!(input as DeriveInput),
        WriteKind::Insertable,
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

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
}

struct ColumnInput {
    db: String,
    const_ident: Ident,
    pg: String,
    rust_ty: Option<Type>,
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

mod kw {
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
        while !content.is_empty() {
            columns.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            kind,
            qualified_name: name.qualified,
            module,
            columns,
        })
    }
}

impl RelationInput {
    fn expand(self) -> proc_macro2::TokenStream {
        let module = self.module;
        let qualified_name = self.qualified_name;
        let constructor = match self.kind {
            RelationKind::Table => quote! { ::rqb::table(#qualified_name, &FIELDS) },
            RelationKind::View => quote! { ::rqb::view(#qualified_name, &FIELDS) },
        };
        let constructor_fn = match self.kind {
            RelationKind::Table => quote! {
                pub fn table() -> ::rqb::Source {
                    #constructor
                }
            },
            RelationKind::View => quote! {
                pub fn view() -> ::rqb::Source {
                    #constructor
                }
            },
        };

        let columns = self
            .columns
            .into_iter()
            .map(ColumnInput::expand)
            .collect::<Vec<_>>();
        let metas = columns.iter().map(|column| &column.meta);
        let fields = columns.iter().filter_map(|column| column.field.as_ref());
        let alias_methods = columns
            .iter()
            .filter_map(|column| column.alias_method.as_ref());
        let meta_idents = columns.iter().map(|column| &column.meta_ident);
        let field_count = Literal::usize_unsuffixed(columns.len());

        quote! {
            pub mod #module {
                // Bring caller-scope Rust types into the generated module.
                #[allow(unused_imports)]
                use super::*;

                #(#metas)*
                #(#fields)*

                pub static FIELDS: [&'static ::rqb::Meta; #field_count] = [#(&#meta_idents),*];

                #constructor_fn

                pub fn alias(alias: impl Into<String>) -> Alias {
                    Alias {
                        alias: alias.into(),
                    }
                }

                #[derive(Clone, Debug)]
                pub struct Alias {
                    alias: String,
                }

                impl Alias {
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
        let rust_ty = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            db,
            const_ident,
            pg,
            rust_ty,
        })
    }
}

impl ColumnInput {
    fn expand(self) -> ExpandedColumn {
        let meta_ident = Ident::new(
            &format!("{}_META", self.const_ident),
            self.const_ident.span(),
        );
        let db = self.db;
        let pg = self.pg;
        let ops = ops_tokens(&pg, self.rust_ty.is_some());
        let json = json_kind_tokens(&pg, self.rust_ty.is_some());
        let const_ident = self.const_ident;
        let method_ident = Ident::new(
            &sanitize_alias_method_ident(&const_ident.to_string().to_snake_case()),
            const_ident.span(),
        );

        let mut meta_expr = quote! { ::rqb::Meta::col(#db, #pg).ops(#ops) };
        if let Some(json) = json {
            meta_expr = quote! { #meta_expr.json(#json) };
        }
        let meta = quote! {
            pub static #meta_ident: ::rqb::Meta = #meta_expr;
        };

        let (field, alias_method) = match self.rust_ty {
            Some(rust_ty) => {
                let field = quote! {
                    pub const #const_ident: ::rqb::Field<#rust_ty> = ::rqb::Field::new(&#meta_ident);
                };
                let alias_method = quote! {
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

fn ops_tokens(pg: &str, typed: bool) -> proc_macro2::TokenStream {
    if !typed {
        return quote! { ::rqb::OpSet::none() };
    }
    if is_equality_only_pg(pg) {
        quote! { ::rqb::OpSet::equality() }
    } else {
        quote! { ::rqb::OpSet::ordered() }
    }
}

fn is_equality_only_pg(pg: &str) -> bool {
    matches!(
        pg,
        "bool"
            | "json"
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

fn json_kind_tokens(pg: &str, typed: bool) -> Option<proc_macro2::TokenStream> {
    if !typed || pg.ends_with("[]") {
        return None;
    }
    let kind = match pg {
        "text" | "varchar" | "bpchar" | "citext" | "inet" | "cidr" => {
            quote! { ::rqb::JsonKind::Text }
        }
        "bool" => quote! { ::rqb::JsonKind::Bool },
        "int2" | "int4" => quote! { ::rqb::JsonKind::Integer },
        "int8" => quote! { ::rqb::JsonKind::BigInt },
        "float4" | "float8" => quote! { ::rqb::JsonKind::Float },
        "numeric" => quote! { ::rqb::JsonKind::NumericString },
        "uuid" => quote! { ::rqb::JsonKind::Uuid },
        "date" => quote! { ::rqb::JsonKind::Date },
        "time" | "timetz" => quote! { ::rqb::JsonKind::Time },
        "timestamp" => quote! { ::rqb::JsonKind::Timestamp },
        "timestamptz" => quote! { ::rqb::JsonKind::Timestamptz },
        "json" | "jsonb" => quote! { ::rqb::JsonKind::Jsonb },
        _ => return None,
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
        WriteKind::Insertable => Ok(quote_spanned! {field.span()=>
            __rqb_assignments.push(#field_path.set_ref(#value));
        }),
        WriteKind::Changeset => {
            if is_option(&field.ty) {
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
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Option" && has_one_angle_arg(&segment.arguments))
}

fn has_one_angle_arg(args: &PathArguments) -> bool {
    match args {
        PathArguments::AngleBracketed(args) => {
            args.args
                .iter()
                .filter(|arg| matches!(arg, GenericArgument::Type(_)))
                .count()
                == 1
        }
        PathArguments::None | PathArguments::Parenthesized(_) => false,
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

fn sanitize_alias_method_ident(value: &str) -> String {
    let ident = sanitize_ident(value);
    if matches!(ident.as_str(), "clone" | "source") {
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
