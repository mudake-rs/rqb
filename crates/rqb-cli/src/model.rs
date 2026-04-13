use rqb_core::{FieldType, SelectRepr, TypeFamily, ValueRepr};

#[derive(Debug, Clone)]
pub(crate) struct Relation {
    pub(crate) name: String,
    pub(crate) kind: RelationKind,
    pub(crate) columns: Vec<Column>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RelationKind {
    Table,
    View,
}

#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) name: String,
    pub(crate) api_name: String,
    pub(crate) rust_name: String,
    pub(crate) const_name: String,
    pub(crate) field_type: ColumnType,
}

#[derive(Debug, Clone)]
pub(crate) enum ColumnType {
    Core(FieldType),
    Enum(PgEnum),
    ArrayEnum(PgEnum),
    Domain(PgDomain),
    ArrayDomain(PgDomain),
}

impl ColumnType {
    pub(crate) fn is_jsonb(&self) -> bool {
        matches!(self, Self::Core(FieldType::Jsonb))
            || matches!(self, Self::Domain(domain) if domain.family == TypeFamily::Jsonb)
    }

    pub(crate) fn is_array(&self) -> bool {
        matches!(self, Self::Core(field_type) if field_type.is_array())
            || matches!(self, Self::ArrayEnum(_) | Self::ArrayDomain(_))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PgEnum {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) const_name: String,
    pub(crate) rust_name: String,
    pub(crate) variants: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PgDomain {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) const_name: String,
    pub(crate) family: TypeFamily,
    pub(crate) value_repr: ValueRepr,
    pub(crate) select_repr: SelectRepr,
}
