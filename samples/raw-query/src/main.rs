use rqb::prelude::*;
use rqb_sample_schema::orders;
use uuid::Uuid;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Raw SQL uses `?` as a bind placeholder outside quoted strings,
    // dollar-quoted bodies, quoted identifiers, and comments. Escape SQL
    // operator question marks as `??` when the SQL text itself needs one.
    let raw_stmt = raw("SELECT ?? AS marker, ?::uuid AS id")
        .bind(Uuid::nil())
        .build()?;

    assert_eq!(raw_stmt.sql, "SELECT ? AS marker, $1::uuid AS id");
    assert_eq!(raw_stmt.params.len(), 1);
    assert!(!raw_stmt.cacheable);

    // A raw source can join typed queries, but rqb still needs the columns it
    // exposes so later `.column(...)` calls can render qualified fields.
    let raw_orders = raw_source(
        "SELECT ?::uuid AS id, ?::bigint AS total_cents",
        "recent",
        vec![Param::typed(Uuid::nil()), Param::typed(5_000_i64)],
        (orders::ID, orders::TOTAL_CENTS),
    );
    let mixed = select(raw_orders)
        .columns((orders::ID.at("recent"), orders::TOTAL_CENTS.at("recent")))
        .filter(raw_predicate("total_cents > ?", [Param::typed(1_000_i64)]))
        .build()?;

    assert_eq!(
        mixed.sql,
        "SELECT \"recent\".\"id\" AS \"recent_id\", \"recent\".\"total_cents\" AS \"recent_total_cents\" FROM (SELECT $1::uuid AS id, $2::bigint AS total_cents) AS \"recent\" (\"id\", \"total_cents\") WHERE total_cents > $3"
    );
    assert_eq!(mixed.params.len(), 3);
    assert!(!mixed.cacheable);

    // Raw expressions fit into projections and compose their bind placeholders
    // with the rest of the surrounding typed query.
    let computed = select(orders::table())
        .column(orders::ID)
        .expr_as(
            raw_expr(
                "coalesce(metadata ->> ?, ?)",
                [
                    Param::typed("source".to_owned()),
                    Param::typed("unknown".to_owned()),
                ],
            ),
            "source",
        )
        .filter(orders::ID.eq(Uuid::nil()))
        .build()?;

    assert_eq!(
        computed.sql,
        "SELECT \"id\", coalesce(metadata ->> $1, $2) AS \"source\" FROM \"sample\".\"orders\" WHERE \"id\" = $3"
    );
    assert_eq!(computed.params.len(), 3);
    assert!(!computed.cacheable);

    assert!(matches!(
        raw("SELECT ?::int4").build(),
        Err(rqb::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        })
    ));

    println!("{}", raw_stmt.sql);
    println!("{}", mixed.sql);
    println!("{}", computed.sql);
    Ok(())
}
