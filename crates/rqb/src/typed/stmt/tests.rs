use crate::typed::{
    BoolExpr, BoolOp, FetchClause, Field, Insert, Meta, OpSet, OrderItem, Select, SelectItem,
    Source, ValueExpr,
};

static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
static ID_FIELDS: [&Meta; 1] = [&ID_META];
const ID: Field<i32> = Field::new(&ID_META);

fn users() -> Source {
    Source::Table {
        name: "app_users",
        alias: None,
        fields: &ID_FIELDS,
    }
}

#[test]
fn subquery_value_expr_collects_nested_params_at_expression_position() {
    let subquery = crate::typed::Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source: users(),
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: vec![SelectItem {
            expr: ID.expr(),
            alias: None,
        }],
        filter: Some(ID.eq(10)),
        group_by: Vec::new(),
        having: None,
        order: Vec::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    }));
    let outer = ValueExpr::Subquery(Box::new(subquery));

    let mut params = Vec::new();
    outer.collect_params(&mut params);

    assert_eq!(params.len(), 1);
}

#[test]
fn select_params_follow_sql_text_order() {
    let source = Source::Raw {
        sql: "select ?::int4 as id".to_owned(),
        alias: "generated".to_owned(),
        params: vec![crate::typed::Param::typed(1_i32)],
        fields: vec![ID_META],
    };
    let stmt = crate::typed::Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source,
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: vec![SelectItem {
            expr: ValueExpr::Param(crate::typed::Param::typed(2_i32)),
            alias: Some("projected".to_owned()),
        }],
        filter: Some(BoolExpr::Compare {
            left: ID.expr(),
            op: BoolOp::Eq,
            right: ValueExpr::Param(crate::typed::Param::typed(3_i32)),
        }),
        group_by: Vec::new(),
        having: None,
        order: vec![OrderItem::asc(ID)],
        limit: Some(crate::typed::Param::typed(10_i64)),
        offset: Some(crate::typed::Param::typed(5_i64)),
        fetch: None,
        lock: None,
    }));

    let params = stmt.params();

    assert_eq!(params.len(), 5);
    stmt.validate().unwrap();
}

#[test]
fn fetch_with_ties_requires_order_and_excludes_limit() {
    let without_order = crate::typed::Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source: users(),
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: Vec::new(),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order: Vec::new(),
        limit: None,
        offset: None,
        fetch: Some(FetchClause {
            count: ValueExpr::from(10_i32),
            with_ties: true,
        }),
        lock: None,
    }));

    assert!(matches!(
        without_order.validate().unwrap_err(),
        crate::Error::InvalidSelectShape { message }
            if message == "fetch with ties requires order_by"
    ));

    let with_limit = crate::typed::Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source: users(),
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: Vec::new(),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order: vec![OrderItem::asc(ID)],
        limit: Some(crate::typed::Param::typed(10_i64)),
        offset: None,
        fetch: Some(FetchClause {
            count: ValueExpr::from(10_i32),
            with_ties: false,
        }),
        lock: None,
    }));

    assert!(matches!(
        with_limit.validate().unwrap_err(),
        crate::Error::InvalidSelectShape { message }
            if message == "limit and fetch cannot both be set"
    ));
}

#[test]
fn delete_requires_filter() {
    let stmt = crate::typed::Stmt::Delete(Box::new(crate::typed::Delete {
        target: users(),
        using: Vec::new(),
        filter: None,
        returning: Vec::new(),
    }));

    assert!(matches!(
        stmt.validate().unwrap_err(),
        crate::Error::TypedDeleteWithoutFilter
    ));
}

#[test]
fn returning_all_uses_source_fields_with_api_aliases() {
    static NAME_META: Meta = Meta::new("displayName", "display_name", "text");
    static RETURN_FIELDS: [&Meta; 2] = [&ID_META, &NAME_META];
    let source = Source::Table {
        name: "public.users",
        alias: None,
        fields: &RETURN_FIELDS,
    };

    let stmt = Insert::into(source).set(ID.set(1)).returning_all();
    let built = stmt.build().unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"users\" (\"id\") VALUES ($1) RETURNING \"id\", \"display_name\" AS \"displayName\""
    );
}

#[test]
fn returning_all_replaces_existing_returning_fields() {
    static NAME_META: Meta = Meta::new("displayName", "display_name", "text");
    static RETURN_FIELDS: [&Meta; 2] = [&ID_META, &NAME_META];
    let source = Source::Table {
        name: "public.users",
        alias: None,
        fields: &RETURN_FIELDS,
    };

    let stmt = Insert::into(source)
        .set(ID.set(1))
        .returning(ID)
        .returning_all()
        .returning_all();
    let built = stmt.build().unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"users\" (\"id\") VALUES ($1) RETURNING \"id\", \"display_name\" AS \"displayName\""
    );
}
