use heck::ToShoutySnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, GenericArgument, LitBool, Path,
    PathArguments, Type, parse_macro_input,
};

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
