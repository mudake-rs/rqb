use super::*;
use crate::{
    ColumnOperator, Dataset, DbEnum, ElemType, EnumType, Error, Expr, Field, FieldType,
    IntoSqlExpr, Join, JoinKind, JsonPathPolicy, LogicalExpr, LogicalOp, Operator, SelectRepr,
    Sort, SubqueryOperator, TypeFamily, TypeSpec, Value, ValueRepr, avg, case_when, cast, coalesce,
    count, count_field, excluded, field, insert, max, min, set_default, set_expr, string_agg, sum,
};
use pretty_assertions::assert_eq;
use serde::{Serialize, Serializer};

const ASSET_STATE: EnumType = EnumType::new(None, "asset_state", &["active", "archived"]);
const UINT_256: TypeSpec = TypeSpec::domain(Some("public"), "uint_256")
    .base(TypeFamily::Numeric)
    .value_repr(ValueRepr::DecimalString)
    .select_repr(SelectRepr::Text);

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
        Field::mapped("displayName", "display_name", FieldType::Citext),
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
        Field::new("amount", FieldType::Numeric),
        Field::mapped("uintAmount", "uint_amount", FieldType::Custom(&UINT_256)),
        Field::mapped(
            "uintAmounts",
            "uint_amounts",
            FieldType::Array(ElemType::Custom(&UINT_256)),
        )
        .sortable(false),
        Field::new("active", FieldType::Bool),
        Field::mapped("createdAt", "created_at", FieldType::Timestamp),
        Field::mapped("observedAt", "observed_at", FieldType::Timestamptz),
        Field::new("payload", FieldType::Bytea),
        Field::mapped("ipAddr", "ip_addr", FieldType::Inet),
        Field::new("network", FieldType::Cidr),
        Field::mapped(
            "activeWindow",
            "active_window",
            FieldType::Range(ElemType::Timestamptz),
        ),
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
fn accepts_known_fields_on_unfielded_dataset_without_joins() {
    let id = Field::new("id", FieldType::Uuid);
    let total = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
    let dataset = Dataset::table("orders_archive");

    let select = crate::select(dataset.clone())
        .fields([id, total])
        .filter(total.gte(10_000))
        .build();
    let validated = ValidatedSelect::new(select).unwrap();
    assert_eq!(validated.selected_fields.len(), 2);
    assert_eq!(validated.selected_fields[1].db_name, "total_cents");

    let insert = insert(dataset.clone())
        .set(id, "30000000-0000-0000-0000-000000000001")
        .set(total, 15_900)
        .build()
        .unwrap();
    ValidatedInsert::new(insert).unwrap();

    let update = crate::update(dataset)
        .set(total, 10_900)
        .filter(id.eq("30000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();
    ValidatedUpdate::new(update).unwrap();
}

#[test]
fn rejects_named_fields_on_unfielded_dataset_without_descriptor() {
    let query = crate::select(Dataset::table("orders_archive"))
        .fields(["id"])
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::UnknownField { dataset, field }
            if dataset == "orders_archive" && field == "id"
    ));
}

#[test]
fn rejects_hidden_selection() {
    let query = crate::select(dataset()).fields(["id", "blobName"]).build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(err, Error::NotSelectable { .. }));
}

#[test]
fn validates_expression_select_items_and_output_columns() {
    let query = crate::select(dataset())
        .fields(["id"])
        .select_expr(coalesce([field("displayName").expr(), field("name").expr()]).alias("label"))
        .select_expr(
            case_when(field("state").eq("active"))
                .then("live")
                .when(field("state").eq("archived"))
                .then("old")
                .otherwise("other")
                .alias("stateLabel"),
        )
        .select_expr(cast(field("score").expr(), FieldType::Text).alias("scoreText"))
        .build();

    let validated = ValidatedSelect::new(query).unwrap();

    assert_eq!(validated.select_items.len(), 3);
    assert_eq!(
        validated
            .columns
            .iter()
            .map(crate::SelectColumn::alias)
            .collect::<Vec<_>>(),
        ["id", "label", "stateLabel", "scoreText"]
    );
}

#[test]
fn rejects_expression_select_items_that_leak_hidden_fields() {
    let query = crate::select(dataset())
        .select_expr(coalesce([field("blobName").expr(), field("name").expr()]).alias("label"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();

    assert!(matches!(err, Error::NotSelectable { field } if field == "blobName"));
}

#[test]
fn rejects_case_conditions_that_leak_hidden_fields() {
    let query = crate::select(dataset())
        .select_expr(
            case_when(field("blobName").eq("secret"))
                .then("yes")
                .otherwise("no")
                .alias("derived"),
        )
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();

    assert!(matches!(err, Error::NotSelectable { field } if field == "blobName"));
}

#[test]
fn rejects_duplicate_output_aliases_across_fields_aggregates_and_expressions() {
    let field_duplicate = crate::select(dataset())
        .fields(["id"])
        .select_expr(field("name").expr().alias("id"))
        .build();
    let err = ValidatedSelect::new(field_duplicate).unwrap_err();
    assert!(matches!(err, Error::DuplicateOutputAlias { alias } if alias == "id"));

    let aggregate_duplicate = crate::select(dataset())
        .agg(count("total"))
        .select_expr(field("name").expr().alias("total"))
        .build();
    let err = ValidatedSelect::new(aggregate_duplicate).unwrap_err();
    assert!(matches!(err, Error::DuplicateOutputAlias { alias } if alias == "total"));

    let expression_duplicate = crate::select(dataset())
        .select_expr(field("name").expr().alias("label"))
        .select_expr(field("displayName").expr().alias("label"))
        .build();
    let err = ValidatedSelect::new(expression_duplicate).unwrap_err();
    assert!(matches!(err, Error::DuplicateOutputAlias { alias } if alias == "label"));
}

#[test]
fn rejects_expression_select_items_with_incompatible_branch_types() {
    let query = crate::select(dataset())
        .select_expr(
            case_when(field("active").eq(true))
                .then("yes")
                .otherwise(0)
                .alias("activeLabel"),
        )
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();

    assert!(
        matches!(err, Error::IncompatibleExpressionTypes { expression, .. } if expression == "case")
    );
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
fn lowers_user_predicates_to_concrete_validated_shapes() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("name").contains("ada"),
            field("network").contains("10.1.2.0/24"),
            field("activeWindow").overlaps("[2026-02-01T00:00:00Z,2026-03-01T00:00:00Z)"),
            field("score").between(1, 10),
            field("state").is_in(["active", "archived"]),
        ]))
        .build();

    let validated = ValidatedSelect::new(query).unwrap();
    let ValidatedExpr::Logical { predicates, .. } = validated.filter.unwrap() else {
        panic!("expected top-level logical expression");
    };

    assert!(matches!(
        &predicates[0],
        ValidatedExpr::Predicate(ValidatedPredicate::Like {
            pattern: ValidatedLikePattern::Contains,
            value,
            negated: false,
            ..
        }) if value == "ada"
    ));
    assert!(matches!(
        &predicates[1],
        ValidatedExpr::Predicate(ValidatedPredicate::Containment {
            op: ValidatedContainmentOperator::Contains,
            target: ValidatedContainmentTarget::Network,
            negated: false,
            ..
        })
    ));
    assert!(matches!(
        &predicates[2],
        ValidatedExpr::Predicate(ValidatedPredicate::Containment {
            op: ValidatedContainmentOperator::Overlaps,
            target: ValidatedContainmentTarget::Range,
            ..
        })
    ));
    assert!(matches!(
        &predicates[3],
        ValidatedExpr::Predicate(ValidatedPredicate::Between {
            lower: Value::I64(1),
            upper: Value::I64(10),
            negated: false,
            ..
        })
    ));
    assert!(matches!(
        &predicates[4],
        ValidatedExpr::Predicate(ValidatedPredicate::In {
            values,
            negated: false,
            ..
        }) if values.len() == 2
    ));
}

#[test]
fn lowers_non_value_predicates_to_concrete_validated_shapes() {
    let subquery = crate::select(dataset().alias("b"))
        .fields([field("b.id")])
        .filter(field("b.state").eq("active"));
    let exists_query = crate::select(dataset().alias("c")).filter(field("c.state").eq("archived"));

    let query = crate::select(dataset().alias("a"))
        .filter(crate::all([
            field("a.score").gt_col(field("a.score")),
            field("a.id").in_subquery(subquery),
            crate::exists(exists_query),
            Expr::raw(crate::raw("score > ?").bind(10)),
        ]))
        .build();

    let validated = ValidatedSelect::new(query).unwrap();
    let ValidatedExpr::Logical { predicates, .. } = validated.filter.unwrap() else {
        panic!("expected top-level logical expression");
    };

    assert!(matches!(
        &predicates[0],
        ValidatedExpr::Predicate(ValidatedPredicate::ColumnBinary {
            operator: ColumnOperator::Gt,
            ..
        })
    ));
    assert!(matches!(
        &predicates[1],
        ValidatedExpr::Predicate(ValidatedPredicate::Subquery {
            operator: SubqueryOperator::In,
            query,
            ..
        }) if query.columns().len() == 1
    ));
    assert!(matches!(
        &predicates[2],
        ValidatedExpr::Predicate(ValidatedPredicate::Exists { negated: false, .. })
    ));
    assert!(matches!(
        &predicates[3],
        ValidatedExpr::Predicate(ValidatedPredicate::Raw(raw)) if raw.binds.len() == 1
    ));
}

#[test]
fn accepts_between_on_numeric_and_temporal_fields() {
    for expr in [
        field("score").between(1, 10),
        field("createdAt").between("2026-01-01T00:00:00Z", "2026-02-01T00:00:00Z"),
        field("observedAt").between("2026-01-01T00:00:00Z", "2026-02-01T00:00:00Z"),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        ValidatedSelect::new(query).unwrap();
    }
}

#[test]
fn accepts_native_postgres_type_values_and_operators() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("displayName").eq("Ada"),
            field("payload").eq(Value::bytes([0xde, 0xad])),
            field("ipAddr").contained_by("10.0.0.0/8"),
            field("network").contains("10.1.2.0/24"),
            field("activeWindow").overlaps("[2026-02-01T00:00:00Z,2026-03-01T00:00:00Z)"),
        ]))
        .build();

    ValidatedSelect::new(query).unwrap();
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
        field("properties").eq([1, 2, 3]),
        field("properties").is_not_distinct_from(["vip", "gift"]),
        field("properties").elem_match(serde_json::json!({"gift": true})),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        ValidatedSelect::new(query).unwrap();
    }
}

#[test]
fn rejects_array_elem_match_non_scalar_for_sql_arrays() {
    let query = crate::select(dataset())
        .filter(field("tags").elem_match(serde_json::json!({"tag": "vip"})))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "tags" && message == "expected scalar, got json")
    );
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
fn rejects_range_and_network_operators_on_other_fields() {
    let query = crate::select(dataset())
        .filter(field("name").overlaps("x"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::UnsupportedOperator { field, operator, .. }
            if field == "name" && operator == "overlaps"
    ));
}

#[test]
fn rejects_json_value_comparison_on_non_jsonb_fields() {
    let query = crate::select(dataset())
        .filter(field("name").eq(serde_json::json!({"name": "Ada"})))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "name" && message == "JSON values require a JSONB field")
    );
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
fn reports_specific_expr_json_shape_errors_for_all_expr_families() {
    for (json, expected) in [
        (
            serde_json::json!("not an object"),
            "expression must be a JSON object",
        ),
        (
            serde_json::json!({ "logical": "and" }),
            "logical expression is missing `predicates`",
        ),
        (
            serde_json::json!({ "predicates": [] }),
            "logical expression is missing `logical`",
        ),
        (
            serde_json::json!({ "left": "a", "operator": "equals" }),
            "column predicate is missing `right`",
        ),
        (
            serde_json::json!({ "right": "b", "operator": "equals" }),
            "column predicate is missing `left`",
        ),
        (
            serde_json::json!({ "field": "name", "operator": "unknown" }),
            "invalid predicate:",
        ),
        (
            serde_json::json!({ "unexpected": true }),
            "expression must contain `field`, `left`/`right`, or `logical`",
        ),
    ] {
        let err = serde_json::from_value::<crate::Expr>(json).unwrap_err();
        assert!(
            err.to_string().contains(expected),
            "expected `{expected}` in `{err}`"
        );
    }
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
        .conflict_filter(field("name").eq("Ada"))
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
fn rejects_in_on_array_fields() {
    let query = crate::select(dataset())
        .filter(field("tags").is_in(["vip"]))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedOperator { field, operator, .. } if field == "tags" && operator == "in")
    );
}

#[test]
fn rejects_scalar_values_that_do_not_match_field_types() {
    for (expr, field, expected) in [
        (field("name").eq(42), "name", "expected string, got i64"),
        (
            field("score").eq("42"),
            "score",
            "expected integer, got string",
        ),
        (
            field("active").eq("true"),
            "active",
            "expected bool, got string",
        ),
        (field("id").eq(42), "id", "expected UUID string, got i64"),
        (
            field("createdAt").eq(42),
            "createdAt",
            "expected timestamp string, got i64",
        ),
        (
            field("observedAt").eq(42),
            "observedAt",
            "expected timestamptz string, got i64",
        ),
        (
            field("payload").eq("deadbeef"),
            "payload",
            "expected bytes, got string",
        ),
        (
            field("ipAddr").eq(42),
            "ipAddr",
            "expected network string, got i64",
        ),
        (
            field("activeWindow").eq(42),
            "activeWindow",
            "expected range literal string, got i64",
        ),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        let err = ValidatedSelect::new(query).unwrap_err();
        assert!(
            matches!(err, Error::InvalidValue { field: ref actual, ref message, .. } if actual == field && message == expected),
            "unexpected error for {field}: {err}"
        );
    }
}

#[test]
fn accepts_numeric_strings_only_for_numeric_fields() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("amount").eq("9007199254740993"),
            field("amount").between("1.25", "2.50"),
        ]))
        .build();

    ValidatedSelect::new(query).unwrap();
}

#[test]
fn accepts_exact_decimal_strings_for_custom_numeric_domains() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("uintAmount").eq("900719925474099312345678901234567890"),
            field("uintAmount").gte(10_i64),
            field("uintAmount").lte_col(field("amount")),
        ]))
        .build();

    ValidatedSelect::new(query).unwrap();
}

#[test]
fn accepts_exact_decimal_strings_for_custom_numeric_domain_arrays() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("uintAmounts").contains_all(["900719925474099312345678901234567890"]),
            field("uintAmounts").has(42_i64),
        ]))
        .build();

    ValidatedSelect::new(query).unwrap();
}

#[test]
fn validates_custom_numeric_domains_from_json_requests() {
    let request: crate::SearchRequest = serde_json::from_value(serde_json::json!({
        "query": {
            "field": "uintAmount",
            "operator": "gte",
            "value": "900719925474099312345678901234567890"
        }
    }))
    .unwrap();

    let query = crate::select(dataset()).request(request).build();
    ValidatedSelect::new(query).unwrap();
}

#[test]
fn rejects_inexact_values_for_decimal_string_domains() {
    for (expr, expected) in [
        (
            field("uintAmount").eq(1.25),
            "expected integer or decimal string, got f64",
        ),
        (
            field("uintAmount").eq("not-a-number"),
            "expected integer or decimal string, got string",
        ),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        let err = ValidatedSelect::new(query).unwrap_err();
        assert!(
            matches!(err, Error::InvalidValue { ref field, ref message, .. } if field == "uintAmount" && message == expected),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn rejects_inexact_values_for_decimal_string_domain_arrays() {
    for (expr, expected) in [
        (
            field("uintAmounts").contains_all([1.25]),
            "expected integer or decimal string, got f64",
        ),
        (
            field("uintAmounts").has("not-a-number"),
            "expected integer or decimal string, got string",
        ),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        let err = ValidatedSelect::new(query).unwrap_err();
        assert!(
            matches!(err, Error::InvalidValue { ref field, ref message, .. } if field == "uintAmounts" && message == expected),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn rejects_array_operator_values_that_do_not_match_element_type() {
    let query = crate::select(dataset())
        .filter(field("tags").contains_all([1]))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "tags" && message == "expected string, got i64")
    );

    let query = crate::select(dataset())
        .filter(field("tags").has(1))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "tags" && message == "expected string, got i64")
    );

    let query = crate::select(dataset())
        .filter(field("tags").elem_match(1))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "tags" && message == "expected string, got i64")
    );
}

#[test]
fn rejects_non_finite_numbers_that_would_be_encoded_as_json() {
    for expr in [
        field("properties").eq(f64::NAN),
        field("properties.score").eq(f64::INFINITY),
        field("properties").elem_match(f64::NEG_INFINITY),
        field("properties").eq(Value::Array(vec![Value::F64(f64::NAN)])),
    ] {
        let query = crate::select(dataset()).filter(expr).build();
        let err = ValidatedSelect::new(query).unwrap_err();
        assert!(
            matches!(err, Error::InvalidValue { ref message, .. } if message == "non-finite numbers are not supported"),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn accepts_is_distinct_from_scalar() {
    let query = crate::select(dataset())
        .filter(crate::all([
            field("name").is_distinct_from("x"),
            field("state").is_not_distinct_from(()),
        ]))
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
fn rejects_invalid_enum_array_elem_match_value() {
    let query = crate::select(dataset())
        .filter(field("stateHistory").elem_match("missing"))
        .build();

    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(
        matches!(err, Error::InvalidEnumValue { field, value, .. } if field == "stateHistory" && value == "missing")
    );
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
fn accepts_jsonb_array_write_values() {
    let query = crate::update(dataset())
        .set("properties", Value::Array(vec![1.into(), 2.into()]))
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    ValidatedUpdate::new(query).unwrap();
}

#[test]
fn accepts_write_expressions_defaults_and_returning_expressions() {
    let update = crate::update(dataset())
        .set_expr(
            "score",
            coalesce([field("score").expr(), 0.into_sql_expr()]),
        )
        .set_default("properties")
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .returning(["id"])
        .returning_expr(field("name").expr().alias("label"))
        .build()
        .unwrap();
    let validated = ValidatedUpdate::new(update).unwrap();
    assert_eq!(validated.assignments.len(), 2);
    assert_eq!(validated.returning.len(), 2);

    let insert = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .set_expr(
            "name",
            coalesce(["generated".into_sql_expr(), "fallback".into_sql_expr()]),
        )
        .set_default("properties")
        .returning_expr(field("name").expr().alias("label"))
        .build()
        .unwrap();
    ValidatedInsert::new(insert).unwrap();
}

#[test]
fn accepts_advanced_write_sources_and_conflict_assignments() {
    let update = crate::update(orders_dataset().alias("o"))
        .from(users_dataset().alias("u"))
        .set_col("userId", field("u.id"))
        .filter(field("o.userId").eq_col(field("u.id")))
        .build()
        .unwrap();
    let validated = ValidatedUpdate::new(update).unwrap();
    assert_eq!(validated.from.len(), 1);

    let delete = crate::delete(orders_dataset().alias("o"))
        .using(users_dataset().alias("u"))
        .filter(field("o.userId").eq_col(field("u.id")))
        .build();
    let validated = ValidatedDelete::new(delete).unwrap();
    assert_eq!(validated.using.len(), 1);

    let insert = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .set("score", 10)
        .on_conflict("id")
        .index_where(field("active").eq(true))
        .do_update_set([
            set_expr("score", excluded("score")),
            set_default("properties"),
        ])
        .conflict_filter(field("active").eq(true))
        .build()
        .unwrap();
    let validated = ValidatedInsert::new(insert).unwrap();
    assert!(validated.conflict.is_some());
}

#[test]
fn rejects_insert_expressions_that_reference_target_fields() {
    let insert = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .set_expr("name", field("name").expr())
        .build()
        .unwrap();

    let err = ValidatedInsert::new(insert).unwrap_err();

    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "name" && message == "insert expressions cannot reference target fields")
    );
}

#[test]
fn rejects_excluded_outside_conflict_update_assignments() {
    let update = crate::update(dataset())
        .set_expr("score", excluded("score"))
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    let err = ValidatedUpdate::new(update).unwrap_err();

    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "excluded" && message == "EXCLUDED fields are only valid in ON CONFLICT DO UPDATE assignments")
    );
}

#[test]
fn rejects_invalid_conflict_assignment_shapes() {
    let empty_update = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .on_conflict("id")
        .do_update_set([])
        .build()
        .unwrap();

    let err = ValidatedInsert::new(empty_update).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "assets" && message == "DO UPDATE requires at least one assignment")
    );

    let index_where_on_constraint = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .on_conflict_constraint("assets_pkey")
        .index_where(field("active").eq(true))
        .do_nothing()
        .build()
        .unwrap_err();

    assert!(
        matches!(index_where_on_constraint, Error::InvalidValue { field, message, .. } if field == "conflict" && message == "index_where can only be used with column conflict targets")
    );
}

#[test]
fn rejects_write_values_that_do_not_match_field_types() {
    let update = crate::update(dataset())
        .set("score", "42")
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();
    let err = ValidatedUpdate::new(update).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "score" && message == "expected integer, got string")
    );

    let insert = insert(dataset()).set("tags", [1]).build().unwrap();
    let err = ValidatedInsert::new(insert).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "tags" && message == "expected string, got i64")
    );

    let update = crate::update(dataset())
        .set("properties", Value::Array(vec![Value::F64(f64::INFINITY)]))
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();
    let err = ValidatedUpdate::new(update).unwrap_err();
    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "properties" && message == "non-finite numbers are not supported")
    );
}

#[test]
fn rejects_write_expressions_with_incompatible_types() {
    let update = crate::update(dataset())
        .set_expr("score", field("name").expr())
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    let err = ValidatedUpdate::new(update).unwrap_err();

    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "score" && message == "expected expression compatible with integer, got text")
    );

    let lossy_numeric = crate::update(dataset())
        .set_expr("score", 1.5)
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    let err = ValidatedUpdate::new(lossy_numeric).unwrap_err();

    assert!(
        matches!(err, Error::InvalidValue { field, message, .. } if field == "score" && message == "expected expression compatible with integer, got float")
    );
}

#[test]
fn rejects_write_raw_bind_mismatch_and_set_col_type_mismatch() {
    let raw_query = crate::update(dataset())
        .set_raw("name", crate::raw("upper(?)"))
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    assert!(matches!(
        ValidatedUpdate::new(raw_query).unwrap_err(),
        Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        }
    ));

    let col_query = crate::update(dataset())
        .set_col("score", "name")
        .filter(field("id").eq("10000000-0000-0000-0000-000000000001"))
        .build()
        .unwrap();

    assert!(matches!(
        ValidatedUpdate::new(col_query).unwrap_err(),
        Error::IncompatibleColumnTypes { left, right, .. } if left == "score" && right == "name"
    ));
}

#[test]
fn rejects_returning_hidden_fields() {
    let query = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .returning("blobName")
        .build()
        .unwrap();

    let err = ValidatedInsert::new(query).unwrap_err();
    assert!(matches!(err, Error::NotSelectable { field } if field == "blobName"));
}

#[test]
fn rejects_returning_expressions_that_leak_hidden_fields_or_duplicate_aliases() {
    let hidden = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .returning_expr(field("blobName").expr().alias("secret"))
        .build()
        .unwrap();
    let err = ValidatedInsert::new(hidden).unwrap_err();
    assert!(matches!(err, Error::NotSelectable { field } if field == "blobName"));

    let duplicate = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .returning(["id"])
        .returning_expr(field("name").expr().alias("id"))
        .build()
        .unwrap();
    let err = ValidatedInsert::new(duplicate).unwrap_err();
    assert!(matches!(err, Error::DuplicateOutputAlias { alias } if alias == "id"));
}

#[test]
fn rejects_insert_from_select_mixed_sources_and_unknown_targets() {
    let mixed = insert(dataset())
        .set("id", "10000000-0000-0000-0000-000000000001")
        .from_select(crate::select(dataset()).fields(["id"]).build())
        .build()
        .unwrap();

    assert!(matches!(
        ValidatedInsert::new(mixed).unwrap_err(),
        Error::InvalidValue { message, .. }
            if message == "cannot combine VALUES and SELECT insert sources"
    ));

    let target = Dataset::table("target").fields([Field::new("id", FieldType::Uuid)]);
    let unknown_target = insert(target)
        .from_select(crate::select(dataset()).fields(["name"]).build())
        .build()
        .unwrap();

    assert!(matches!(
        ValidatedInsert::new(unknown_target).unwrap_err(),
        Error::UnknownField { dataset, field } if dataset == "target" && field == "name"
    ));

    let target = Dataset::table("target").fields([Field::new("name", FieldType::Text)]);
    let source = Dataset::table("source").fields([Field::new("name", FieldType::Integer)]);
    let type_mismatch = insert(target)
        .from_select(crate::select(source).fields(["name"]).build())
        .build()
        .unwrap();

    assert!(matches!(
        ValidatedInsert::new(type_mismatch).unwrap_err(),
        Error::IncompatibleColumnTypes { left, left_type, right, right_type, .. }
            if left == "name" && left_type == "text" && right == "name" && right_type == "integer"
    ));

    let target = Dataset::table("target").fields([Field::new("score", FieldType::Integer)]);
    let expression_type_mismatch = insert(target)
        .from_select(
            crate::select(dataset())
                .select_expr(1.5.into_sql_expr().alias("score"))
                .build(),
        )
        .build()
        .unwrap();

    assert!(matches!(
        ValidatedInsert::new(expression_type_mismatch).unwrap_err(),
        Error::InvalidValue { field, message, .. }
            if field == "score" && message == "expected source column compatible with integer, got float"
    ));
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
fn reports_specific_custom_array_type_names_in_errors() {
    let query = crate::select(dataset())
        .filter(field("uintAmounts").starts_with("1"))
        .build();
    let err = ValidatedSelect::new(query).unwrap_err();
    assert!(matches!(
        err,
        Error::UnsupportedOperator { field, field_type, operator }
            if field == "uintAmounts"
                && field_type == "public.uint_256[]"
                && operator == "startsWith"
    ));
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
fn rejects_aggregate_fields_that_are_hidden_or_have_wrong_type() {
    let hidden = crate::select(dataset())
        .agg(count_field("blobName", "blobs"))
        .build();
    assert!(matches!(
        ValidatedSelect::new(hidden).unwrap_err(),
        Error::NotSelectable { field } if field == "blobName"
    ));

    let text_sum = crate::select(dataset()).agg(sum("name", "total")).build();
    assert!(matches!(
        ValidatedSelect::new(text_sum).unwrap_err(),
        Error::UnsupportedAggregateField { aggregate, field, field_type }
            if aggregate == "sum" && field == "name" && field_type == "text"
    ));

    let bool_avg = crate::select(dataset())
        .agg(avg("active", "avgActive"))
        .build();
    assert!(matches!(
        ValidatedSelect::new(bool_avg).unwrap_err(),
        Error::UnsupportedAggregateField { aggregate, field, field_type }
            if aggregate == "avg" && field == "active" && field_type == "bool"
    ));

    let unsortable_min = crate::select(dataset())
        .agg(min("tags", "firstTags"))
        .build();
    assert!(matches!(
        ValidatedSelect::new(unsortable_min).unwrap_err(),
        Error::NotSortable { field } if field == "tags"
    ));

    let unsortable_max = crate::select(dataset())
        .agg(max("properties", "lastProps"))
        .build();
    assert!(matches!(
        ValidatedSelect::new(unsortable_max).unwrap_err(),
        Error::NotSortable { field } if field == "properties"
    ));

    let numeric_string_agg = crate::select(dataset())
        .agg(string_agg("score", ",", "scores"))
        .build();
    assert!(matches!(
        ValidatedSelect::new(numeric_string_agg).unwrap_err(),
        Error::UnsupportedAggregateField { aggregate, field, field_type }
            if aggregate == "string_agg" && field == "score" && field_type == "integer"
    ));
}

#[test]
fn accepts_supported_aggregate_field_types() {
    let query = crate::select(dataset())
        .fields(["state"])
        .agg(sum("score", "totalScore"))
        .agg(avg("amount", "avgAmount"))
        .agg(min("createdAt", "firstSeen"))
        .agg(max("name", "lastName"))
        .agg(string_agg("state", ",", "states"))
        .group_by(["state"])
        .build();

    ValidatedSelect::new(query).unwrap();
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
