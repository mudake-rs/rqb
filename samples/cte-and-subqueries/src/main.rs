use rqb::dsl::{
    array, count_all, exists, generate_series_step_source, scalar_subquery, true_, unnest_source,
};
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::order_items as items;
use rqb_sample_schema::orders;
use uuid::Uuid;

// Application row shape for the recursive CTE below. The sample renders SQL
// without a database, so it does not fetch rows, but nullable `parent_id` maps
// to `Option<Uuid>` in the row type.
#[allow(dead_code)]
struct CommentNode {
    id: Uuid,
    parent_id: Option<Uuid>,
    body: String,
    depth: i32,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let u = users::alias("u");
    let o = orders::alias("o");
    let item_count = rqb::field!("item_count": int8 => i64, ordered);
    let n = rqb::field!("n": int4 => i32, ordered);
    let tag = rqb::field!("tag": text => String, equality);
    let comment_id = rqb::field!("id": uuid => Uuid, equality);
    let comment_parent_id = rqb::field!("parent_id": uuid => Uuid, equality);
    let comment_body = rqb::field!("body": text => String);
    let comment_depth = rqb::field!("depth": int4 => i32, ordered);

    // Subqueries are still server-owned query shapes. JSON requests cannot
    // introduce EXISTS, IN-subquery, joins, or raw SQL.
    let paid_orders = select(orders::table())
        .column(orders::USER_ID)
        .filter(orders::STATUS.eq("paid"));
    let paid_users = select(&u)
        .column(u.id())
        .filter(u.id().in_subquery(paid_orders.clone()))
        .filter(exists(
            select(&o)
                .column(o.id())
                .filter(o.user_id().eq_field(u.id())),
        ))
        .build()?;

    assert_eq!(
        paid_users.sql,
        "SELECT \"u\".\"id\" AS \"u_id\" FROM \"sample\".\"app_users\" AS \"u\" WHERE (\"u\".\"id\" IN (SELECT \"user_id\" FROM \"sample\".\"orders\" WHERE \"status\" = $1) AND EXISTS (SELECT \"o\".\"id\" AS \"o_id\" FROM \"sample\".\"orders\" AS \"o\" WHERE \"o\".\"user_id\" = \"u\".\"id\"))"
    );
    assert_eq!(paid_users.params.len(), 1);

    // Recursive CTEs often use raw SQL for the recursive body. The `recursive()`
    // marker owns the `WITH RECURSIVE` wrapper; the raw body starts at SELECT.
    let nums = cte(
        "nums",
        raw("SELECT ?::int4 AS n UNION ALL SELECT n + 1 FROM nums WHERE n < ?")
            .bind(1_i32)
            .bind(3_i32),
        n,
    )
    .recursive();
    let recursive = select(nums.source()).with(nums).column(n).build()?;

    assert_eq!(
        recursive.sql,
        "WITH RECURSIVE \"nums\" (\"n\") AS (SELECT $1::int4 AS n UNION ALL SELECT n + 1 FROM nums WHERE n < $2) SELECT \"n\" FROM \"nums\""
    );
    assert_eq!(recursive.params.len(), 2);

    // A tree walk follows the same shape. The database `parent_id` column is
    // nullable, so the row type uses `Option<Uuid>` even though field metadata
    // models the SQL type itself.
    let root_comment = Uuid::nil();
    let comment_walk = cte(
        "comment_walk",
        raw("SELECT id, parent_id, body, ?::int4 AS depth \
             FROM comments WHERE id = ?::uuid \
             UNION ALL \
             SELECT c.id, c.parent_id, c.body, w.depth + 1 \
             FROM comments c JOIN comment_walk w ON c.parent_id = w.id")
        .bind(0_i32)
        .bind(root_comment),
        (comment_id, comment_parent_id, comment_body, comment_depth),
    )
    .recursive();
    let comment_tree = select(comment_walk.source())
        .with(comment_walk)
        .columns((comment_id, comment_parent_id, comment_body, comment_depth))
        .order_asc(comment_depth)
        .build()?;

    assert_eq!(
        comment_tree.sql,
        "WITH RECURSIVE \"comment_walk\" (\"id\", \"parent_id\", \"body\", \"depth\") AS (SELECT id, parent_id, body, $1::int4 AS depth FROM comments WHERE id = $2::uuid UNION ALL SELECT c.id, c.parent_id, c.body, w.depth + 1 FROM comments c JOIN comment_walk w ON c.parent_id = w.id) SELECT \"id\", \"parent_id\", \"body\", \"depth\" FROM \"comment_walk\" ORDER BY \"depth\" ASC"
    );
    assert_eq!(comment_tree.params.len(), 2);

    // Common set-returning functions can be used as typed sources without
    // writing a raw `FROM generate_series(...)` fragment.
    let series = generate_series_step_source(1_i32, 3_i32, 1_i32, "g", n);
    let generated_series = select(series).build()?;

    assert_eq!(
        generated_series.sql,
        "SELECT \"g\".\"n\" FROM generate_series($1, $2, $3) AS \"g\" (\"n\")"
    );
    assert_eq!(generated_series.params.len(), 3);

    // `unnest_source` is the same source pattern for array-valued inputs.
    let tag_rows = unnest_source(array(["paid", "vip"]), "tag_rows", tag);
    let unnested_tags = select(tag_rows)
        .filter(tag.at("tag_rows").eq("paid"))
        .build()?;

    assert_eq!(
        unnested_tags.sql,
        "SELECT \"tag_rows\".\"tag\" FROM unnest(ARRAY[$1, $2]) AS \"tag_rows\" (\"tag\") WHERE \"tag_rows\".\"tag\" = $3"
    );
    assert_eq!(unnested_tags.params.len(), 3);

    // Computed subquery projections need explicit metadata because there is no
    // generated table field for `count(*) AS item_count`.
    let item_counts = subquery(
        select(items::table())
            .expr_as(count_all(), "item_count")
            .filter(items::ORDER_ID.eq_field(o.id())),
        "item_counts",
        item_count,
    );
    let lateral = select(&o)
        .columns((o.id(), item_count.at("item_counts")))
        .left_join_lateral(item_counts, true_())
        .build()?;

    assert_eq!(
        lateral.sql,
        "SELECT \"o\".\"id\" AS \"o_id\", \"item_counts\".\"item_count\" AS \"item_counts_item_count\" FROM \"sample\".\"orders\" AS \"o\" LEFT JOIN LATERAL (SELECT count(*) AS \"item_count\" FROM \"sample\".\"order_items\" WHERE \"order_id\" = \"o\".\"id\") AS \"item_counts\" (\"item_count\") ON TRUE"
    );

    // Scalar subqueries are value expressions. This keeps retention-style
    // deletes typed without constructing `ValueExpr::Subquery` by hand.
    let retention_delete = delete_from(orders::table())
        .filter(orders::USER_ID.eq(Uuid::nil()))
        .filter(
            orders::TOTAL_CENTS.expr().lte(scalar_subquery(
                select(orders::table())
                    .column(orders::TOTAL_CENTS)
                    .filter(orders::USER_ID.eq(Uuid::nil()))
                    .order_desc(orders::TOTAL_CENTS)
                    .offset(200)
                    .limit(1),
            )),
        )
        .build()?;

    assert_eq!(
        retention_delete.sql,
        "DELETE FROM \"sample\".\"orders\" WHERE (\"user_id\" = $1 AND \"total_cents\" <= (SELECT \"total_cents\" FROM \"sample\".\"orders\" WHERE \"user_id\" = $2 ORDER BY \"total_cents\" DESC LIMIT $3 OFFSET $4))"
    );
    assert_eq!(retention_delete.params.len(), 4);

    // Raw sources use the same exposed-field rule as subqueries.
    let raw_ids = raw_source(
        "SELECT ?::uuid AS id",
        "seeded",
        vec![Param::typed(Uuid::nil())],
        users::ID,
    );
    let raw_source_query = select(raw_ids).column(users::ID.at("seeded")).build()?;
    assert_eq!(
        raw_source_query.sql,
        "SELECT \"seeded\".\"id\" AS \"seeded_id\" FROM (SELECT $1::uuid AS id) AS \"seeded\" (\"id\")"
    );

    // VALUES sources are useful for joining or filtering against server-owned
    // in-memory rows while keeping column metadata explicit.
    let seeded_orders = values_source(
        [(Uuid::nil(), "paid"), (Uuid::nil(), "refunded")],
        "seeded_orders",
        (orders::ID, orders::STATUS),
    );
    let values_query = select(seeded_orders)
        .filter(orders::STATUS.at("seeded_orders").eq("paid"))
        .build()?;

    assert_eq!(
        values_query.sql,
        "SELECT \"seeded_orders\".\"id\", \"seeded_orders\".\"status\" FROM (VALUES ($1, $2), ($3, $4)) AS \"seeded_orders\" (\"id\", \"status\") WHERE \"seeded_orders\".\"status\" = $5"
    );
    assert_eq!(values_query.params.len(), 5);

    let set_query = union(
        select(orders::table())
            .column(orders::ID)
            .filter(orders::STATUS.eq("paid")),
        select(orders::table())
            .column(orders::ID)
            .filter(orders::STATUS.eq("refunded")),
    )
    .build()?;
    assert_eq!(
        set_query.sql,
        "(SELECT \"id\" FROM \"sample\".\"orders\" WHERE \"status\" = $1) UNION (SELECT \"id\" FROM \"sample\".\"orders\" WHERE \"status\" = $2)"
    );

    println!("{}", paid_users.sql);
    println!("{}", recursive.sql);
    println!("{}", comment_tree.sql);
    println!("{}", generated_series.sql);
    println!("{}", unnested_tags.sql);
    println!("{}", lateral.sql);
    println!("{}", retention_delete.sql);
    println!("{}", raw_source_query.sql);
    println!("{}", values_query.sql);
    println!("{}", set_query.sql);
    Ok(())
}
