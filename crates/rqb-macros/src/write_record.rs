use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Path, Result};

use crate::field::FieldModel;
use crate::model::Model;

pub(crate) fn expand(input: DeriveInput) -> Result<TokenStream> {
    let model = Model::parse(input)?;
    let fields_path = model.attrs.fields.clone().ok_or_else(|| {
        Error::new_spanned(&model.ident, "missing #[rqb(fields = path::to::fields)]")
    })?;
    let crate_path = model
        .attrs
        .crate_path
        .clone()
        .unwrap_or_else(|| syn::parse_quote!(::rqb));

    let mut pushes = Vec::new();
    for field in &model.fields {
        if let Some(field) = write_field(field, &fields_path, model.attrs.skip_none)? {
            pushes.push(field_push(&crate_path, &field));
        }
    }

    let name = &model.ident;
    let (impl_generics, ty_generics, where_clause) = model.generics.split_for_impl();

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

struct WriteField {
    ident: syn::Ident,
    field_expr: Path,
    value_kind: WriteValueKind,
    optional_policy: OptionalPolicy,
}

#[derive(Clone, Copy)]
enum WriteValueKind {
    Normal,
    Json,
    Bytes,
}

#[derive(Clone, Copy)]
enum OptionalPolicy {
    AlwaysWrite,
    NullOnNone,
    SkipNone,
}

fn write_field(
    field: &FieldModel,
    fields_path: &Path,
    container_skip_none: bool,
) -> Result<Option<WriteField>> {
    if field.attrs.skip {
        return Ok(None);
    }

    let skip_none = field.attrs.skip_none || (container_skip_none && field.is_option);
    let optional_policy = if skip_none {
        if !field.is_option {
            return Err(Error::new_spanned(
                &field.ty,
                "#[rqb(skip_none)] can only be used on Option<T> fields",
            ));
        }
        OptionalPolicy::SkipNone
    } else if field.is_option && (field.attrs.json || field.attrs.bytes) {
        OptionalPolicy::NullOnNone
    } else {
        OptionalPolicy::AlwaysWrite
    };

    let value_kind = if field.attrs.json {
        WriteValueKind::Json
    } else if field.attrs.bytes {
        WriteValueKind::Bytes
    } else {
        WriteValueKind::Normal
    };

    Ok(Some(WriteField {
        ident: field.ident.clone(),
        field_expr: field.field_path(fields_path),
        value_kind,
        optional_policy,
    }))
}

fn field_push(crate_path: &Path, field: &WriteField) -> TokenStream {
    match field.optional_policy {
        OptionalPolicy::SkipNone => render_skip_none(crate_path, field),
        OptionalPolicy::NullOnNone => render_null_on_none(crate_path, field),
        OptionalPolicy::AlwaysWrite => render_always_write(crate_path, field),
    }
}

fn render_skip_none(crate_path: &Path, field: &WriteField) -> TokenStream {
    let ident = &field.ident;
    let field_expr = &field.field_expr;
    let value = value_from_ref(crate_path, field.value_kind);

    quote! {
        if let ::std::option::Option::Some(value) = self.#ident.as_ref() {
            fields.push((#field_expr, #value));
        }
    }
}

fn render_null_on_none(crate_path: &Path, field: &WriteField) -> TokenStream {
    let ident = &field.ident;
    let field_expr = &field.field_expr;
    let value = value_from_ref(crate_path, field.value_kind);

    quote! {
        match self.#ident.as_ref() {
            ::std::option::Option::Some(value) => {
                fields.push((#field_expr, #value));
            }
            ::std::option::Option::None => {
                fields.push((#field_expr, #crate_path::Value::Null));
            }
        }
    }
}

fn render_always_write(crate_path: &Path, field: &WriteField) -> TokenStream {
    let ident = &field.ident;
    let field_expr = &field.field_expr;
    let value = value_from_field(crate_path, field.value_kind, ident);

    quote! {
        fields.push((#field_expr, #value));
    }
}

fn value_from_field(crate_path: &Path, kind: WriteValueKind, ident: &syn::Ident) -> TokenStream {
    match kind {
        WriteValueKind::Json => quote!(#crate_path::__rqb_json_write_value(&self.#ident)?),
        WriteValueKind::Bytes => {
            quote!(#crate_path::Value::bytes(::std::clone::Clone::clone(&self.#ident)))
        }
        WriteValueKind::Normal => {
            quote!(#crate_path::Value::from(::std::clone::Clone::clone(&self.#ident)))
        }
    }
}

fn value_from_ref(crate_path: &Path, kind: WriteValueKind) -> TokenStream {
    match kind {
        WriteValueKind::Json => quote!(#crate_path::__rqb_json_write_value(value)?),
        WriteValueKind::Bytes => {
            quote!(#crate_path::Value::bytes(::std::clone::Clone::clone(value)))
        }
        WriteValueKind::Normal => {
            quote!(#crate_path::Value::from(::std::clone::Clone::clone(value)))
        }
    }
}
