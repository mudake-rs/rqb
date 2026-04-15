use crate::model::{ColumnType, KnownType};

pub(crate) fn map_column_type(data_type: &str, udt_name: &str) -> ColumnType {
    if data_type == "ARRAY"
        && let Some(elem) = udt_name.strip_prefix('_')
    {
        return match map_known_udt(elem) {
            Some(KnownType::Array(_)) => ColumnType::RawOnly {
                pg: udt_name.to_owned(),
            },
            Some(known) => ColumnType::Known(KnownType::Array(Box::new(known))),
            None => ColumnType::RawOnly {
                pg: udt_name.to_owned(),
            },
        };
    }

    match map_known_udt(udt_name) {
        Some(known) => ColumnType::Known(known),
        None => ColumnType::RawOnly {
            pg: udt_name.to_owned(),
        },
    }
}

fn map_known_udt(udt_name: &str) -> Option<KnownType> {
    let known = match udt_name {
        "text" | "varchar" | "bpchar" | "citext" => KnownType::Text,
        "bool" => KnownType::Bool,
        "int2" => KnownType::Int2,
        "int4" => KnownType::Int4,
        "int8" => KnownType::Int8,
        "float4" => KnownType::Float4,
        "float8" => KnownType::Float8,
        "numeric" => KnownType::Numeric,
        "uuid" => KnownType::Uuid,
        "date" => KnownType::Date,
        "time" => KnownType::Time,
        "timetz" => KnownType::Timetz,
        "timestamp" => KnownType::Timestamp,
        "timestamptz" => KnownType::Timestamptz,
        "interval" => KnownType::Interval,
        "json" | "jsonb" => KnownType::Json,
        "bytea" => KnownType::Bytes,
        "inet" => KnownType::Inet,
        "cidr" => KnownType::Cidr,
        "int4range" => KnownType::Range(Box::new(KnownType::Int4)),
        "int8range" => KnownType::Range(Box::new(KnownType::Int8)),
        "numrange" => KnownType::Range(Box::new(KnownType::Numeric)),
        "daterange" => KnownType::Range(Box::new(KnownType::Date)),
        "tsrange" => KnownType::Range(Box::new(KnownType::Timestamp)),
        "tstzrange" => KnownType::Range(Box::new(KnownType::Timestamptz)),
        _ => return None,
    };
    Some(known)
}

#[cfg(test)]
mod tests {
    use crate::model::{ColumnType, KnownType};

    use super::map_column_type;

    #[test]
    fn maps_common_sqlx_supported_types() {
        assert_eq!(
            map_column_type("uuid", "uuid"),
            ColumnType::Known(KnownType::Uuid)
        );
        assert_eq!(
            map_column_type("ARRAY", "_uuid"),
            ColumnType::Known(KnownType::Array(Box::new(KnownType::Uuid)))
        );
        assert_eq!(
            map_column_type("USER-DEFINED", "vector"),
            ColumnType::RawOnly {
                pg: "vector".to_owned()
            }
        );
    }
}
