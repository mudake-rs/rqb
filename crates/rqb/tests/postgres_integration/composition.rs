use rqb::dsl::*;
use rqb::prelude::*;
use serde_json::json;
use uuid::Uuid;

rqb::schema! {
    table pg_temp.audit_rows {
        id: int4 = i32,
        optional_id: uuid = Uuid,
        quantity: int4 = i32,
        label: text = String,
        payload: jsonb = serde_json::Value,
    }
    table pg_temp.audit_ranges { ranges: int4multirange, }
}
use audit_rows as t;

async fn connection() -> sqlx::pool::PoolConnection<sqlx::Postgres> {
    let pool = super::common::pool().await;
    let mut conn = pool.acquire().await.unwrap();
    sqlx::raw_sql(
        "CREATE TEMP TABLE audit_rows (
        id int PRIMARY KEY, optional_id uuid, quantity int, label text,
        payload jsonb DEFAULT '{\"a\":1}');
        INSERT INTO audit_rows(id, label) VALUES (1, 'one'), (2, 'two');
        CREATE TEMP TABLE audit_ranges(ranges int4multirange);
        INSERT INTO audit_ranges VALUES ('{[1,4)}');",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    conn
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn raw_operands_and_lexical_boundaries_preserve_results() {
    let mut conn = connection().await;
    let ids = select(t::table())
        .column(t::ID)
        .filter(t::ID.eq(1))
        .filter(raw_predicate("false OR true", []))
        .fetch_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(ids, [1]);
    let value = select(t::table())
        .expr(raw_expr("1 + 2", []).op("*", 3_i32))
        .limit(1)
        .fetch_one_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, 9);
    let value = select(t::table())
        .expr(subscript(array([1_i32, 2]), 2_i32))
        .limit(1)
        .fetch_one_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, 2);
    let matrix = array([array([1_i32, 2]), array([3_i32, 4])]);
    let value = select(t::table())
        .expr(subscript(subscript(matrix, 1_i32), 2_i32))
        .limit(1)
        .fetch_one_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, 2);
    let value = raw("SELECT foo$tag$ + ? + bar$tag$ FROM (VALUES (1, 2)) AS t(foo$tag$, bar$tag$)")
        .bind(3_i32)
        .fetch_one_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, 6);
    let value = raw("SELECT $тег$?$тег$ || ?")
        .bind("ok")
        .fetch_one_scalar::<String>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, "?ok");
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    struct NonCloneText(String);
    let value = raw("SELECT ?::text")
        .bind(NonCloneText("x".repeat(1024 * 1024)))
        .fetch_one_scalar::<String>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value.len(), 1024 * 1024);
}

#[derive(rqb::Insertable)]
#[rqb(table = t)]
struct Input {
    id: i32,
    optional_id: Option<Uuid>,
    quantity: Option<i32>,
    label: Option<String>,
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn nullable_batch_upsert_and_qualified_returning_round_trip() {
    let mut conn = connection().await;
    insert(t::table())
        .values(&Input {
            id: 3,
            optional_id: None,
            quantity: None,
            label: None,
        })
        .execute(&mut *conn)
        .await
        .unwrap();
    let inputs = [
        Input {
            id: 1,
            optional_id: None,
            quantity: None,
            label: Some("changed".into()),
        },
        Input {
            id: 4,
            optional_id: None,
            quantity: None,
            label: None,
        },
    ];
    let rows = insert(t::table())
        .values_many(&inputs)
        .unwrap()
        .on_conflict(t::ID)
        .do_update_excluded((t::OPTIONAL_ID, t::QUANTITY, t::LABEL))
        .returning((t::ID, t::OPTIONAL_ID, t::QUANTITY, t::LABEL))
        .fetch_all_as::<(i32, Option<Uuid>, Option<i32>, Option<String>)>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.1.is_none() && row.2.is_none()));
    assert!(
        rows.iter()
            .any(|row| row.0 == 1 && row.3.as_deref() == Some("changed"))
    );
    let id = Uuid::new_v4();
    let rows = insert(t::table())
        .values_many([
            Input {
                id: 5,
                optional_id: None,
                quantity: None,
                label: None,
            },
            Input {
                id: 6,
                optional_id: Some(id),
                quantity: Some(9),
                label: Some("six".into()),
            },
        ])
        .unwrap()
        .returning((t::ID, t::OPTIONAL_ID, t::QUANTITY))
        .fetch_all_as::<(i32, Option<Uuid>, Option<i32>)>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(rows, [(5, None, None), (6, Some(id), Some(9))]);

    #[derive(sqlx::FromRow)]
    struct Returned {
        id: i32,
        label: Option<String>,
    }
    for (target, qualifier, id) in [
        (t::table().alias("target"), "target", 1),
        (t::table(), "audit_rows", 3),
    ] {
        let row = update(target.clone())
            .from(t::table().alias("other"))
            .set(t::LABEL.set("updated"))
            .filter(t::ID.at(qualifier).eq(id))
            .filter(t::ID.at("other").eq(2))
            .returning_all()
            .fetch_one_as::<Returned>(&mut *conn)
            .await
            .unwrap();
        assert_eq!(row.id, id);
        assert_eq!(row.label.as_deref(), Some("updated"));
        // Write fetch helpers execute the declared shape; RETURNING is explicit.
        let row = delete_from(target)
            .using(t::table().alias("other"))
            .filter(t::ID.at(qualifier).eq(id))
            .filter(t::ID.at("other").eq(2))
            .returning_all()
            .fetch_one_as::<Returned>(&mut *conn)
            .await
            .unwrap();
        assert_eq!(row.id, id);
    }
}

#[tokio::test]
#[ignore = "requires Postgres 18 and RQB_TEST_DATABASE_URL"]
async fn jsonpath_and_custom_multirange_predicate_execute() {
    let mut conn = connection().await;
    let ids = select(t::table())
        .column(t::ID)
        .filter(jsonb_path_exists(t::PAYLOAD, "$.a"))
        .order_asc(t::ID)
        .fetch_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(ids, [1, 2]);
    let values = select(t::table())
        .expr(jsonb_path_query(t::PAYLOAD, "$.a"))
        .fetch_scalar::<serde_json::Value>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(values, [json!(1), json!(1)]);
    // SQL/JSON syntax coerces text paths itself, unlike jsonb_path_* functions.
    let row = select(t::table())
        .filter(json_exists(t::PAYLOAD.expr().cast("json"), "$.a"))
        .expr(json_query(t::PAYLOAD, "$.a"))
        .expr(json_value(t::PAYLOAD, "$.a"))
        .limit(1)
        .fetch_one_as::<(serde_json::Value, String)>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row, (json!(1), "1".to_owned()));
    let value = select(audit_ranges::table())
        .expr(range_merge(audit_ranges::RANGES_META).cast("text"))
        .filter(audit_ranges::RANGES_META.expr().predicate("@>", 2_i32))
        .limit(1)
        .fetch_one_scalar::<String>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, "[1,4)");
    let value = select(t::table())
        .expr(
            function(
                "range_merge",
                [
                    raw_expr("'[1,2)'::int4range", []),
                    raw_expr("'[3,5)'::int4range", []),
                ],
            )
            .cast("text"),
        )
        .limit(1)
        .fetch_one_scalar::<String>(&mut *conn)
        .await
        .unwrap();
    assert_eq!(value, "[1,5)");
    // NOT NULL must stay NULL, not become true through an IS TRUE wrapper.
    let ids = select(t::table())
        .column(t::ID)
        .filter(not(t::QUANTITY.predicate("=", 1_i32)))
        .fetch_scalar::<i32>(&mut *conn)
        .await
        .unwrap();
    assert!(ids.is_empty());
}
