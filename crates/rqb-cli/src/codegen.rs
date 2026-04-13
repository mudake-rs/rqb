use std::collections::BTreeMap;

use anyhow::{Context, Result};
use heck::ToSnakeCase;
use proc_macro2::{Ident, Span};
use quote::quote;
use rqb_core::{ElemType, FieldType, SelectRepr, TypeFamily, ValueRepr};

use crate::ident::{sanitize_ident, unique_enum_variant_idents};
use crate::model::{Column, ColumnType, PgDomain, PgEnum, Relation, RelationKind};

pub(crate) fn render(
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
        ColumnType::ArrayDomain(domain) => {
            let const_name = Ident::new(&domain.const_name, Span::call_site());
            quote! { FieldType::Array(ElemType::Custom(&super::types::#const_name)) }
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
        ElemType::Custom(_) => {
            unreachable!("custom element types are rendered from ColumnType::ArrayDomain")
        }
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rqb_core::{FieldType, SelectRepr, TypeFamily, ValueRepr};

    use crate::model::{Column, ColumnType, PgDomain, PgEnum, Relation, RelationKind};

    use super::render;

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
                columns: vec![
                    Column {
                        name: "amount".to_owned(),
                        api_name: "amount".to_owned(),
                        rust_name: "amount".to_owned(),
                        const_name: "AMOUNT".to_owned(),
                        field_type: ColumnType::Domain(domain.clone()),
                    },
                    Column {
                        name: "amount_history".to_owned(),
                        api_name: "amountHistory".to_owned(),
                        rust_name: "amount_history".to_owned(),
                        const_name: "AMOUNT_HISTORY".to_owned(),
                        field_type: ColumnType::ArrayDomain(domain),
                    },
                ],
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
        assert!(code.contains("FieldType::Array(ElemType::Custom(&super::types::UINT_256))"));
        assert!(code.contains(".sortable(false)"));
    }
}
