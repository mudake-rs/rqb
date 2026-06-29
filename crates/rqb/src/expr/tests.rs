use uuid::Uuid;

use super::escaped_like_pattern;
use crate::{
    BoolExpr, BoolOp, Field, IntoColumn, IntoFieldRef, JsonKind, Meta, OpSet, OrderItem, Param,
    ValueExpr, ValueOp, raw_expr, raw_predicate, row,
};

#[test]
fn field_t_erases_to_bool_expr_with_sqlx_param() {
    static ID_META: Meta = Meta::new("id", "id", "uuid")
        .ops(OpSet::equality())
        .json(JsonKind::Uuid);
    static ID_FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<Uuid> = Field::new(&ID_META);

    let built = crate::select(crate::table("public.users", &ID_FIELDS))
        .filter(ID.eq(Uuid::nil()))
        .build()
        .unwrap();

    assert_eq!(built.params.len(), 1);
    assert!(built.params.debug_names()[0].ends_with("uuid::Uuid"));
}

#[test]
fn field_is_copy_without_requiring_t_to_be_copy() {
    static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::text());
    const EMAIL: Field<String> = Field::new(&EMAIL_META);

    let field = EMAIL;
    let _first = field.expr();
    let _second = field.expr();
}

#[test]
fn operator_validation_uses_meta_not_rust_type_traits() {
    static PAYLOAD_META: Meta = Meta::new("payload", "payload", "jsonb")
        .json(JsonKind::Jsonb)
        .ops(OpSet::equality());
    const PAYLOAD: Field<serde_json::Value> = Field::new(&PAYLOAD_META);

    let err = PAYLOAD
        .gt(serde_json::json!({ "n": 1 }))
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "payload" && err.operator == "gt"
    ));
}

#[test]
fn value_expr_is_separate_from_bool_expr() {
    static EMAIL_META: Meta = Meta::new("email", "email", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    const EMAIL: Field<String> = Field::new(&EMAIL_META);

    let lower = ValueExpr::Function {
        name: "lower",
        args: vec![EMAIL.expr()],
    };
    let filter = crate::BoolExpr::Compare {
        left: lower,
        op: BoolOp::Eq,
        right: ValueExpr::Param(Param::typed("egor@example.com".to_owned())),
    };

    filter.validate().unwrap();
}

#[test]
fn null_value_expr_validates_without_params() {
    let expr = crate::null();
    expr.validate().unwrap();

    assert!(matches!(expr, ValueExpr::Null));
}

#[test]
fn static_sql_literal_validates_without_params() {
    let expr = crate::literal("day");
    expr.validate().unwrap();

    assert!(matches!(expr, ValueExpr::SqlLiteral("day")));
}

#[test]
fn aggregate_modifiers_reject_non_aggregate_expressions() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::text());
    const ID: Field<i32> = Field::new(&ID_META);
    const EMAIL: Field<String> = Field::new(&EMAIL_META);

    let filter_err = crate::lower(EMAIL)
        .aggregate_filter(ID.gt(10))
        .validate()
        .unwrap_err();
    assert!(matches!(
        filter_err,
        crate::Error::InvalidAggregateModifier {
            modifier: "aggregate_filter"
        }
    ));

    let order_err = crate::lower(EMAIL)
        .aggregate_order_desc(ID)
        .validate()
        .unwrap_err();
    assert!(matches!(
        order_err,
        crate::Error::InvalidAggregateModifier {
            modifier: "aggregate_order_by"
        }
    ));

    let over_err = crate::lower(EMAIL)
        .over(crate::window())
        .validate()
        .unwrap_err();
    assert!(matches!(
        over_err,
        crate::Error::InvalidAggregateModifier { modifier: "over" }
    ));

    let distinct_over_err = crate::count_distinct(EMAIL)
        .over(crate::window())
        .validate()
        .unwrap_err();
    assert!(matches!(
        distinct_over_err,
        crate::Error::InvalidAggregateModifier { modifier: "over" }
    ));

    let aggregate_order_over_err = crate::array_agg(EMAIL)
        .over(crate::window())
        .aggregate_order_desc(ID)
        .validate()
        .unwrap_err();
    assert!(matches!(
        aggregate_order_over_err,
        crate::Error::InvalidAggregateModifier {
            modifier: "aggregate_order_by"
        }
    ));
}

#[test]
fn value_ops_expose_sql_tokens() {
    assert_eq!(ValueOp::Add.as_sql(), "+");
    assert_eq!(ValueOp::Sub.as_sql(), "-");
    assert_eq!(ValueOp::Mul.as_sql(), "*");
    assert_eq!(ValueOp::Div.as_sql(), "/");
    assert_eq!(ValueOp::Custom("<->").as_sql(), "<->");
}

#[test]
fn and_pair_flattens_nonempty_groups_without_hiding_empty_groups() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let flattened = BoolExpr::and_pair(BoolExpr::and([ID.gt(1), ID.lt(10)]), ID.ne(5));
    assert!(matches!(flattened, BoolExpr::And(ref exprs) if exprs.len() == 3));
    flattened.validate().unwrap();

    let invalid = BoolExpr::and_pair(BoolExpr::and([]), ID.ne(5));
    assert!(matches!(
        invalid.validate().unwrap_err(),
        crate::Error::EmptyLogical { logical } if logical == "and"
    ));
}

#[test]
fn and_or_free_functions_build_logical_groups() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let and_group = crate::and([crate::and([ID.gt(1), ID.lt(10)]), ID.ne(5)]);
    let or_group = crate::or([crate::or([ID.eq(1), ID.eq(2)]), ID.eq(3)]);

    assert!(matches!(and_group, BoolExpr::And(ref exprs) if exprs.len() == 3));
    assert!(matches!(or_group, BoolExpr::Or(ref exprs) if exprs.len() == 3));

    let invalid = crate::and([crate::and([]), ID.ne(5)]);
    assert!(matches!(
        invalid.validate().unwrap_err(),
        crate::Error::EmptyLogical { logical } if logical == "and"
    ));
}

#[test]
fn exists_not_and_value_comparison_helpers_avoid_manual_ast_construction() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<i32> = Field::new(&ID_META);

    let subquery = crate::select(crate::table("public.users", &FIELDS))
        .column(ID)
        .filter(ID.gt(10));
    let exists = crate::exists(subquery);
    let not_exists = crate::not(exists.clone());
    let aggregate_filter = crate::count(ID).gt(0_i64);

    assert!(matches!(exists, BoolExpr::Exists(_)));
    assert!(matches!(not_exists, BoolExpr::Not(_)));
    assert!(matches!(
        aggregate_filter,
        BoolExpr::Compare {
            op: crate::BoolOp::Gt,
            ..
        }
    ));
}

#[test]
fn scalar_subquery_rejects_write_statement_context() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<i32> = Field::new(&ID_META);

    let expr = crate::scalar_subquery(
        crate::delete_from(crate::table("public.users", &FIELDS)).filter(ID.eq(1)),
    );
    let err = expr.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSelectShape { message }
            if message == "scalar subquery must be SELECT, set, or raw statement"
    ));
}

#[test]
fn borrowed_field_refs_and_raw_metadata_convert_to_value_shapes() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static EMAIL_META: Meta = Meta::new("email", "email_address", "text").ops(OpSet::text());
    const ID: Field<i32> = Field::new(&ID_META);

    let id = ID.at("u");
    let expr = ValueExpr::from(&id);
    let item = (&id).into_column().items.pop().unwrap();
    let raw_meta_item = EMAIL_META.into_column().items.pop().unwrap();

    assert!(matches!(
        expr,
        ValueExpr::Field {
            qualifier: Some(ref qualifier),
            ..
        } if qualifier == "u"
    ));
    assert_eq!(item.alias.as_deref(), Some("u_id"));
    assert_eq!(raw_meta_item.alias.as_deref(), Some("email"));
}

#[test]
fn empty_or_group_is_invalid_like_empty_and_group() {
    let err = BoolExpr::or([]).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::EmptyLogical { logical } if logical == "or"
    ));
}

#[test]
fn meta_defaults_to_no_typed_operators() {
    static SCORE_META: Meta = Meta::new("score", "score", "int4");
    const SCORE: Field<i32> = Field::new(&SCORE_META);

    let err = SCORE.eq(10).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "score" && err.operator == "eq"
    ));
}

#[test]
fn raw_predicate_validates_bind_count() {
    let err = raw_predicate("score > ? and active = ?", [Param::typed(10_i32)])
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::RawBindMismatch {
            placeholders: 2,
            binds: 1
        }
    ));
}

#[test]
fn field_at_produces_qualified_value_expr() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let expr = ID.at("u").expr();

    assert!(matches!(
        expr,
        ValueExpr::Field {
            meta,
            qualifier: Some(ref qualifier),
        } if meta == ID_META && qualifier == "u"
    ));
}

#[test]
fn into_field_ref_accepts_fields_and_existing_refs() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let from_field = ID.into_field_ref();
    let from_ref = ID.at("orders").into_field_ref();

    assert_eq!(from_field.qualifier, None);
    assert_eq!(from_ref.qualifier.as_deref(), Some("orders"));
}

#[test]
fn empty_in_list_is_false_and_empty_not_in_is_true() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    assert!(matches!(
        ID.in_list(Vec::<i32>::new()),
        BoolExpr::Constant(false)
    ));
    assert!(matches!(
        ID.not_in(Vec::<i32>::new()),
        BoolExpr::Constant(true)
    ));
}

#[test]
fn ordered_predicates_reject_equality_only_fields() {
    static STATUS_META: Meta = Meta::new("status", "status", "text").ops(OpSet::equality());
    const STATUS: Field<String> = Field::new(&STATUS_META);

    let err = STATUS.gt("paid".to_owned()).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "status" && err.operator == "gt"
    ));
}

#[test]
fn text_predicates_reject_non_text_fields() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let err = BoolExpr::Like {
        expr: ID.expr(),
        pattern: ValueExpr::from("42"),
        case_insensitive: true,
        negated: false,
        escape: false,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "id" && err.operator == "like"
    ));

    let err = BoolExpr::Regex {
        expr: ID.expr(),
        pattern: ValueExpr::from("^[0-9]+$"),
        case_insensitive: false,
        negated: false,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "id" && err.operator == "regex"
    ));
}

#[test]
fn jsonb_infix_predicates_reject_non_jsonb_fields() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let err = BoolExpr::Infix {
        left: ID.expr(),
        op: "?",
        right: ValueExpr::from("id"),
        negated: false,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "id" && err.operator == "?"
    ));
}

#[test]
fn array_predicates_reject_non_array_fields() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let err = BoolExpr::ArrayIsEmpty {
        expr: ID.expr(),
        negated: false,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "id" && err.operator == "array_empty"
    ));
}

#[test]
fn any_predicate_rejects_non_array_operands() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let err = BoolExpr::Any {
        value: ValueExpr::from(1_i32),
        array: ID.expr(),
        negated: false,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "id" && err.operator == "any"
    ));
}

#[test]
fn range_infix_predicates_reject_plain_scalar_fields() {
    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    const ID: Field<i32> = Field::new(&ID_META);

    let err = BoolExpr::Infix {
        left: ID.expr(),
        op: "&&",
        right: ValueExpr::from(10_i32),
        negated: false,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "id" && err.operator == "&&"
    ));
}

#[test]
fn escaped_like_pattern_escapes_wildcards_and_backslashes() {
    assert_eq!(escaped_like_pattern("paid", "%", "%"), "%paid%");
    assert_eq!(
        escaped_like_pattern("50%_\\done", "%", "%"),
        "%50\\%\\_\\\\done%"
    );
}

#[test]
fn raw_value_expr_validates_bind_count() {
    let err = raw_expr("lower(?) || ?", [Param::typed("email".to_owned())])
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::RawBindMismatch {
            placeholders: 2,
            binds: 1
        }
    ));
}

#[test]
fn case_expr_validates_nested_conditions_before_rendering() {
    let expr = ValueExpr::Case {
        branches: vec![(
            BoolExpr::Raw {
                sql: "? = ?".to_owned(),
                params: vec![Param::typed(1_i32)],
            },
            ValueExpr::from("one"),
        )],
        else_: Some(Box::new(ValueExpr::from("other"))),
    };

    assert!(matches!(
        expr.validate().unwrap_err(),
        crate::Error::RawBindMismatch {
            placeholders: 2,
            binds: 1
        }
    ));
}

#[test]
fn value_expr_array_row_subscript_slice_validate_nested_values() {
    static TAGS_META: Meta = Meta::new("tags", "tags", "text[]").ops(OpSet::equality());
    const TAGS: Field<Vec<String>> = Field::new(&TAGS_META);

    ValueExpr::Array(vec![ValueExpr::from(1_i32), ValueExpr::from(2_i32)])
        .validate()
        .unwrap();
    ValueExpr::Row(vec![ValueExpr::from("id"), TAGS.element(1)])
        .validate()
        .unwrap();
    TAGS.slice(Some(1), Some(3)).validate().unwrap();
}

#[test]
fn row_compare_rejects_mismatched_arity_before_rendering() {
    let err = row((ValueExpr::from(1_i32), ValueExpr::from(2_i32)))
        .eq(row((
            ValueExpr::from(1_i32),
            ValueExpr::from(2_i32),
            ValueExpr::from(3_i32),
        )))
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidRowShape { left: 2, right: 3 }
    ));
}

#[test]
fn boolean_tests_validate_on_boolean_fields() {
    static ACTIVE_META: Meta = Meta::new("active", "active", "bool").ops(OpSet::equality());
    const ACTIVE: Field<bool> = Field::new(&ACTIVE_META);

    ACTIVE.is_true().validate().unwrap();
    ACTIVE.is_not_false().validate().unwrap();
    ACTIVE.is_unknown().validate().unwrap();
}

#[test]
fn ordered_set_aggregate_requires_within_group_ordering() {
    let err = ValueExpr::OrderedSetAggregate {
        name: "percentile_cont",
        args: vec![ValueExpr::from(0.5_f64)],
        within_group: Vec::new(),
        filter: None,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidOperator(err)
            if err.field == "ordered_set_aggregate" && err.operator == "within_group"
    ));
}

#[test]
fn ordered_set_aggregate_validates_within_group_ordering() {
    static TOTAL_META: Meta = Meta::new("total", "total_cents", "int8").ops(OpSet::ordered());
    const TOTAL: Field<i64> = Field::new(&TOTAL_META);

    let expr = ValueExpr::OrderedSetAggregate {
        name: "percentile_cont",
        args: vec![ValueExpr::from(0.5_f64)],
        within_group: vec![OrderItem::asc(TOTAL)],
        filter: Some(Box::new(TOTAL.gt(0))),
    };

    expr.validate().unwrap();
}
