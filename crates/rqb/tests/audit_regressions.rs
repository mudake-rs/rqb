use rqb::dsl::*;
use rqb::prelude::*;

rqb::schema! {
    table public.audit_rows {
        id: int4 = i32,
        email: text = String,
        payload: jsonb = serde_json::Value,
        legacy: json = serde_json::Value,
        offset_time: timetz = sqlx::postgres::types::PgTimeTz,
        ranges: int4multirange,
        #[rqb(ops = none)] tags: "text[]" = Vec<String>,
    }
    table public.colliding_names {
        id: int4 = i32,
        id_meta: int4 = i32,
        source: text = String,
        source_ as SOURCE_: text = String,
        fields as FIELDS_1: int4 = i32,
    }
    table public.raw_method_names {
        source: custom_type,
        source_ as SOURCE_: int4 = i32,
    }
}
use audit_rows as t;

#[test]
fn schema_expansion_has_one_owner_for_each_identifier() {
    assert_eq!(colliding_names::ID.meta.db, "id");
    assert_eq!(colliding_names::ID_META.meta.db, "id_meta");
    let a = colliding_names::alias("a");
    assert_eq!(a.source_().meta.db, "source");
    assert_eq!(a.source_2().meta.db, "source_");
    assert_eq!(colliding_names::FIELDS_1.meta.db, "fields");
    assert_eq!(raw_method_names::alias("r").source_().meta.db, "source_");
    assert_eq!(t::LEGACY.meta.ops, OpSet::none());
    assert_eq!(t::LEGACY.meta.json, None);
    assert_eq!(t::OFFSET_TIME.meta.json, None);
    assert_eq!(t::PAYLOAD.meta.json, Some(JsonKind::Jsonb));
}

#[test]
fn raw_composition_preserves_predicate_and_arithmetic_grouping() {
    let q = select(t::table())
        .column(t::ID)
        .filter(t::ID.eq(1))
        .filter(raw_predicate("false OR true", []))
        .build()
        .unwrap();
    assert!(q.sql.ends_with("WHERE (\"id\" = $1 AND (false OR true))"));
    let q = select(t::table())
        .expr(raw_expr("1 + 2", []).op("*", 3_i32))
        .filter(not(raw_predicate("false OR true", [])))
        .build()
        .unwrap();
    assert!(q.sql.starts_with("SELECT ((1 + 2) * $1)"));
    assert!(q.sql.ends_with("NOT (false OR true)"));
    let star = select(t::table())
        .expr(function("count", [raw_expr("*", [])]))
        .build()
        .unwrap();
    assert!(star.sql.starts_with("SELECT count(*)"));
}

#[test]
fn raw_dollar_quotes_respect_identifier_boundaries() {
    for (sql, expected) in [
        (
            "SELECT foo$tag$ + ? + bar$tag$",
            "SELECT foo$tag$ + $1 + bar$tag$",
        ),
        ("SELECT $тег$?$тег$, ?", "SELECT $тег$?$тег$, $1"),
        (
            "SELECT имя$tag$ + ? + other$tag$",
            "SELECT имя$tag$ + $1 + other$tag$",
        ),
    ] {
        let q = raw(sql).bind(1_i32).build().unwrap();
        assert_eq!(q.sql, expected);
        assert!(q.pretty_sql().contains("$1"));
    }
}

#[test]
fn known_subquery_arities_are_checked_without_guessing_raw_projection() {
    let one = || select(t::table()).column(t::ID);
    let two = || select(t::table()).columns((t::ID, t::EMAIL));
    assert!(one().expr(scalar_subquery(two())).build().is_err());
    assert!(one().filter(t::ID.in_subquery(two())).build().is_err());
    assert!(one().union(two()).build().is_err());
    assert!(one().union(raw("SELECT 1")).union(two()).build().is_err());
    assert!(
        one()
            .expr(scalar_subquery(select(t::table()).expr(raw_expr("*", []))))
            .build()
            .is_ok()
    );
    assert!(one().expr(case().else_(0_i32)).build().is_err());
    assert!(
        one()
            .expr(case().when(t::ID.eq(1), 1_i32).else_(0_i32))
            .build()
            .is_ok()
    );
}

#[test]
fn modeled_aggregates_and_windows_exclude_same_scope_row_locks() {
    for query in [
        select(t::table()).distinct(),
        select(t::table()).distinct_on(t::ID),
        select(t::table()).group_by(t::ID),
        select(t::table()).expr(count(t::ID)),
        select(t::table()).expr(coalesce([sum(t::ID), 0_i32.into()])),
        select(t::table()).expr(row_number().over(window())),
    ] {
        assert!(query.for_update().build().is_err());
    }
    assert!(
        select(t::table())
            .expr(scalar_subquery(select(t::table()).expr(count(t::ID))))
            .for_update()
            .build()
            .is_ok()
    );
    assert!(
        select(t::table())
            .for_update()
            .skip_locked()
            .build()
            .is_ok()
    );
}

#[test]
fn set_operands_reject_locks_without_crossing_subquery_scopes() {
    let one = || select(t::table()).column(t::ID);
    for query in [
        one().for_update().union(one()),
        one().union_all(one().for_share()),
        one().for_update().intersect(one()),
        one().intersect_all(one().for_share()),
        one().for_update().except(one()),
        one().except_all(one().for_share()),
        one().union(one().union(one().for_update())),
        one().for_update().union(one()).union(one()),
    ] {
        assert!(matches!(
            query.build(),
            Err(rqb::Error::InvalidSelectShape { .. })
        ));
    }
    let locked_source = one().for_update().infer_source("locked").unwrap();
    assert!(select(locked_source).union(one()).build().is_ok());
}

#[test]
fn frame_bound_categories_follow_postgres_ordering() {
    let bounds = [
        unbounded_preceding(),
        preceding(1_i32),
        current_row(),
        following(1_i32),
        unbounded_following(),
    ];
    for (start_index, start) in bounds.iter().enumerate() {
        for (end_index, end) in bounds.iter().enumerate() {
            let q = select(t::table()).expr(
                row_number().over(
                    window()
                        .order_asc(t::ID)
                        .frame(rows(start.clone()).between(end.clone())),
                ),
            );
            let valid = start_index != 4 && end_index != 0 && start_index <= end_index;
            assert_eq!(q.build().is_ok(), valid, "{start_index} -> {end_index}");
        }
    }
    assert!(
        select(t::table())
            .expr(row_number().over(window().frame(rows(following(1_i32)))))
            .build()
            .is_err()
    );
    assert!(
        select(t::table())
            .expr(row_number().over(window().frame(groups(current_row()))))
            .build()
            .is_err()
    );
    assert!(
        select(t::table())
            .expr(row_number().over(window().frame(range(preceding(1_i32)))))
            .build()
            .is_err()
    );
}

#[test]
fn merge_exhaustion_and_assignment_replacement_use_reachable_builders() {
    let merge = || {
        merge_into(
            t::table().alias("t"),
            t::table().alias("s"),
            t::ID.at("t").eq_field(t::ID.at("s")),
        )
    };
    assert!(
        merge()
            .when_matched()
            .delete()
            .when_matched_if(t::ID.eq(1))
            .do_nothing()
            .build()
            .is_err()
    );
    assert!(
        merge()
            .when_matched_if(t::ID.eq(1))
            .delete()
            .when_matched()
            .do_nothing()
            .build()
            .is_ok()
    );
    assert!(
        merge()
            .when_matched()
            .delete()
            .when_not_matched()
            .do_nothing()
            .build()
            .is_ok()
    );
    let q = insert(t::table())
        .set(t::ID.set(1))
        .on_conflict(t::ID)
        .do_update_set((t::EMAIL.set("old"), t::EMAIL.set("new")))
        .build()
        .unwrap();
    assert!(q.sql.ends_with("DO UPDATE SET \"email\" = $2"));
    assert_eq!(q.params.len(), 2);
    assert!(
        update(t::table().alias(""))
            .set(t::ID.set(1))
            .build()
            .is_err()
    );
}

#[test]
fn filter_only_preserves_server_paging_and_checks_capabilities() {
    let filter = serde_json::from_value(
        serde_json::json!({"field":"email","operator":"iLike","value":"a%"}),
    )
    .unwrap();
    let q = select(t::table())
        .filter(t::ID.gt(0))
        .order_desc(t::ID)
        .limit(7)
        .offset(2)
        .apply_filter(filter)
        .unwrap()
        .build()
        .unwrap();
    assert!(q.sql.ends_with("ORDER BY \"id\" DESC LIMIT $3 OFFSET $4"));
    assert_eq!(q.params.len(), 4);
    assert!(q.sql.contains("AND \"email\" ILIKE $2"));
    for field in ["ranges", "legacy", "offset_time"] {
        let filter = serde_json::from_value(
            serde_json::json!({"field":field,"operator":"equals","value":"a"}),
        )
        .unwrap();
        assert!(select(t::table()).apply_filter(filter).is_err());
    }
    assert!(
        select(t::table())
            .filter(t::RANGES_META.expr().predicate("@>", raw_expr("1", [])))
            .build()
            .is_ok()
    );
    // Server-owned collection helpers check PostgreSQL type shape. OpSet remains
    // authoritative for JSON search; it is not a custom SQL operator registry.
    assert!(
        select(t::table())
            .filter(t::TAGS.contains(vec!["a".to_owned()]))
            .build()
            .is_ok()
    );
    let mismatched_array = Field::<Vec<String>>::new(t::ID.meta);
    assert!(
        select(t::table())
            .filter(mismatched_array.contains(vec!["a".to_owned()]))
            .build()
            .is_err()
    );
}

#[test]
fn inferred_qualified_fields_retain_metadata_and_output_column_list() {
    let u = t::alias("u");
    let source = select(&u).column(u.id()).infer_source("ids").unwrap();
    let q = select(source).build().unwrap();
    assert!(q.sql.contains("AS \"ids\" (\"id\")"));
    assert!(
        select(t::table())
            .expr_as(t::ID, "renamed")
            .infer_source("ids")
            .is_err()
    );

    let v = t::alias("v");
    let joined = || select(&u).cross_join(&v).columns((u.id(), v.id()));
    assert!(joined().infer_source("ids").is_err());
    assert!(joined().infer_cte("ids").is_err());
    static SAME_API_A: Meta = Meta::new("key", "a", "int4");
    static SAME_API_B: Meta = Meta::new("key", "b", "int4");
    static FIELDS: [&Meta; 2] = [&SAME_API_A, &SAME_API_B];
    let source = || rqb::table("public.ambiguous", &FIELDS);
    assert!(select(source()).infer_source("q").is_err());
    assert!(select(source()).infer_cte("q").is_err());
    assert!(
        select(source())
            .columns((
                Field::<i32>::new(&SAME_API_A),
                Field::<i32>::new(&SAME_API_B)
            ))
            .infer_source("q")
            .is_err()
    );
}

#[test]
fn field_projection_and_inference_share_automatic_alias_rules() {
    static META: Meta = Meta::new("public_name", "stored_name", "int4");
    static FIELDS: [&Meta; 1] = [&META];
    let field = Field::<i32>::new(&META);
    let source = || rqb::table("public.renamed", &FIELDS);
    for (query, prefix) in [
        (
            select(source()).column(field),
            "SELECT \"stored_name\" AS \"public_name\" FROM",
        ),
        (
            select(source().alias("u")).column(field.at("u")),
            "SELECT \"u\".\"stored_name\" AS \"u_public_name\" FROM",
        ),
    ] {
        assert!(query.build().unwrap().sql.starts_with(prefix));
        let derived = query.clone().infer_source("q").unwrap();
        let sql = select(derived).build().unwrap().sql;
        assert!(sql.starts_with("SELECT \"q\".\"stored_name\" AS \"public_name\" FROM"));
        let cte = query.infer_cte("q").unwrap();
        let sql = select(cte.source()).with(cte).build().unwrap().sql;
        assert!(sql.starts_with("WITH \"q\" (\"stored_name\") AS"));
    }
}

#[derive(rqb::Insertable)]
#[rqb(table = t)]
struct NullableInsert {
    id: Option<i32>,
    email: Option<String>,
}

#[derive(rqb::Changeset)]
#[rqb(table = t)]
struct NullablePatch {
    email: Option<Option<String>>,
}

#[test]
fn nullable_dto_transitions_preserve_null_vs_omit() {
    let q = insert(t::table())
        .values(&NullableInsert {
            id: None,
            email: Some("a".into()),
        })
        .build()
        .unwrap();
    assert!(q.sql.contains("VALUES (NULL, $1)"));
    let q = insert(t::table())
        .values_many([NullableInsert {
            id: None,
            email: None,
        }])
        .unwrap()
        .build()
        .unwrap();
    assert!(q.sql.contains("CAST(NULL AS int4), CAST(NULL AS text)"));
    assert!(
        NullablePatch { email: None }
            .changeset_assignments()
            .is_empty()
    );
    let q = update(t::table())
        .patch(&NullablePatch { email: Some(None) })
        .build()
        .unwrap();
    assert!(q.sql.ends_with("SET \"email\" = NULL"));
    let q = update(t::table())
        .patch(&NullablePatch {
            email: Some(Some("new".into())),
        })
        .build()
        .unwrap();
    assert!(q.sql.ends_with("SET \"email\" = $1"));
}
