use rqb::dsl::count_all;
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::order_items as items;
use rqb_sample_schema::orders;
use uuid::Uuid;

static ITEM_COUNT_META: Meta = Meta::new("item_count", "item_count", "int8").ops(OpSet::ordered());
const ITEM_COUNT: Field<i64> = Field::new(&ITEM_COUNT_META);

static N_META: Meta = Meta::new("n", "n", "int4").ops(OpSet::ordered());
const N: Field<i32> = Field::new(&N_META);

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let u = users::alias("u");
    let o = orders::alias("o");

    let paid_orders = select(orders::table())
        .column(orders::USER_ID)
        .filter(orders::STATUS.eq("paid"));
    let paid_users = select(&u)
        .column(u.id())
        .filter(u.id().in_subquery(paid_orders.clone()))
        .filter(BoolExpr::Exists(Box::new(
            select(&o)
                .column(o.id())
                .filter(o.user_id().eq_field(u.id()))
                .into(),
        )))
        .build()?;

    assert_eq!(
        paid_users.sql,
        "SELECT \"u\".\"id\" AS \"u_id\" FROM \"sample\".\"app_users\" AS \"u\" WHERE (\"u\".\"id\" IN (SELECT \"user_id\" FROM \"sample\".\"orders\" WHERE \"status\" = $1) AND EXISTS (SELECT \"o\".\"id\" AS \"o_id\" FROM \"sample\".\"orders\" AS \"o\" WHERE \"o\".\"user_id\" = \"u\".\"id\"))"
    );
    assert_eq!(paid_users.params.len(), 1);

    let nums = cte(
        "nums",
        Stmt::Raw(
            raw("SELECT ?::int4 AS n UNION ALL SELECT n + 1 FROM nums WHERE n < ?")
                .bind(1_i32)
                .bind(3_i32),
        ),
        vec![N_META],
    )
    .recursive();
    let recursive = select(nums.source()).with(nums).column(N).build()?;

    assert_eq!(
        recursive.sql,
        "WITH RECURSIVE \"nums\" (\"n\") AS (SELECT $1::int4 AS n UNION ALL SELECT n + 1 FROM nums WHERE n < $2) SELECT \"n\" FROM \"nums\""
    );
    assert_eq!(recursive.params.len(), 2);

    let item_counts = subquery(
        select(items::table())
            .agg(count_all().alias("item_count"))
            .filter(items::ORDER_ID.eq_field(o.id())),
        "item_counts",
        vec![ITEM_COUNT_META],
    );
    let lateral = select(&o)
        .column(o.id())
        .left_join_lateral(item_counts, BoolExpr::Constant(true))
        .column(ITEM_COUNT.at("item_counts"))
        .build()?;

    assert_eq!(
        lateral.sql,
        "SELECT \"o\".\"id\" AS \"o_id\", \"item_counts\".\"item_count\" AS \"item_counts_item_count\" FROM \"sample\".\"orders\" AS \"o\" LEFT JOIN LATERAL (SELECT count(*) AS \"item_count\" FROM \"sample\".\"order_items\" WHERE \"order_id\" = \"o\".\"id\") AS \"item_counts\" ON TRUE"
    );

    let raw_ids = raw_source(
        "SELECT ?::uuid AS id",
        "seeded",
        vec![Param::typed(Uuid::nil())],
        vec![*users::ID.meta],
    );
    let raw_source_query = select(raw_ids).column(users::ID.at("seeded")).build()?;
    assert_eq!(
        raw_source_query.sql,
        "SELECT \"seeded\".\"id\" AS \"seeded_id\" FROM (SELECT $1::uuid AS id) AS \"seeded\""
    );

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
    println!("{}", lateral.sql);
    println!("{}", raw_source_query.sql);
    println!("{}", set_query.sql);
    Ok(())
}
