use heck::ToShoutySnakeCase;
use quote::format_ident;
use syn::{Error, Field as SynField, Ident, Path, Result, Type};

use crate::attrs::FieldAttrs;

pub(crate) struct FieldModel {
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
    pub(crate) attrs: FieldAttrs,
    pub(crate) is_option: bool,
}

impl FieldModel {
    pub(crate) fn parse(field: SynField) -> Result<Self> {
        let ident = field
            .ident
            .ok_or_else(|| Error::new_spanned(&field.ty, "expected a named field"))?;
        let attrs = FieldAttrs::parse(&field.attrs)?;
        let is_option = is_option_type(&field.ty);
        Ok(Self {
            ident,
            ty: field.ty,
            attrs,
            is_option,
        })
    }

    pub(crate) fn field_path(&self, fields_path: &Path) -> Path {
        self.attrs
            .field
            .clone()
            .unwrap_or_else(|| default_field_path(fields_path, &self.ident))
    }
}

fn default_field_path(fields_path: &Path, ident: &Ident) -> Path {
    let const_ident = format_ident!("{}", ident.to_string().to_shouty_snake_case());
    syn::parse_quote!(#fields_path::#const_ident)
}

fn is_option_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Option")
}
