use std::collections::BTreeMap;

use anyhow::{Result, bail};
use heck::ToShoutySnakeCase;
use rqb_core::{ElemType, FieldType, TypeFamily, ValueRepr};

use crate::ident::sanitize_ident;
use crate::model::{ColumnType, PgDomain, PgDomainSource, PgEnum, SchemaTypeKey};

pub(crate) fn map_field_type(
    data_type: &str,
    udt_schema: &str,
    udt_name: &str,
    domain_schema: Option<&str>,
    domain_name: Option<&str>,
    enums: &BTreeMap<SchemaTypeKey, PgEnum>,
    domains: &BTreeMap<SchemaTypeKey, PgDomainSource>,
) -> Result<ColumnType> {
    if let Some(domain_name) = domain_name {
        if let Some(domain_schema) = domain_schema
            && let Some(source) = domains.get(&schema_type_key(domain_schema, domain_name))
        {
            return Ok(ColumnType::Domain(materialize_domain(source)?));
        }

        bail!(
            "unsupported Postgres domain `{}`{}",
            domain_schema
                .map(|schema| format!("{schema}."))
                .unwrap_or_default(),
            domain_name,
        );
    }

    map_non_domain_field_type(data_type, udt_schema, udt_name, enums, domains)
}

fn map_non_domain_field_type(
    data_type: &str,
    udt_schema: &str,
    udt_name: &str,
    enums: &BTreeMap<SchemaTypeKey, PgEnum>,
    domains: &BTreeMap<SchemaTypeKey, PgDomainSource>,
) -> Result<ColumnType> {
    if data_type == "USER-DEFINED"
        && let Some(pg_enum) = enums.get(&schema_type_key(udt_schema, udt_name))
    {
        return Ok(ColumnType::Enum(pg_enum.clone()));
    }
    if data_type == "ARRAY"
        && let Some(enum_name) = udt_name.strip_prefix('_')
        && let Some(pg_enum) = enums.get(&schema_type_key(udt_schema, enum_name))
    {
        return Ok(ColumnType::ArrayEnum(pg_enum.clone()));
    }
    if data_type == "ARRAY"
        && let Some(domain_name) = udt_name.strip_prefix('_')
        && let Some(source) = domains.get(&schema_type_key(udt_schema, domain_name))
    {
        return Ok(ColumnType::ArrayDomain(materialize_domain(source)?));
    }

    let field_type = match (data_type, udt_name) {
        ("ARRAY", "_text" | "_varchar" | "_bpchar") => FieldType::Array(ElemType::Text),
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
        ("ARRAY", "_time") => FieldType::Array(ElemType::Time),
        ("ARRAY", "_timetz") => FieldType::Array(ElemType::Timetz),
        ("ARRAY", "_interval") => FieldType::Array(ElemType::Interval),
        (_, "int4range") => FieldType::Range(ElemType::Int),
        (_, "int8range") => FieldType::Range(ElemType::BigInt),
        (_, "numrange") => FieldType::Range(ElemType::Numeric),
        (_, "tsrange") => FieldType::Range(ElemType::Timestamp),
        (_, "tstzrange") => FieldType::Range(ElemType::Timestamptz),
        (_, "daterange") => FieldType::Range(ElemType::Date),
        (_, "text" | "varchar" | "bpchar") => FieldType::Text,
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
        (_, "time") => FieldType::Time,
        (_, "timetz") => FieldType::Timetz,
        (_, "interval") => FieldType::Interval,
        (_, "timestamp") => FieldType::Timestamp,
        (_, "timestamptz") => FieldType::Timestamptz,
        (_, "json" | "jsonb") => FieldType::Jsonb,
        _ => bail!("unsupported Postgres type: data_type `{data_type}`, udt_name `{udt_name}`"),
    };

    Ok(ColumnType::Core(field_type))
}

fn materialize_domain(source: &PgDomainSource) -> Result<PgDomain> {
    let family = type_family_for_udt(&source.base_udt_name)?;
    Ok(PgDomain {
        schema: source.schema.clone(),
        name: source.name.clone(),
        const_name: sanitize_ident(&source.name.to_shouty_snake_case()),
        family,
        value_repr: value_repr_for_family(family),
        select_repr: rqb_core::SelectRepr::Text,
    })
}

fn schema_type_key(schema: &str, name: &str) -> SchemaTypeKey {
    (schema.to_owned(), name.to_owned())
}

pub(crate) fn type_family_for_udt(udt_name: &str) -> Result<TypeFamily> {
    let family = match udt_name {
        "text" | "varchar" | "bpchar" | "citext" => TypeFamily::Text,
        "bool" => TypeFamily::Bool,
        "int2" | "int4" | "int8" | "float4" | "float8" | "numeric" => TypeFamily::Numeric,
        "uuid" => TypeFamily::Uuid,
        "date" => TypeFamily::Date,
        "time" => TypeFamily::Time,
        "timetz" => TypeFamily::Timetz,
        "interval" => TypeFamily::Interval,
        "timestamp" => TypeFamily::Timestamp,
        "timestamptz" => TypeFamily::Timestamptz,
        "json" | "jsonb" => TypeFamily::Jsonb,
        "bytea" => TypeFamily::Bytes,
        "inet" | "cidr" => TypeFamily::Network,
        "int4range" | "int8range" | "numrange" | "tsrange" | "tstzrange" | "daterange" => {
            TypeFamily::Range
        }
        _ => bail!("unsupported Postgres domain base type: udt_name `{udt_name}`"),
    };
    Ok(family)
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
        | TypeFamily::Time
        | TypeFamily::Timetz
        | TypeFamily::Interval
        | TypeFamily::Network
        | TypeFamily::Range => ValueRepr::String,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rqb_core::{ElemType, FieldType, TypeFamily, ValueRepr};

    use crate::model::{ColumnType, PgDomainSource};

    use super::map_field_type;

    #[test]
    fn maps_postgres_types() {
        let uint_256 = PgDomainSource {
            schema: "public".to_owned(),
            name: "uint_256".to_owned(),
            base_udt_name: "numeric".to_owned(),
        };
        let mut domains = BTreeMap::new();
        domains.insert((uint_256.schema.clone(), uint_256.name.clone()), uint_256);

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
            ("_time", ElemType::Time),
            ("_timetz", ElemType::Timetz),
            ("_interval", ElemType::Interval),
        ] {
            assert!(
                matches!(
                    map_field_type(
                        "ARRAY",
                        "pg_catalog",
                        udt_name,
                        None,
                        None,
                        &BTreeMap::new(),
                        &BTreeMap::new()
                    )
                    .unwrap(),
                    ColumnType::Core(FieldType::Array(actual)) if actual == expected
                ),
                "{udt_name} should map to {expected:?}"
            );
        }
        assert!(matches!(
            map_field_type(
                "USER-DEFINED",
                "pg_catalog",
                "uuid",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .unwrap(),
            ColumnType::Core(FieldType::Uuid)
        ));
        assert!(matches!(
            map_field_type(
                "jsonb",
                "pg_catalog",
                "jsonb",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .unwrap(),
            ColumnType::Core(FieldType::Jsonb)
        ));
        assert!(matches!(
            map_field_type(
                "timestamp with time zone",
                "pg_catalog",
                "timestamptz",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .unwrap(),
            ColumnType::Core(FieldType::Timestamptz)
        ));
        for (udt_name, expected) in [
            ("bytea", FieldType::Bytea),
            ("citext", FieldType::Citext),
            ("inet", FieldType::Inet),
            ("cidr", FieldType::Cidr),
            ("time", FieldType::Time),
            ("timetz", FieldType::Timetz),
            ("interval", FieldType::Interval),
            ("int4range", FieldType::Range(ElemType::Int)),
            ("tstzrange", FieldType::Range(ElemType::Timestamptz)),
        ] {
            assert!(
                matches!(
                    map_field_type(
                        "USER-DEFINED",
                        "pg_catalog",
                        udt_name,
                        None,
                        None,
                        &BTreeMap::new(),
                        &BTreeMap::new()
                    )
                    .unwrap(),
                    ColumnType::Core(actual) if actual == expected
                ),
                "{udt_name} should map to {expected:?}"
            );
        }
        assert!(
            map_field_type(
                "unknown",
                "public",
                "ltree",
                None,
                None,
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .unwrap_err()
            .to_string()
            .contains("unsupported Postgres type")
        );
        assert!(matches!(
            map_field_type(
                "numeric",
                "pg_catalog",
                "numeric",
                Some("public"),
                Some("uint_256"),
                &BTreeMap::new(),
                &domains
            )
            .unwrap(),
            ColumnType::Domain(domain)
                if domain.name == "uint_256"
                    && domain.family == TypeFamily::Numeric
                    && domain.value_repr == ValueRepr::DecimalString
        ));
        assert!(matches!(
            map_field_type(
                "ARRAY",
                "public",
                "_uint_256",
                None,
                None,
                &BTreeMap::new(),
                &domains
            )
            .unwrap(),
            ColumnType::ArrayDomain(domain)
                if domain.name == "uint_256"
                    && domain.family == TypeFamily::Numeric
                    && domain.value_repr == ValueRepr::DecimalString
        ));
    }
}
