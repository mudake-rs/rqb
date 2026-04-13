use std::collections::BTreeMap;

use rqb_core::{ElemType, FieldType, TypeFamily, ValueRepr};

use crate::model::{ColumnType, PgDomain, PgEnum};

pub(crate) fn map_field_type(
    data_type: &str,
    udt_name: &str,
    domain_schema: Option<&str>,
    domain_name: Option<&str>,
    enums: &BTreeMap<String, PgEnum>,
    domains: &BTreeMap<String, PgDomain>,
) -> ColumnType {
    if let Some(domain_name) = domain_name
        && let Some(domain) = domains.get(domain_name)
        && domain_schema.is_none_or(|schema| schema == domain.schema)
    {
        return ColumnType::Domain(domain.clone());
    }

    if data_type == "USER-DEFINED"
        && let Some(pg_enum) = enums.get(udt_name)
    {
        return ColumnType::Enum(pg_enum.clone());
    }
    if data_type == "ARRAY"
        && let Some(enum_name) = udt_name.strip_prefix('_')
        && let Some(pg_enum) = enums.get(enum_name)
    {
        return ColumnType::ArrayEnum(pg_enum.clone());
    }
    if data_type == "ARRAY"
        && let Some(domain_name) = udt_name.strip_prefix('_')
        && let Some(domain) = domains.get(domain_name)
    {
        return ColumnType::ArrayDomain(domain.clone());
    }

    ColumnType::Core(match (data_type, udt_name) {
        ("ARRAY", "_text" | "_varchar") => FieldType::Array(ElemType::Text),
        ("ARRAY", "_citext") => FieldType::Array(ElemType::Citext),
        ("ARRAY", "_int2" | "_int4") => FieldType::Array(ElemType::Int),
        ("ARRAY", "_int8") => FieldType::Array(ElemType::BigInt),
        ("ARRAY", "_float4" | "_float8") => FieldType::Array(ElemType::Float),
        ("ARRAY", "_numeric") => FieldType::Array(ElemType::Numeric),
        ("ARRAY", "_bool") => FieldType::Array(ElemType::Bool),
        ("ARRAY", "_uuid") => FieldType::Array(ElemType::Uuid),
        ("ARRAY", "_timestamp") => FieldType::Array(ElemType::Timestamp),
        ("ARRAY", "_timestamptz") => FieldType::Array(ElemType::Timestamptz),
        ("ARRAY", "_date") => FieldType::Array(ElemType::Date),
        (_, "int4range") => FieldType::Range(ElemType::Int),
        (_, "int8range") => FieldType::Range(ElemType::BigInt),
        (_, "numrange") => FieldType::Range(ElemType::Numeric),
        (_, "tsrange") => FieldType::Range(ElemType::Timestamp),
        (_, "tstzrange") => FieldType::Range(ElemType::Timestamptz),
        (_, "daterange") => FieldType::Range(ElemType::Date),
        (_, "uuid") => FieldType::Uuid,
        (_, "bool") => FieldType::Bool,
        (_, "bytea") => FieldType::Bytea,
        (_, "citext") => FieldType::Citext,
        (_, "inet") => FieldType::Inet,
        (_, "cidr") => FieldType::Cidr,
        (_, "int2" | "int4") => FieldType::Integer,
        (_, "int8") => FieldType::BigInt,
        (_, "float4" | "float8") => FieldType::Float,
        (_, "numeric") => FieldType::Numeric,
        (_, "date") => FieldType::Date,
        (_, "timestamp") => FieldType::Timestamp,
        (_, "timestamptz") => FieldType::Timestamptz,
        (_, "json" | "jsonb") => FieldType::Jsonb,
        _ => FieldType::Text,
    })
}

pub(crate) fn type_family_for_udt(udt_name: &str) -> TypeFamily {
    match udt_name {
        "bool" => TypeFamily::Bool,
        "int2" | "int4" | "int8" | "float4" | "float8" | "numeric" => TypeFamily::Numeric,
        "uuid" => TypeFamily::Uuid,
        "date" => TypeFamily::Date,
        "timestamp" => TypeFamily::Timestamp,
        "timestamptz" => TypeFamily::Timestamptz,
        "json" | "jsonb" => TypeFamily::Jsonb,
        "bytea" => TypeFamily::Bytes,
        "inet" | "cidr" => TypeFamily::Network,
        "int4range" | "int8range" | "numrange" | "tsrange" | "tstzrange" | "daterange" => {
            TypeFamily::Range
        }
        _ => TypeFamily::Text,
    }
}

pub(crate) fn value_repr_for_family(family: TypeFamily) -> ValueRepr {
    match family {
        TypeFamily::Numeric => ValueRepr::DecimalString,
        TypeFamily::Bool | TypeFamily::Jsonb | TypeFamily::Bytes => ValueRepr::Native,
        TypeFamily::Text
        | TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Network
        | TypeFamily::Range => ValueRepr::String,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rqb_core::{ElemType, FieldType, SelectRepr, TypeFamily, ValueRepr};

    use crate::model::{ColumnType, PgDomain};

    use super::map_field_type;

    #[test]
    fn maps_postgres_types() {
        let uint_256 = PgDomain {
            schema: "public".to_owned(),
            name: "uint_256".to_owned(),
            const_name: "UINT_256".to_owned(),
            family: TypeFamily::Numeric,
            value_repr: ValueRepr::DecimalString,
            select_repr: SelectRepr::Text,
        };
        let mut domains = BTreeMap::new();
        domains.insert(uint_256.name.clone(), uint_256);

        for (udt_name, expected) in [
            ("_text", ElemType::Text),
            ("_varchar", ElemType::Text),
            ("_citext", ElemType::Citext),
            ("_int2", ElemType::Int),
            ("_int4", ElemType::Int),
            ("_int8", ElemType::BigInt),
            ("_float4", ElemType::Float),
            ("_float8", ElemType::Float),
            ("_numeric", ElemType::Numeric),
            ("_bool", ElemType::Bool),
            ("_uuid", ElemType::Uuid),
            ("_timestamp", ElemType::Timestamp),
            ("_timestamptz", ElemType::Timestamptz),
            ("_date", ElemType::Date),
        ] {
            assert!(
                matches!(
                    map_field_type(
                        "ARRAY",
                        udt_name,
                        None,
                        None,
                        &BTreeMap::new(),
                        &BTreeMap::new()
                    ),
                    ColumnType::Core(FieldType::Array(actual)) if actual == expected
                ),
                "{udt_name} should map to {expected:?}"
            );
        }
        assert!(matches!(
            map_field_type(
                "USER-DEFINED",
                "uuid",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Uuid)
        ));
        assert!(matches!(
            map_field_type(
                "jsonb",
                "jsonb",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Jsonb)
        ));
        assert!(matches!(
            map_field_type(
                "timestamp with time zone",
                "timestamptz",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Timestamptz)
        ));
        for (udt_name, expected) in [
            ("bytea", FieldType::Bytea),
            ("citext", FieldType::Citext),
            ("inet", FieldType::Inet),
            ("cidr", FieldType::Cidr),
            ("int4range", FieldType::Range(ElemType::Int)),
            ("tstzrange", FieldType::Range(ElemType::Timestamptz)),
        ] {
            assert!(
                matches!(
                    map_field_type(
                        "USER-DEFINED",
                        udt_name,
                        None,
                        None,
                        &BTreeMap::new(),
                        &BTreeMap::new()
                    ),
                    ColumnType::Core(actual) if actual == expected
                ),
                "{udt_name} should map to {expected:?}"
            );
        }
        assert!(matches!(
            map_field_type(
                "unknown",
                "ltree",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            ),
            ColumnType::Core(FieldType::Text)
        ));
        assert!(matches!(
            map_field_type(
                "numeric",
                "numeric",
                Some("public"),
                Some("uint_256"),
                &BTreeMap::new(),
                &domains
            ),
            ColumnType::Domain(domain)
                if domain.name == "uint_256"
                    && domain.family == TypeFamily::Numeric
                    && domain.value_repr == ValueRepr::DecimalString
        ));
        assert!(matches!(
            map_field_type(
                "ARRAY",
                "_uint_256",
                None,
                None,
                &BTreeMap::new(),
                &domains
            ),
            ColumnType::ArrayDomain(domain)
                if domain.name == "uint_256"
                    && domain.family == TypeFamily::Numeric
                    && domain.value_repr == ValueRepr::DecimalString
        ));
    }
}
