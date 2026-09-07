use rqb::LockMode;
use rqb::prelude::*;

rqb::schema! {
    table public.state_rows {
        id: int4 = i32,
        value: int4 = i32,
    }
}
use state_rows as t;

#[derive(Insertable)]
#[rqb(table = t)]
struct Row {
    id: i32,
    value: i32,
}

fn configured_insert() -> InsertRow {
    insert(t::table())
        .with(cte("ids", select(t::table()).column(t::ID), t::ID))
        .returning(t::ID)
        .on_conflict(t::ID)
        .do_update_excluded(t::VALUE)
}

#[test]
fn insert_clauses_preserve_row_or_batch_composition() {
    let row = Row { id: 1, value: 2 };
    let single: InsertRow = configured_insert()
        .values(&row)
        .set_many((t::VALUE.set(3), t::VALUE.set(4)))
        .set_if(false, t::ID.set(99))
        .set_option(Some(5), |v| t::VALUE.set(v));
    single.validate().unwrap();
    let before = single.build().unwrap();
    let completed: Insert = single.clone().into();
    assert_eq!(before.sql, completed.build().unwrap().sql);
    assert_eq!(before.sql, Stmt::from(single).build().unwrap().sql);
    assert_eq!(before.params.len(), 2);
    let batch: Insert = configured_insert()
        .values_many([&row, &Row { id: 11, value: 23 }])
        .unwrap();
    assert!(
        batch
            .build()
            .unwrap()
            .sql
            .contains("FROM (VALUES ($1, $2), ($3, $4))")
    );
    assert!(batch.build().unwrap().sql.ends_with(
        "ON CONFLICT (\"id\") DO UPDATE SET \"value\" = EXCLUDED.\"value\" RETURNING \"id\""
    ));
    let replaced = batch
        .default_values()
        .from_select(t::ID, select(t::table()).column(t::ID));
    assert!(replaced.build().unwrap().sql.contains("SELECT \"id\""));
    let cte = cte("created", configured_insert().values(row), t::ID);
    assert!(select(cte.source()).with(cte).build().is_ok());
}

#[test]
fn conflict_finishers_preserve_each_concrete_host() {
    macro_rules! finishes {
        ($q:expr) => {{
            let q = $q;
            [
                q.clone().do_nothing(),
                q.clone().do_update_set([t::VALUE.set(1)]),
                q.clone().do_update_excluded(t::VALUE),
                q.clone()
                    .do_update_set_where([t::VALUE.set(1)], t::ID.eq(1)),
                q.do_update_excluded_where(t::VALUE, t::ID.eq(1)),
            ]
        }};
    }
    let row = insert(t::table()).returning(t::ID);
    let columns: [InsertRow; 5] = finishes!(
        row.clone()
            .on_conflict(t::ID)
            .column(t::VALUE)
            .target_where(t::ID.gt(0))
    );
    let constraints: [InsertRow; 5] =
        finishes!(row.clone().on_conflict_constraint("state_rows_pkey"));
    for q in columns.into_iter().chain(constraints) {
        assert!(q.values(Row { id: 1, value: 2 }).build().is_ok());
    }
    let completed: Insert = row.default_values();
    let columns: [Insert; 5] = finishes!(completed.clone().on_conflict(t::ID));
    let constraints: [Insert; 5] = finishes!(completed.on_conflict_constraint("state_rows_pkey"));
    for q in columns.into_iter().chain(constraints) {
        assert!(q.build().is_ok());
    }
}

#[test]
fn repeated_same_slot_clauses_follow_documented_precedence() {
    let q = insert(t::table())
        .set(t::ID.set(1))
        .on_conflict(t::ID)
        .do_nothing()
        .on_conflict_constraint("state_rows_pkey")
        .do_update_excluded(t::VALUE)
        .build()
        .unwrap();
    assert!(!q.sql.contains("DO NOTHING"));
    assert!(
        q.sql
            .contains("ON CONFLICT ON CONSTRAINT \"state_rows_pkey\"")
    );
    let first = select(t::table())
        .distinct()
        .distinct_on(t::ID)
        .build()
        .unwrap();
    let last = select(t::table())
        .distinct_on(t::ID)
        .distinct()
        .build()
        .unwrap();
    assert_eq!(first.sql, last.sql);
}

#[test]
fn lock_updates_preserve_independent_properties() {
    let base = || select(t::alias("a")).cross_join(t::alias("b"));
    for mode in [
        LockMode::Update,
        LockMode::NoKeyUpdate,
        LockMode::Share,
        LockMode::KeyShare,
    ] {
        let left = base()
            .skip_locked()
            .lock_of(LockMode::Update, "a")
            .lock_of(LockMode::Share, "b")
            .lock_of(LockMode::KeyShare, "a")
            .lock(mode);
        let right = base().lock_of(mode, "a").lock_of(mode, "b").skip_locked();
        assert_eq!(left.build().unwrap().sql, right.build().unwrap().sql);
        assert!(
            left.build()
                .unwrap()
                .sql
                .ends_with(" OF \"a\", \"b\" SKIP LOCKED")
        );
        assert!(
            left.nowait()
                .build()
                .unwrap()
                .sql
                .ends_with(" OF \"a\", \"b\" NOWAIT")
        );
        assert!(
            base()
                .nowait()
                .lock(mode)
                .skip_locked()
                .build()
                .unwrap()
                .sql
                .ends_with(" SKIP LOCKED")
        );
    }
}

#[test]
fn empty_lists_validate_capability_before_boolean_identity() {
    static DENIED: Meta = Meta::col("id", "int4")
        .ops(OpSet::none())
        .json(JsonKind::Integer);
    static DENIED_FIELDS: [&Meta; 1] = [&DENIED];
    const ID: Field<i32> = Field::new(&DENIED);
    for qualified in [false, true] {
        for empty in [false, true] {
            for negated in [false, true] {
                let values = if empty { vec![] } else { vec![1_i32] };
                let predicate = |field: Field<i32>| match (qualified, negated) {
                    (true, true) => field.at("t").not_in(values.clone()),
                    (true, false) => field.at("t").in_list(values.clone()),
                    (false, true) => field.not_in(values.clone()),
                    (false, false) => field.in_list(values.clone()),
                };
                let denied = predicate(ID);
                assert!(matches!(
                    select(t::table()).filter(denied.clone()).build(),
                    Err(Error::InvalidOperator(_))
                ));
                assert!(matches!(
                    delete_from(t::table()).filter(denied).build(),
                    Err(Error::InvalidOperator(_))
                ));
                let allowed = predicate(t::ID);
                let q = select(t::alias("t")).filter(allowed).build().unwrap();
                if empty {
                    assert!(
                        q.sql
                            .ends_with(if negated { "WHERE TRUE" } else { "WHERE FALSE" })
                    );
                    assert!(q.params.is_empty());
                } else {
                    assert_eq!(q.params.len(), 1);
                }
            }
        }
    }
    for op in ["in", "notIn"] {
        let req: SearchRequest = serde_json::from_value(serde_json::json!({
            "filter": {"field": "id", "operator": op, "value": []}
        }))
        .unwrap();
        assert!(matches!(
            select(table("denied", &DENIED_FIELDS)).apply_search(req),
            Err(Error::InvalidSearchOperator(_))
        ));
    }

    for negated in [false, true] {
        let mut empty = if negated {
            t::ID.not_in([] as [i32; 0])
        } else {
            t::ID.in_list([] as [i32; 0])
        };
        // Public AST matching permits replacing the operand even without a constructor.
        let BoolExpr::InList { expr, .. } = &mut empty else {
            panic!("empty IN must retain its operand until validation");
        };
        *expr = raw_expr("?", [Param::typed(999_i32)]);
        let q = select(t::table())
            .filter(and([t::ID.eq(17), empty.clone(), t::VALUE.eq(29)]))
            .build()
            .unwrap();
        assert!(q.sql.ends_with(&format!(
            "WHERE (\"id\" = $1 AND {} AND \"value\" = $2)",
            if negated { "TRUE" } else { "FALSE" }
        )));
        assert_eq!(q.params.len(), 2);
        if let BoolExpr::InList { expr, .. } = &mut empty {
            *expr = raw_expr("?", []);
        }
        assert!(matches!(
            select(t::table()).filter(empty).build(),
            Err(Error::RawBindMismatch {
                placeholders: 1,
                binds: 0
            })
        ));
    }
}

#[test]
fn empty_metadata_default_projection_is_root_only() {
    let roots = [
        (
            table("public.state_rows", &[]),
            "\"public\".\"state_rows\".*",
        ),
        (
            view("public.state_rows", &[]),
            "\"public\".\"state_rows\".*",
        ),
        (table("public.state_rows", &[]).alias("r"), "\"r\".*"),
        (raw_source("SELECT 1 AS value", "r", [], ()), "\"r\".*"),
        (subquery(raw("SELECT 1 AS value"), "r", ()), "\"r\".*"),
        (
            cte("root", raw("SELECT 1 AS value"), ()).source(),
            "\"root\".*",
        ),
        (
            cte("root", raw("SELECT 1 AS value"), ())
                .source()
                .alias("r"),
            "\"r\".*",
        ),
        (
            rqb::function_source("generate_series", vec![1_i32.into(), 2_i32.into()], "r", ())
                .into(),
            "\"r\".*",
        ),
        (view("public.state_rows", &[]).alias("r"), "\"r\".*"),
    ];
    for (root, wildcard) in roots {
        let joined = select(root.clone()).cross_join(raw_source("SELECT 2", "j", [], ()));
        assert!(
            joined
                .build()
                .unwrap()
                .sql
                .starts_with(&format!("SELECT {wildcard} FROM "))
        );
        assert!(
            select(root)
                .expr(1_i32)
                .build()
                .unwrap()
                .sql
                .starts_with("SELECT $1 FROM ")
        );
    }
}

#[test]
fn aggregate_object_keys_use_api_metadata_and_explicit_overrides() {
    static VALUE: Meta = Meta::new("apiValue", "db_value", "int4").ops(OpSet::ordered());
    const FIELD: Field<i32> = Field::new(&VALUE);
    let q = select(raw_source("SELECT 1 AS db_value", "r", [], [&VALUE]))
        .expr(jsonb_agg_object!(
            FIELD,
            FIELD.at("r"),
            ("custom", FIELD.at("r"))
        ))
        .build()
        .unwrap();
    assert_eq!(rqb::__jsonb_object_pair(FIELD).0, "apiValue");
    assert_eq!(rqb::__jsonb_object_pair(FIELD.at("r")).0, "apiValue");
    assert_eq!(rqb::__jsonb_object_pair(("custom", FIELD)).0, "custom");
    assert!(
        q.sql
            .contains("$1, \"db_value\", $2, \"r\".\"db_value\", $3, \"r\".\"db_value\"")
    );
}

fn missing<T>(result: rqb::Result<T>, expected: &str) {
    assert!(
        matches!(result, Err(Error::WriteWithoutReturning { statement }) if statement == expected)
    );
}

#[tokio::test]
async fn every_modeled_write_fetch_checks_returning_before_io() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused/unused")
        .unwrap();
    pool.close().await;
    macro_rules! check {
        ($q:expr, $name:literal) => {{
            let q = $q;
            assert!(q.build().is_ok());
            missing(q.fetch_all(&pool).await, $name);
            missing(q.fetch_one(&pool).await, $name);
            missing(q.fetch_optional(&pool).await, $name);
            missing(q.fetch_all_as::<(i32,)>(&pool).await, $name);
            missing(q.fetch_one_as::<(i32,)>(&pool).await, $name);
            missing(q.fetch_optional_as::<(i32,)>(&pool).await, $name);
            missing(q.fetch_scalar::<i32>(&pool).await, $name);
            missing(q.fetch_one_scalar::<i32>(&pool).await, $name);
            missing(q.fetch_optional_scalar::<i32>(&pool).await, $name);
            missing(q.clone().fetch_stream_pool(pool.clone()), $name);
            missing(
                q.clone().fetch_stream_pool_as::<(i32,)>(pool.clone()),
                $name,
            );
            missing(
                q.clone().fetch_stream_pool_scalar::<i32>(pool.clone()),
                $name,
            );
            assert!(matches!(q.execute(&pool).await, Err(Error::Connection(_))));
        }};
    }
    let row = insert(t::table()).set(t::ID.set(1));
    check!(row.clone(), "INSERT");
    check!(Insert::from(row.clone()), "INSERT");
    check!(Stmt::from(row), "INSERT");
    check!(
        insert(table("state_rows", &[]))
            .default_values()
            .returning_all(),
        "INSERT"
    );
    let update = update(t::table()).set(t::VALUE.set(1));
    check!(update.clone(), "UPDATE");
    check!(Stmt::from(update), "UPDATE");
    let delete = delete_from(t::table()).filter(t::ID.eq(1));
    check!(delete.clone(), "DELETE");
    check!(Stmt::from(delete), "DELETE");
    let merge = merge_into(
        t::alias("t"),
        t::alias("s"),
        t::ID.at("t").eq_field(t::ID.at("s")),
    )
    .when_matched()
    .do_nothing();
    check!(merge.clone(), "MERGE");
    check!(Stmt::from(merge), "MERGE");
    assert!(matches!(
        insert(t::table())
            .set(t::ID.set(1))
            .returning(t::ID)
            .fetch_one_scalar::<i32>(&pool)
            .await,
        Err(Error::Connection(_))
    ));
    assert!(matches!(
        raw("DELETE FROM state_rows").fetch_all(&pool).await,
        Err(Error::Connection(_))
    ));
    assert!(matches!(
        insert(t::table())
            .set(t::ID.set(1))
            .build()
            .unwrap()
            .fetch_all(&pool)
            .await,
        Err(Error::Connection(_))
    ));
}
