use crate::model::{ColumnType, KnownType};

pub(crate) fn map_column_type(udt_name: &str) -> ColumnType {
    if let Some(elem) = udt_name.strip_prefix('_') {
        return match map_known_udt(elem) {
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
        "json" => KnownType::Json,
        "jsonb" => KnownType::Jsonb,
        "bytea" => KnownType::Bytes,
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
        assert_eq!(map_column_type("uuid"), ColumnType::Known(KnownType::Uuid));
        assert_eq!(
            map_column_type("_uuid"),
            ColumnType::Known(KnownType::Array(Box::new(KnownType::Uuid)))
        );
        assert_eq!(
            map_column_type("vector"),
            ColumnType::RawOnly {
                pg: "vector".to_owned()
            }
        );
    }

    #[test]
    fn maps_temporal_network_range_and_json_types() {
        assert_eq!(
            map_column_type("timestamptz"),
            ColumnType::Known(KnownType::Timestamptz)
        );
        assert_eq!(
            map_column_type("timetz"),
            ColumnType::Known(KnownType::Timetz)
        );
        assert!(matches!(
            map_column_type("inet"),
            ColumnType::RawOnly { .. }
        ));
        assert!(matches!(
            map_column_type("cidr"),
            ColumnType::RawOnly { .. }
        ));
        assert_eq!(
            map_column_type("tstzrange"),
            ColumnType::Known(KnownType::Range(Box::new(KnownType::Timestamptz)))
        );
        assert_eq!(map_column_type("json"), ColumnType::Known(KnownType::Json));
        assert_eq!(
            map_column_type("jsonb"),
            ColumnType::Known(KnownType::Jsonb)
        );
    }

    #[test]
    fn unknown_element_types_fall_back_to_raw_only() {
        assert_eq!(
            map_column_type("__int4"),
            ColumnType::RawOnly {
                pg: "__int4".to_owned()
            }
        );
    }

    #[test]
    fn maps_arrays_of_temporal_range_and_network_types() {
        assert_eq!(
            map_column_type("_timestamptz"),
            ColumnType::Known(KnownType::Array(Box::new(KnownType::Timestamptz)))
        );
        assert_eq!(
            map_column_type("_int4range"),
            ColumnType::Known(KnownType::Array(Box::new(KnownType::Range(Box::new(
                KnownType::Int4
            )))))
        );
        assert_eq!(
            map_column_type("_inet"),
            ColumnType::RawOnly {
                pg: "_inet".to_owned()
            }
        );
    }

    #[test]
    fn unknown_arrays_remain_raw_only() {
        assert_eq!(
            map_column_type("_vector"),
            ColumnType::RawOnly {
                pg: "_vector".to_owned()
            }
        );
    }
}
