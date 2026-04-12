use super::*;
use crate::{
    ColumnOperator, Dataset, DbEnum, ElemType, EnumType, Error, Expr, Field, FieldType, Join,
    JoinKind, JsonPathPolicy, LogicalExpr, LogicalOp, Operator, Sort, Value, count, field, insert,
    sum,
};
use pretty_assertions::assert_eq;
use serde::{Serialize, Serializer};

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
        Field::new("internal", FieldType::Text).filterable(false),
        Field::new("score", FieldType::Integer),
        Field::new("active", FieldType::Bool),
        Field::mapped("createdAt", "created_at", FieldType::Timestamp),
        Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false),
        Field::new("strictProperties", FieldType::Jsonb).sortable(false),
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
fn accepts_between_on_numeric_and_temporal_fields() {
    for expr in [
        field("score").between(1, 10),
        field("createdAt").between("2026-01-01T00:00:00Z", "2026-02-01T00:00:00Z"),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        ValidatedSelect::new(query).unwrap();
    }
}

#[test]
fn rejects_between_on_bool_fields() {
    let query = crate::select(dataset())
        .filter(field("active").between(false, true))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnsupportedOperator { operator, .. } if operator == "between"));
}

#[test]
fn rejects_between_without_two_values() {
    let query = crate::select(dataset())
        .filter(Expr::predicate(
            field("score"),
            Operator::Between,
            Value::Array(vec![1.into()]),
        ))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidValue { message, .. } if message == "expected exactly 2 values, got 1"
    ));
}

#[test]
fn rejects_json_path_between_with_non_numeric_bounds() {
    let query = crate::select(dataset())
        .filter(field("properties.score").between("low", "high"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidValue { message, .. }
            if message == "JSONB path range comparisons require numeric bounds"
    ));
}

#[test]
fn accepts_array_contains_all_and_elem_match_on_supported_fields() {
    for expr in [
        field("tags").contains_all(["vip", "gift"]),
        field("tags").elem_match("vip"),
        field("properties").elem_match(serde_json::json!({"gift": true})),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        ValidatedSelect::new(query).unwrap();
    }
}

#[test]
fn accepts_json_keys_exist_all_on_jsonb_fields() {
    let query = crate::select(dataset())
        .filter(field("properties").keys_exist_all(["campaign", "score"]))
        .build();

    ValidatedSelect::new(query).unwrap();
}

#[test]
fn rejects_not_contains_on_non_text_fields() {
    let query = crate::select(dataset())
        .filter(field("score").not_contains("1"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::UnsupportedOperator { operator, .. } if operator == "notContains"
    ));
}

#[test]
fn rejects_filters_on_non_filterable_fields() {
    let query = crate::select(dataset())
        .filter(field("internal").eq("secret"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::NotFilterable { field } if field == "internal"));
}

#[test]
fn rejects_json_path_when_policy_denies_dynamic_paths() {
    let query = crate::select(dataset())
        .filter(field("strictProperties.score").eq(80))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::JsonbPathDenied { field } if field == "strictProperties"));
}

#[test]
fn rejects_json_path_on_non_jsonb_fields() {
    let query = crate::select(dataset())
        .filter(field("name.first").eq("Ada"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::NotJsonbPath { field, path } if field == "name" && path == "name.first")
    );
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
fn rejects_empty_logical_expression() {
    let query = crate::select(dataset())
        .filter(crate::all(Vec::<Expr>::new()))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::EmptyLogical { logical } if logical == "and"));
}

#[test]
fn rejects_not_with_multiple_predicates() {
    let query = crate::select(dataset())
        .filter(Expr::Logical(LogicalExpr {
            logical: LogicalOp::Not,
            predicates: vec![field("name").eq("Ada"), field("score").gt(10)],
        }))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::InvalidNot));
}

#[test]
fn rejects_limit_over_dataset_maximum() {
    let query = crate::select(dataset().max_limit(10)).limit(11).build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::LimitExceeded {
            requested: 11,
            max: 10
        }
    ));
}

#[test]
fn rejects_incompatible_column_predicate_types() {
    let query = crate::select(dataset())
        .filter(Expr::column_predicate(
            field("name"),
            ColumnOperator::Equals,
            field("id"),
        ))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::IncompatibleColumnTypes { left, right, .. } if left == "name" && right == "id"
    ));
}

#[test]
fn rejects_empty_insert_and_update_queries() {
    let insert = insert(dataset()).build().unwrap();
    assert!(matches!(
        ValidatedInsert::new(insert).unwrap_err(),
        Error::EmptyInsert
    ));

    let update = crate::update(dataset()).build().unwrap();
    assert!(matches!(
        ValidatedUpdate::new(update).unwrap_err(),
        Error::EmptyUpdate
    ));
}

#[test]
fn rejects_unsupported_write_sources() {
    let query = insert(Dataset::cte("source").fields([Field::new("id", FieldType::Uuid)]))
        .set("id", "10000000-0000-0000-0000-000000000001")
        .build()
        .unwrap();

    let err = ValidatedInsert::new(query).unwrap_err();
    assert!(matches!(err, Error::UnsupportedWriteSource));
}

#[test]
fn rejects_inconsistent_insert_row_shapes() {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Patch {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        score: Option<i64>,
    }

    let query = insert(dataset())
        .value(&Patch {
            name: Some("Ada".to_owned()),
            score: None,
        })
        .value(&Patch {
            name: None,
            score: Some(10),
        })
        .build()
        .unwrap();

    let err = ValidatedInsert::new(query).unwrap_err();
    assert!(matches!(err, Error::InconsistentInsertFields));
}

#[test]
fn rejects_non_object_insert_records() {
    let err = insert(dataset()).value(&42).build().unwrap_err();
    assert!(
        matches!(err, Error::ExpectedObject { message } if message == "expected object, got number")
    );
}

#[test]
fn reports_serde_errors_from_record_conversion() {
    struct BrokenRecord;

    impl Serialize for BrokenRecord {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    let err = insert(dataset()).value(&BrokenRecord).build().unwrap_err();
    assert!(matches!(err, Error::SerdeError { message } if message == "boom"));
}

#[test]
fn rejects_conflict_filter_without_update_action() {
    let err = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .filter(field("name").eq("Ada"))
        .build()
        .unwrap_err();

    assert!(matches!(err, Error::InvalidConflictFilter));
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

#[test]
fn rejects_duplicate_aggregate_aliases() {
    let query = crate::select(dataset())
        .agg(count("total"))
        .agg(sum("score", "total"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::DuplicateAggregateAlias { alias } if alias == "total"));
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

#[test]
fn rejects_missing_join_condition() {
    let mut query = crate::select(orders_dataset().alias("o")).build();
    query.joins.push(Join {
        kind: JoinKind::Inner,
        dataset: users_dataset().alias("u"),
        on: None,
    });

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::MissingJoinCondition { kind, dataset }
            if kind == "JOIN" && dataset == "app_users"
    ));
}

#[test]
fn rejects_unknown_dataset_qualifier() {
    let query = crate::select(orders_dataset().alias("o"))
        .join(
            users_dataset().alias("u"),
            field("o.userId").eq_col(field("u.id")),
        )
        .filter(Field::new("status", FieldType::Text).on("x").eq("active"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::UnknownDatasetQualifier { qualifier } if qualifier == "x"));
}

#[test]
fn rejects_duplicate_dataset_qualifiers() {
    let query = crate::select(orders_dataset().alias("x"))
        .join(
            users_dataset().alias("x"),
            field("x.userId").eq_col(field("x.id")),
        )
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::DuplicateDatasetQualifier { qualifier } if qualifier == "x"));
}

#[test]
fn rejects_ambiguous_dataset_qualifiers() {
    let query = crate::select(orders_dataset().alias("left_order"))
        .join(
            orders_dataset().alias("right_order"),
            field("left_order.id").eq_col(field("right_order.id")),
        )
        .filter(field("orders.id").eq("10000000-0000-0000-0000-000000000001"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::AmbiguousDatasetQualifier { qualifier, .. } if qualifier == "orders")
    );
}
