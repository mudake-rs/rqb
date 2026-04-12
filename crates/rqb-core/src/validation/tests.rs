use super::*;
use crate::{
    Dataset, DbEnum, ElemType, EnumType, Error, Field, FieldType, JsonPathPolicy, Sort, count,
    field, sum,
};

const ASSET_STATE: EnumType = EnumType::new(None, "asset_state", &["active", "archived"]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetState {
    Active,
    Archived,
}

impl AssetState {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

impl DbEnum for AssetState {
    const TYPE: EnumType = ASSET_STATE;

    fn as_db_str(self) -> &'static str {
        Self::as_db_str(self)
    }
}

fn dataset() -> Dataset {
    Dataset::table("assets").fields([
        Field::new("id", FieldType::Uuid),
        Field::new("name", FieldType::Text).text_search("english"),
        Field::new("state", FieldType::Enum(<AssetState as DbEnum>::TYPE)),
        Field::mapped(
            "stateHistory",
            "state_history",
            FieldType::Array(ElemType::Enum(ASSET_STATE)),
        )
        .sortable(false),
        Field::mapped("blobName", "blob_name", FieldType::Text).selectable(false),
        Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false),
        Field::new("properties", FieldType::Jsonb)
            .sortable(false)
            .json_paths(JsonPathPolicy::Dynamic),
    ])
}

#[test]
fn rejects_unknown_query_fields() {
    let query = crate::select(dataset())
        .filter(field("missing").eq("x"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnknownField { .. }));
}

#[test]
fn rejects_hidden_selection() {
    let query = crate::select(dataset()).fields(["id", "blobName"]).build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::NotSelectable { .. }));
}

#[test]
fn rejects_json_path_sorting() {
    let query = crate::select(dataset())
        .order_by(Sort::asc("properties.score"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::NotSortable { .. }));
}

#[test]
fn accepts_json_path_filtering() {
    let query = crate::select(dataset())
        .filter(field("properties.score").gt(10))
        .build();
    ValidatedSelect::new(query).unwrap();
}

#[test]
fn deserializes_api_request_without_runtime_generics() {
    let request: crate::SearchRequest = serde_json::from_value(serde_json::json!({
        "fields": ["id", "name"],
        "sort": [{ "field": "name", "dir": "ASC" }],
        "query": {
            "logical": "or",
            "predicates": [
                { "field": "name", "operator": "contains", "value": "hero" },
                { "field": "properties.score", "operator": "gte", "value": 70 }
            ]
        },
        "limit": 50,
        "offset": 10
    }))
    .unwrap();

    let query = crate::select(dataset()).request(request).build();
    let validated = ValidatedSelect::new(query).unwrap();
    assert_eq!(validated.limit, 50);
    assert_eq!(validated.offset, 10);
    assert_eq!(validated.selected_fields.len(), 2);
}

#[test]
fn reports_specific_expr_json_shape_errors() {
    let err = serde_json::from_value::<crate::Expr>(serde_json::json!({
        "field": "name",
        "operatr": "equals",
        "value": "chair"
    }))
    .unwrap_err();

    assert!(
        err.to_string().contains("predicate is missing `operator`"),
        "{err}"
    );
}

#[test]
fn rejects_mismatched_raw_binds() {
    let query = crate::select(dataset())
        .filter(crate::raw("exists (select 1 where x = ? and y = ?)").bind(1))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::RawBindMismatch { .. }));
}

#[test]
fn rejects_not_in_without_array() {
    let query = crate::select(dataset())
        .filter(field("name").not_in("x"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::InvalidValue { .. }));
}

#[test]
fn accepts_is_distinct_from_scalar() {
    let query = crate::select(dataset())
        .filter(field("name").is_distinct_from("x"))
        .build();
    ValidatedSelect::new(query).unwrap();
}

#[test]
fn accepts_rust_enum_values_for_enum_fields() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("state").eq(AssetState::Active),
            field("state").gte(AssetState::Active),
            field("state").not_in([AssetState::Archived]),
            field("stateHistory").has(AssetState::Archived),
            field("stateHistory").contains_any([AssetState::Active, AssetState::Archived]),
        ]))
        .build();

    ValidatedSelect::new(query).unwrap();
}

#[test]
fn rejects_invalid_enum_value() {
    let query = crate::select(dataset())
        .filter(field("state").eq("missing"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidEnumValue { field, value, allowed }
            if field == "state" && value == "missing" && allowed == "active, archived"
    ));
}

#[test]
fn rejects_invalid_enum_array_value() {
    let query = crate::select(dataset())
        .filter(field("stateHistory").contains_any(["active", "missing"]))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidEnumValue { field, value, .. }
            if field == "stateHistory" && value == "missing"
    ));
}

#[test]
fn accepts_null_write_values_for_enum_fields() {
    let query = crate::update(dataset())
        .set_null("state")
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    ValidatedUpdate::new(query).unwrap();
}

#[test]
fn accepts_null_write_values_for_enum_array_fields() {
    let query = crate::update(dataset())
        .set_null("stateHistory")
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    ValidatedUpdate::new(query).unwrap();
}

#[test]
fn rejects_delete_without_filter() {
    let query = crate::delete(dataset()).build();
    let err = ValidatedDelete::new(query).unwrap_err();
    assert!(matches!(err, Error::DeleteWithoutFilter));
}

#[test]
fn rejects_regex_on_uuid_fields() {
    let query = crate::select(dataset())
        .filter(field("id").regex("1000.*"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnsupportedOperator { .. }));
}

#[test]
fn rejects_array_contains_on_non_array_field() {
    let query = crate::select(dataset())
        .filter(field("name").has("x"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnsupportedOperator { .. }));
}

#[test]
fn rejects_json_key_exists_on_non_jsonb_field() {
    let query = crate::select(dataset())
        .filter(field("name").key_exists("score"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnsupportedOperator { .. }));
}

#[test]
fn rejects_text_search_on_unconfigured_field() {
    let query = crate::select(dataset())
        .filter(field("blobName").search("hidden"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::TextSearchNotConfigured { .. }));
}

#[test]
fn accepts_grouped_aggregate_selection() {
    let query = crate::select(dataset())
        .fields(["state"])
        .agg(count("count"))
        .group_by(["state"])
        .build();

    let validated = ValidatedSelect::new(query).unwrap();
    assert_eq!(validated.group_by.len(), 1);
    assert_eq!(validated.aggregates.len(), 1);
    assert_eq!(validated.columns.len(), 2);
}

#[test]
fn rejects_non_grouped_selected_field() {
    let query = crate::select(dataset())
        .fields(["state", "name"])
        .agg(count("count"))
        .group_by(["state"])
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UngroupedField { field } if field == "name"));
}

#[test]
fn rejects_group_by_on_json_path() {
    let query = crate::select(dataset())
        .fields(["state"])
        .agg(count("count"))
        .group_by(["properties.score"])
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::NotSelectable { .. }));
}

#[test]
fn rejects_unknown_aggregate_alias_modifier() {
    let query = crate::select(dataset())
        .agg(sum("id", "total"))
        .filter_agg("totla", field("state").eq("active"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnknownAggregateAlias { alias } if alias == "totla"));
}

#[test]
fn rejects_order_within_on_non_orderable_aggregate() {
    let query = crate::select(dataset())
        .agg(count("count"))
        .order_within("count", Sort::asc("name"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::AggregateOrderUnsupported { alias } if alias == "count"));
}

fn orders_dataset() -> Dataset {
    Dataset::table("orders").fields([
        Field::new("id", FieldType::Uuid),
        Field::mapped("userId", "user_id", FieldType::Uuid),
        Field::new("status", FieldType::Text),
    ])
}

fn users_dataset() -> Dataset {
    Dataset::table("app_users").fields([
        Field::new("id", FieldType::Uuid),
        Field::new("email", FieldType::Text),
        Field::new("status", FieldType::Text),
    ])
}

#[test]
fn accepts_qualified_join_fields() {
    let query = crate::select(orders_dataset().alias("o"))
        .join(
            users_dataset().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .fields(["o.id", "u.email"])
        .filter(field("u.status").eq("active"))
        .build();

    let validated = ValidatedSelect::new(query).unwrap();
    assert_eq!(validated.selected_fields[0].display_name(), "o.id");
    assert_eq!(validated.selected_fields[1].display_name(), "u.email");
}

#[test]
fn rejects_ambiguous_join_field() {
    let query = crate::select(orders_dataset().alias("o"))
        .join(
            users_dataset().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::AmbiguousField { .. }));
}
