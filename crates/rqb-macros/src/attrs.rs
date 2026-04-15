use syn::{Attribute, Path, Result};

#[derive(Default)]
pub(crate) struct ContainerAttrs {
    pub(crate) fields: Option<Path>,
    pub(crate) crate_path: Option<Path>,
    pub(crate) skip_none: bool,
}

impl ContainerAttrs {
    pub(crate) fn parse(attrs: &[Attribute]) -> Result<Self> {
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

#[derive(Clone, Default)]
pub(crate) struct FieldAttrs {
    pub(crate) field: Option<Path>,
    pub(crate) skip: bool,
    pub(crate) skip_none: bool,
    pub(crate) json: bool,
    pub(crate) bytes: bool,
}

impl FieldAttrs {
    pub(crate) fn parse(attrs: &[Attribute]) -> Result<Self> {
        let mut parsed = Self::default();
        for attr in rqb_attrs(attrs) {
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
