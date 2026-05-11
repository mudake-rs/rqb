use crate::{
    BoolExpr, BoolOp, FetchClause, Field, Join, JoinKind, MergeAction, MergeWhen, Meta, OpSet,
    OrderItem, Param, RawStmt, Select, SelectItem, SetOperator, SetQuery, Source, ValueExpr, cte,
    cte_ref, delete_from, insert, merge_into, raw, select, subquery, update,
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
    let subquery = crate::Stmt::Select(Box::new(Select {
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
        params: vec![crate::Param::typed(1_i32)],
        fields: vec![ID_META],
    };
    let stmt = crate::Stmt::Select(Box::new(Select {
        ctes: Vec::new(),
        source,
        joins: Vec::new(),
        distinct: false,
        distinct_on: Vec::new(),
        projection: vec![SelectItem {
            expr: ValueExpr::Param(crate::Param::typed(2_i32)),
            alias: Some("projected".to_owned()),
        }],
        filter: Some(BoolExpr::Compare {
            left: ID.expr(),
            op: BoolOp::Eq,
            right: ValueExpr::Param(crate::Param::typed(3_i32)),
        }),
        group_by: Vec::new(),
        having: None,
        order: vec![OrderItem::asc(ID)],
        limit: Some(crate::Param::typed(10_i64)),
        offset: Some(crate::Param::typed(5_i64)),
        fetch: None,
        lock: None,
    }));

    let params = stmt.params();

    assert_eq!(params.len(), 5);
    stmt.validate().unwrap();
}

#[test]
fn fetch_with_ties_requires_order_and_excludes_limit() {
    let without_order = crate::Stmt::Select(Box::new(Select {
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

    let with_limit = crate::Stmt::Select(Box::new(Select {
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
        limit: Some(crate::Param::typed(10_i64)),
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
fn set_query_fetch_with_ties_requires_order_and_excludes_limit() {
    let without_order = select(users()).column(ID).union(select(users()).column(ID));
    let err = without_order
        .clone()
        .fetch_first_with_ties(ValueExpr::from(10_i32))
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSelectShape { message }
            if message == "fetch with ties requires order_by"
    ));

    let with_limit = SetQuery {
        left: without_order.left,
        operator: SetOperator::Union,
        right: without_order.right,
        order: vec![OrderItem::asc(ID)],
        limit: Some(Param::typed(10_i64)),
        offset: None,
        fetch: Some(FetchClause {
            count: ValueExpr::from(10_i32),
            with_ties: false,
        }),
    };
    let err = with_limit.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSelectShape { message }
            if message == "limit and fetch cannot both be set"
    ));
}

#[test]
fn delete_requires_filter() {
    let stmt = crate::Stmt::Delete(Box::new(crate::Delete {
        ctes: Vec::new(),
        target: users(),
        using: Vec::new(),
        filter: None,
        returning: Vec::new(),
    }));

    assert!(matches!(
        stmt.validate().unwrap_err(),
        crate::Error::DeleteWithoutFilter
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

    let stmt = insert(source).set(ID.set(1)).returning_all();
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

    let stmt = insert(source)
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

#[test]
fn select_filter_and_having_chain_with_and_semantics() {
    let built = select(users())
        .column(ID)
        .filter(ID.gt(10))
        .filter(ID.lt(20))
        .group_by(ID)
        .having(ID.gt(0))
        .having(ID.lt(100))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\" FROM \"app_users\" WHERE (\"id\" > $1 AND \"id\" < $2) GROUP BY \"id\" HAVING (\"id\" > $3 AND \"id\" < $4)"
    );
}

#[test]
fn insert_from_select_rejects_projection_count_mismatch() {
    static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
    const EMAIL: Field<String> = Field::new(&EMAIL_META);
    let insert = insert(users())
        .column(ID)
        .column(EMAIL)
        .from_select(select(users()).column(ID));

    let err = insert.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidInsertShape { message }
            if message == "insert-select column count must match SELECT projection count"
    ));
}

#[test]
fn insert_from_select_requires_target_columns() {
    let insert = insert(users()).from_select(select(users()).column(ID));

    let err = insert.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::EmptyColumns { statement } if statement == "insert-select"
    ));
}

#[test]
fn insert_from_select_rejects_values_assignments() {
    let insert = insert(users())
        .column(ID)
        .set(ID.set(1))
        .from_select(select(users()).column(ID));

    let err = insert.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidInsertShape { message }
            if message == "insert-select cannot also contain VALUES assignments"
    ));
}

#[test]
fn empty_conflict_constraint_name_is_rejected() {
    let insert = insert(users())
        .set(ID.set(1))
        .on_conflict_constraint("")
        .do_nothing();

    let err = insert.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidInsertShape { message }
            if message == "conflict constraint name cannot be empty"
    ));
}

#[test]
fn column_conflict_target_supports_multiple_fields_and_predicate() {
    static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
    const EMAIL: Field<String> = Field::new(&EMAIL_META);

    let built = insert(users())
        .set(ID.set(1))
        .on_conflict((ID, EMAIL))
        .target_where(ID.gt(0))
        .do_nothing()
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"app_users\" (\"id\") VALUES ($1) ON CONFLICT (\"id\", \"email\") WHERE \"id\" > $2 DO NOTHING"
    );
}

#[test]
fn conflict_update_requires_assignments() {
    let insert = insert(users())
        .set(ID.set(1))
        .on_conflict(ID)
        .do_update_set([]);

    let err = insert.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::EmptyAssignments { statement } if statement == "conflict update"
    ));
}

#[test]
fn conflict_update_filter_is_validated() {
    let insert = insert(users())
        .set(ID.set(1))
        .on_conflict(ID)
        .do_update_set_where(
            [ID.set(2)],
            BoolExpr::Raw {
                sql: "? = ?".to_owned(),
                params: vec![Param::typed(1_i32)],
            },
        );

    assert!(matches!(
        insert.validate().unwrap_err(),
        crate::Error::RawBindMismatch {
            placeholders: 2,
            binds: 1
        }
    ));
}

#[test]
fn update_requires_assignments() {
    let update = update(users()).filter(ID.eq(1));

    let err = update.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::EmptyAssignments { statement } if statement == "update"
    ));
}

#[test]
fn write_targets_reject_subquery_sources() {
    let target = subquery(select(users()).column(ID), "u", ID);

    let err = insert(target).set(ID.set(1)).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidWriteTarget {
            statement: "insert",
            source_kind: "subquery",
        }
    ));
}

#[test]
fn cte_accepts_raw_statement_without_manual_stmt_variant() {
    let ids = cte("ids", raw("SELECT ?::int4 AS id").bind(1_i32), ID);

    assert!(select(ids.source()).with(ids).column(ID).validate().is_ok());
}

#[test]
fn update_and_delete_accept_empty_cte_lists_and_reject_duplicate_names() {
    update(users()).set(ID.set(1)).validate().unwrap();
    delete_from(users()).filter(ID.eq(1)).validate().unwrap();

    let first = cte("dupe", select(users()).column(ID), ID);
    let second = cte("dupe", select(users()).column(ID), ID);
    let err = update(users())
        .with(first)
        .with(second)
        .set(ID.set(1))
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidCteShape { name, message }
            if name == "dupe" && message == "duplicate CTE name"
    ));

    let first = cte("dupe", select(users()).column(ID), ID);
    let second = cte("dupe", select(users()).column(ID), ID);
    let err = delete_from(users())
        .with(first)
        .with(second)
        .filter(ID.eq(1))
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidCteShape { name, message }
            if name == "dupe" && message == "duplicate CTE name"
    ));
}

#[test]
fn subquery_sources_reject_write_statements_after_into_stmt_conversion() {
    let source = subquery(delete_from(users()).filter(ID.eq(1)), "deleted", ID);

    let err = select(source).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSelectShape { message }
            if message == "subquery source must be SELECT, set, or raw statement"
    ));
}

#[test]
fn set_queries_reject_write_statement_operands() {
    let set = select(users()).union(delete_from(users()).filter(ID.eq(1)));

    let err = set.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidSelectShape { message }
            if message == "set query operands must be SELECT, set, or raw statements"
    ));
}

#[test]
fn update_from_sources_are_validated() {
    let update = update(users()).set(ID.set(1)).from(Source::Raw {
        sql: "select ? as id".to_owned(),
        alias: "incoming".to_owned(),
        params: Vec::new(),
        fields: vec![ID_META],
    });

    assert!(matches!(
        update.validate().unwrap_err(),
        crate::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        }
    ));
}

#[test]
fn returning_expressions_are_validated_for_writes() {
    let insert = insert(users()).set(ID.set(1)).returning_item(SelectItem {
        expr: ValueExpr::Raw {
            sql: "?".to_owned(),
            params: Vec::new(),
        },
        alias: Some("broken".to_owned()),
    });

    assert!(matches!(
        insert.validate().unwrap_err(),
        crate::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        }
    ));
}

#[test]
fn duplicate_cte_names_are_rejected_before_rendering() {
    let first = cte("dupe", select(users()).column(ID), vec![ID_META]);
    let second = cte("dupe", select(users()).column(ID), vec![ID_META]);

    let err = select(users())
        .with(first)
        .with(second)
        .validate()
        .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidCteShape { name, message }
            if name == "dupe" && message == "duplicate CTE name"
    ));
}

#[test]
fn non_cross_join_requires_condition_but_cross_join_does_not() {
    let missing_inner = Join {
        kind: JoinKind::Inner,
        source: users(),
        on: None,
        lateral: false,
    };

    assert!(matches!(
        missing_inner.validate().unwrap_err(),
        crate::Error::MissingJoinCondition { join } if join == "JOIN"
    ));

    let cross = Join::cross(users());
    cross.validate().unwrap();
}

#[test]
fn merge_requires_at_least_one_action() {
    let merge = merge_into(
        users(),
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    );

    let err = merge.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidMergeShape { message }
            if message == "merge requires at least one action"
    ));
}

#[test]
fn merge_update_is_not_valid_for_when_not_matched() {
    let mut merge = merge_into(
        users(),
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    );
    merge.actions.push(MergeAction::Update {
        when: MergeWhen::NotMatched,
        condition: None,
        assignments: vec![ID.set(1)],
    });

    let err = merge.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidMergeShape { message }
            if message == "merge update is not valid for WHEN NOT MATCHED"
    ));
}

#[test]
fn merge_delete_is_not_valid_for_when_not_matched() {
    let mut merge = merge_into(
        users(),
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    );
    merge.actions.push(MergeAction::Delete {
        when: MergeWhen::NotMatched,
        condition: None,
    });

    let err = merge.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidMergeShape { message }
            if message == "merge delete is not valid for WHEN NOT MATCHED"
    ));
}

#[test]
fn merge_insert_is_not_valid_for_when_matched() {
    let mut merge = merge_into(
        users(),
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    );
    merge.actions.push(MergeAction::Insert {
        when: MergeWhen::Matched,
        condition: None,
        assignments: vec![ID.set(1)],
    });

    let err = merge.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidMergeShape { message }
            if message == "merge insert is not valid for WHEN MATCHED"
    ));
}

#[test]
fn merge_insert_is_not_valid_for_when_not_matched_by_source() {
    let mut merge = merge_into(
        users(),
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    );
    merge.actions.push(MergeAction::Insert {
        when: MergeWhen::NotMatchedBySource,
        condition: None,
        assignments: vec![ID.set(1)],
    });

    let err = merge.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::InvalidMergeShape { message }
            if message == "merge insert is not valid for WHEN NOT MATCHED BY SOURCE"
    ));
}

#[test]
fn merge_insert_requires_assignments() {
    let merge = merge_into(
        users(),
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    )
    .when_not_matched()
    .insert([]);

    let err = merge.validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::EmptyAssignments { statement } if statement == "merge-insert"
    ));
}

#[test]
fn merge_rejects_non_table_target() {
    let target = cte_ref("target_cte", vec![ID_META]);
    let merge = merge_into(
        target,
        users().alias("source"),
        ID.eq_field(ID.at("source")),
    )
    .when_matched()
    .do_nothing();

    assert!(matches!(
        merge.validate().unwrap_err(),
        crate::Error::InvalidWriteTarget {
            statement: "merge",
            source_kind: "cte",
        }
    ));
}

#[test]
fn merge_validates_using_source_and_on_condition() {
    let using = Source::Raw {
        sql: "select ? as id".to_owned(),
        alias: "source".to_owned(),
        params: Vec::new(),
        fields: vec![ID_META],
    };
    let merge = merge_into(users(), using, ID.eq_field(ID.at("source")))
        .when_matched()
        .do_nothing();

    assert!(matches!(
        merge.validate().unwrap_err(),
        crate::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        }
    ));
}

#[test]
fn delete_using_preserves_required_filter() {
    let built = delete_from(users())
        .using(users().alias("old"))
        .filter(ID.eq_field(ID.at("old")))
        .returning(ID)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "DELETE FROM \"app_users\" USING \"app_users\" AS \"old\" WHERE \"id\" = \"old\".\"id\" RETURNING \"id\""
    );
}

#[test]
fn raw_statement_validates_bind_count() {
    let err = raw("select ? + ?").bind(1_i32).validate().unwrap_err();

    assert!(matches!(
        err,
        crate::Error::RawBindMismatch {
            placeholders: 2,
            binds: 1
        }
    ));

    let err = RawStmt {
        sql: "select ?".to_owned(),
        params: Vec::new(),
    }
    .validate()
    .unwrap_err();

    assert!(matches!(
        err,
        crate::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        }
    ));
}
