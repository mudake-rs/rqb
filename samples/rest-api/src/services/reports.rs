use chrono::{Duration, Utc};
use rqb::dsl::{DatePart, count_all, date_trunc_part, sum};
use rqb::prelude::*;
use rqb_sample_schema::orders;

use crate::types::DailyOrderStats;

pub async fn orders_by_day(db: &PgPool, days: i64) -> rqb::Result<Vec<DailyOrderStats>> {
    let days = days.clamp(1, 90);
    let since = Utc::now() - Duration::days(days);
    let day = date_trunc_part(DatePart::Day, orders::CREATED_AT);

    // Derived expressions are regular AST nodes. Keep the expression in a
    // local variable when SELECT, GROUP BY, and ORDER BY must refer to the
    // same SQL fragment.
    select(orders::table())
        .expr_as(day.clone(), "day")
        .column(orders::STATUS)
        .expr_as(count_all(), "order_count")
        .expr_as(sum(orders::TOTAL_CENTS).cast("int8"), "gross_cents")
        .filter(orders::CREATED_AT.gte(since))
        .group_by(day.clone())
        .group_by(orders::STATUS)
        .order_asc(day)
        .order_asc(orders::STATUS)
        .fetch_all_as::<DailyOrderStats>(db)
        .await
}
