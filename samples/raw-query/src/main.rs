use rqb::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawOrderStats {
    status: String,
    orders: i64,
    avg_total_cents: f64,
}

#[derive(Debug, Deserialize)]
struct EscapedQuestion {
    literal: String,
    value: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    // 1. Raw SQL can return a scalar when no generated metadata is useful.
    let version: String = raw_query("SELECT version()").fetch_one_scalar(&db).await?;
    println!("postgres version: {version}");

    // 2. Bind values with `?`; result columns are mapped by name into the row DTO.
    let stats = raw_query(
        "SELECT status::text AS status, \
                COUNT(*)::bigint AS orders, \
                AVG(total_cents)::float8 AS avg_total_cents \
         FROM order_search_view \
         WHERE status = ?::text::order_status \
         GROUP BY status",
    )
    .bind("paid")
    .fetch_all_as::<RawOrderStats>(&db)
    .await?;
    for row in &stats {
        println!(
            "status={} orders={} avg_total_cents={}",
            row.status, row.orders, row.avg_total_cents
        );
    }

    // 3. Scalar reads can still use bind params.
    let active_count: i64 =
        raw_query("SELECT COUNT(*)::bigint FROM app_users WHERE status = ?::text::user_status")
            .bind("active")
            .fetch_one_scalar(&db)
            .await?;
    println!("active users: {active_count}");

    // 4. Use `??` when the SQL text needs a literal question mark.
    let escaped: EscapedQuestion = raw_query("SELECT '??' AS literal, ?::text AS value")
        .bind("bound value")
        .fetch_one_as(&db)
        .await?;
    println!("literal={} value={}", escaped.literal, escaped.value);

    // 5. Raw queries use the same executor path as builders, including transactions.
    let tx = db.begin().await?;
    raw_query("UPDATE app_users SET profile = profile || ?::jsonb WHERE email = ?")
        .bind(serde_json::json!({ "rawQuerySample": true }))
        .bind("ada@example.com")
        .execute(&tx)
        .await?;
    let flag: bool =
        raw_query("SELECT (profile->>'rawQuerySample')::bool FROM app_users WHERE email = ?")
            .bind("ada@example.com")
            .fetch_one_scalar(&tx)
            .await?;
    println!("raw query update visible inside tx: {flag}");
    tx.rollback().await?;

    Ok(())
}
