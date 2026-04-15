use syn::{Data, DeriveInput, Error, Fields, Generics, Ident, Result};

use crate::attrs::ContainerAttrs;
use crate::field::FieldModel;

pub(crate) struct Model {
    pub(crate) ident: Ident,
    pub(crate) generics: Generics,
    pub(crate) attrs: ContainerAttrs,
    pub(crate) fields: Vec<FieldModel>,
}

impl Model {
    pub(crate) fn parse(input: DeriveInput) -> Result<Self> {
        let attrs = ContainerAttrs::parse(&input.attrs)?;
        let fields = match input.data {
            Data::Struct(data) => match data.fields {
                Fields::Named(fields) => fields
                    .named
                    .into_iter()
                    .map(FieldModel::parse)
                    .collect::<Result<Vec<_>>>()?,
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

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            attrs,
            fields,
        })
    }
}
