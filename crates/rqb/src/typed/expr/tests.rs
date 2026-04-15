use uuid::Uuid;

use crate::typed::{BoolExpr, BoolOp, Field, JsonKind, Meta, OpSet, Param, Params, ValueExpr};

#[test]
fn field_t_erases_to_bool_expr_with_sqlx_param() {
    static ID_META: Meta = Meta::new("id", "id", "uuid")
        .ops(OpSet::equality())
        .json(JsonKind::Uuid);
    const ID: Field<Uuid> = Field::new(&ID_META);

    let expr = ID.eq(Uuid::nil());
    expr.validate().unwrap();

    let mut raw_params = Vec::new();
    expr.collect_params(&mut raw_params);
    let params = Params::from_vec(raw_params);

    assert_eq!(params.len(), 1);
    assert!(params.debug_names()[0].ends_with("uuid::Uuid"));
}

#[test]
fn field_is_copy_without_requiring_t_to_be_copy() {
    static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
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
        crate::Error::InvalidTypedOperator { field, operator }
            if field == "payload" && operator == "gt"
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
    let filter = crate::typed::BoolExpr::Compare {
        left: lower,
        op: BoolOp::Eq,
        right: ValueExpr::Param(Param::typed("egor@example.com".to_owned())),
    };

    filter.validate().unwrap();
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
        crate::Error::EmptyTypedLogical { logical } if logical == "and"
    ));
}

#[test]
fn meta_defaults_to_no_typed_operators() {
    static SCORE_META: Meta = Meta::new("score", "score", "int4");
    const SCORE: Field<i32> = Field::new(&SCORE_META);

    let err = SCORE.eq(10).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidTypedOperator { field, operator }
            if field == "score" && operator == "eq"
    ));
}

#[test]
fn raw_predicate_validates_bind_count() {
    let err = crate::typed::BoolExpr::Raw {
        sql: "score > ? and active = ?".to_owned(),
        params: vec![Param::typed(10_i32)],
    }
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
