//! Derive macros for rqb.

use heck::ToShoutySnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Field as SynField, Fields, Ident, Path, Result,
    parse_macro_input,
};

#[proc_macro_derive(WriteRecord, attributes(rqb))]
/// Derives `rqb::WriteRecord` for insert and update DTOs.
pub fn derive_write_record(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_write_record(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_write_record(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let container = ContainerAttrs::parse(&input.attrs)?;
    let skip_none_mode = container.skip_none;
    let fields_path = container.fields.ok_or_else(|| {
        Error::new_spanned(&input.ident, "missing #[rqb(fields = path::to::fields)]")
    })?;
    let crate_path = container
        .crate_path
        .unwrap_or_else(|| syn::parse_quote!(::rqb));

    let named_fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return Err(Error::new_spanned(
                    input.ident,
                    "WriteRecord can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                input.ident,
                "WriteRecord can only be derived for structs",
            ));
        }
    };

    let mut pushes = Vec::new();
    for field in named_fields {
        let Some(ident) = field.ident.as_ref() else {
            continue;
        };
        let mut attrs = FieldAttrs::parse(&field)?;
        let is_option = is_option_type(&field.ty);
        if skip_none_mode && is_option {
            attrs.skip_none = true;
        }
        if attrs.skip_none && !is_option {
            return Err(Error::new_spanned(
                &field.ty,
                "#[rqb(skip_none)] can only be used on Option<T> fields",
            ));
        }
        if attrs.skip {
            continue;
        }

        let field_expr = attrs
            .field
            .clone()
            .unwrap_or_else(|| default_field_path(&fields_path, ident));
        pushes.push(field_push(
            &crate_path,
            ident,
            &field_expr,
            &attrs,
            is_option,
        ));
    }

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #crate_path::WriteRecord for #name #ty_generics #where_clause {
            fn write_fields(
                &self,
            ) -> #crate_path::__RqbWriteRecordResult<::std::vec::Vec<(#crate_path::Field, #crate_path::Value)>> {
                let mut fields = ::std::vec::Vec::new();
                #(#pushes)*
                ::std::result::Result::Ok(fields)
            }
        }
    })
}

#[derive(Default)]
struct ContainerAttrs {
    fields: Option<Path>,
    crate_path: Option<Path>,
    skip_none: bool,
}

impl ContainerAttrs {
    fn parse(attrs: &[Attribute]) -> Result<Self> {
        let mut parsed = Self::default();
        for attr in rqb_attrs(attrs) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("fields") {
                    parsed.fields = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("crate") {
                    parsed.crate_path = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("skip_none") {
                    parsed.skip_none = true;
                    Ok(())
                } else {
                    Err(meta.error("unsupported rqb container attribute"))
                }
            })?;
        }
        Ok(parsed)
    }
}

#[derive(Default)]
struct FieldAttrs {
    field: Option<Path>,
    skip: bool,
    skip_none: bool,
    json: bool,
    bytes: bool,
}

impl FieldAttrs {
    fn parse(field: &SynField) -> Result<Self> {
        let mut parsed = Self::default();
        for attr in rqb_attrs(&field.attrs) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("field") {
                    parsed.field = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("skip") {
                    parsed.skip = true;
                    Ok(())
                } else if meta.path.is_ident("skip_none") {
                    parsed.skip_none = true;
                    Ok(())
                } else if meta.path.is_ident("json") {
                    parsed.json = true;
                    Ok(())
                } else if meta.path.is_ident("bytes") {
                    parsed.bytes = true;
                    Ok(())
                } else {
                    Err(meta.error("unsupported rqb field attribute"))
                }
            })?;
        }
        Ok(parsed)
    }
}

fn rqb_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attr| attr.path().is_ident("rqb"))
}

fn default_field_path(fields_path: &Path, ident: &Ident) -> Path {
    let const_ident = format_ident!("{}", ident.to_string().to_shouty_snake_case());
    syn::parse_quote!(#fields_path::#const_ident)
}

fn is_option_type(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Option")
}

fn field_push(
    crate_path: &Path,
    ident: &Ident,
    field_expr: &Path,
    attrs: &FieldAttrs,
    is_option: bool,
) -> proc_macro2::TokenStream {
    let value = if attrs.json {
        quote!(#crate_path::__rqb_json_write_value(&self.#ident)?)
    } else if attrs.bytes {
        quote!(#crate_path::Value::bytes(::std::clone::Clone::clone(&self.#ident)))
    } else {
        quote!(#crate_path::Value::from(::std::clone::Clone::clone(&self.#ident)))
    };

    if attrs.skip_none {
        let value = if attrs.json {
            quote!(#crate_path::__rqb_json_write_value(value)?)
        } else if attrs.bytes {
            quote!(#crate_path::Value::bytes(::std::clone::Clone::clone(value)))
        } else {
            quote!(#crate_path::Value::from(::std::clone::Clone::clone(value)))
        };

        quote! {
            if let ::std::option::Option::Some(value) = self.#ident.as_ref() {
                fields.push((#field_expr, #value));
            }
        }
    } else if is_option && attrs.json {
        quote! {
            match self.#ident.as_ref() {
                ::std::option::Option::Some(value) => {
                    fields.push((#field_expr, #crate_path::__rqb_json_write_value(value)?));
                }
                ::std::option::Option::None => {
                    fields.push((#field_expr, #crate_path::Value::Null));
                }
            }
        }
    } else if is_option && attrs.bytes {
        quote! {
            match self.#ident.as_ref() {
                ::std::option::Option::Some(value) => {
                    fields.push((#field_expr, #crate_path::Value::bytes(::std::clone::Clone::clone(value))));
                }
                ::std::option::Option::None => {
                    fields.push((#field_expr, #crate_path::Value::Null));
                }
            }
        }
    } else {
        quote! {
            fields.push((#field_expr, #value));
        }
    }
}
