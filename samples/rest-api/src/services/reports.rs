use chrono::{Duration, Utc};
use rqb::dsl::{count_all, date_trunc, sum};
use rqb::prelude::*;
use rqb_sample_schema::orders;

use crate::types::DailyOrderStats;

pub async fn orders_by_day<'e>(
    db: impl PgExecutor<'e>,
    days: i64,
) -> rqb::Result<Vec<DailyOrderStats>> {
    let days = days.clamp(1, 90);
    let since = Utc::now() - Duration::days(days);
    let day = date_trunc("day", orders::CREATED_AT);

    // Derived expressions are regular AST nodes. Keep the expression in a
    // local variable when SELECT, GROUP BY, and ORDER BY must refer to the
    // same SQL fragment.
    select(orders::table())
        .item(day.clone().alias("day"))
        .column(orders::STATUS)
        .item(count_all().alias("order_count"))
        .item(sum(orders::TOTAL_CENTS).cast("int8").alias("gross_cents"))
        .filter(orders::CREATED_AT.gte(since))
        .group_by(day.clone())
        .group_by(orders::STATUS)
        .order_asc(day)
        .order_asc(orders::STATUS)
        .fetch_all_as::<DailyOrderStats>(db)
        .await
}
