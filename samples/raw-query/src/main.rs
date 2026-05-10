use rqb::prelude::*;
use rqb_sample_schema::orders;
use uuid::Uuid;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Raw SQL uses `?` as a bind placeholder. Escape it as `??` when the SQL
    // text itself needs a literal question mark.
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
        .column(orders::ID.at("recent"))
        .column(orders::TOTAL_CENTS.at("recent"))
        .filter(BoolExpr::Raw {
            sql: "total_cents > ?".to_owned(),
            params: vec![Param::typed(1_000_i64)],
        })
        .build()?;

    assert_eq!(
        mixed.sql,
        "SELECT \"recent\".\"id\" AS \"recent_id\", \"recent\".\"total_cents\" AS \"recent_total_cents\" FROM (SELECT $1::uuid AS id, $2::bigint AS total_cents) AS \"recent\" (\"id\", \"total_cents\") WHERE total_cents > $3"
    );
    assert_eq!(mixed.params.len(), 3);
    assert!(!mixed.cacheable);

    assert!(matches!(
        raw("SELECT ?::int4").build(),
        Err(rqb::Error::RawBindMismatch {
            placeholders: 1,
            binds: 0
        })
    ));

    println!("{}", raw_stmt.sql);
    println!("{}", mixed.sql);
    Ok(())
}
