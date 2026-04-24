use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use rqb::dsl::param;
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::invoices;
use rqb_sample_schema::orders;
use serde_json::{Value, json};
use sqlx::types::BigDecimal;
use uuid::Uuid;

// Derives use the generated schema module directly; there is no serde_json
// bridge between the DTO and sqlx bind values.
#[derive(Insertable)]
#[rqb(table = invoices)]
struct NewInvoice {
    customer_id: Uuid,
    amount: BigDecimal,
    due_on: NaiveDate,
    metadata: Value,
}

#[derive(Changeset)]
#[rqb(table = invoices)]
struct InvoiceChanges {
    paid_at: Option<DateTime<Utc>>,
    grace_period: Option<sqlx::postgres::types::PgInterval>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let invoice_id = Uuid::nil();
    let customer_id = Uuid::nil();
    let amount = BigDecimal::from_str("19.99")?;
    let due_on = NaiveDate::from_ymd_opt(2026, 5, 1).ok_or("invalid sample due date")?;
    let paid_at = Utc::now();
    let new_invoice = NewInvoice {
        customer_id,
        amount: amount.clone(),
        due_on,
        metadata: json!({ "source": "sample" }),
    };
    let mark_paid = InvoiceChanges {
        paid_at: Some(paid_at),
        grace_period: None,
    };

    // Manual field assignment and derived `Insertable` compose in the same
    // insert statement.
    let insert_sql = insert(invoices::table())
        .set(invoices::ID.set(invoice_id))
        .values(&new_invoice)
        .returning_all()
        .build()?;

    // `Changeset` keeps partial update DTOs small and skips `None` fields.
    let update_sql = update(invoices::table())
        .patch(&mark_paid)
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let delete_sql = delete_from(invoices::table())
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let raw_sql = raw("SELECT ?::numeric + ?::numeric")
        .bind(amount.clone())
        .bind(BigDecimal::from_str("5.00")?)
        .build()?;

    // Conflict targets accept one field, a tuple of fields, or a named
    // constraint. Predicates are allowed only for column targets.
    let upsert_user_sql = insert(users::table())
        .set(users::ID.set(Uuid::nil()))
        .set(users::EMAIL.set("ada@example.com"))
        .set(users::DISPLAY_NAME.set("Ada"))
        .set(users::STATUS.set("active"))
        .on_conflict(users::EMAIL)
        .target_where(users::ACTIVE.eq(true))
        .do_update_set_where(
            [
                users::DISPLAY_NAME.set_excluded(),
                users::STATUS.set("active"),
            ],
            users::ACTIVE.eq(true),
        )
        .returning_all()
        .build()?;

    let ignore_duplicate_sql = insert(users::table())
        .set(users::ID.set(Uuid::nil()))
        .set(users::EMAIL.set("ada@example.com"))
        .set(users::DISPLAY_NAME.set("Ada"))
        .on_conflict_constraint("app_users_email_key")
        .do_nothing()
        .returning(users::ID)
        .build()?;

    // `INSERT ... SELECT` stays typed on both sides and validates target column
    // count against the select projection before rendering SQL.
    let seed_open_orders_sql = insert(orders::table())
        .column(orders::ID)
        .column(orders::USER_ID)
        .column(orders::STATUS)
        .column(orders::TOTAL_CENTS)
        .from_select(
            select(users::table())
                .expr(param(Uuid::nil()))
                .column(users::ID)
                .expr("open")
                .expr(1000_i64)
                .filter(users::ACTIVE.eq(true)),
        )
        .returning(orders::ID)
        .build()?;

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"sample\".\"invoices\" (\"id\", \"customer_id\", \"amount\", \"due_on\", \"metadata\") VALUES ($1, $2, $3, $4, $5) RETURNING \"id\", \"invoice_no\", \"customer_id\", \"state\", \"amount\", \"tax_rate\", \"amount_history\", \"due_on\", \"issued_at\", \"paid_at\", \"reminder_time\", \"cutoff_time\", \"grace_period\", \"service_days\", \"billing_window\", \"client_ip\", \"client_network\", \"pdf\", \"tags\", \"metadata\""
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"sample\".\"invoices\" SET \"paid_at\" = $1 WHERE \"id\" = $2 RETURNING \"id\""
    );
    assert_eq!(
        delete_sql.sql,
        "DELETE FROM \"sample\".\"invoices\" WHERE \"id\" = $1 RETURNING \"id\""
    );
    assert_eq!(raw_sql.sql, "SELECT $1::numeric + $2::numeric");
    assert_eq!(
        upsert_user_sql.sql,
        "INSERT INTO \"sample\".\"app_users\" (\"id\", \"email\", \"display_name\", \"status\") VALUES ($1, $2, $3, $4) ON CONFLICT (\"email\") WHERE \"active\" = $5 DO UPDATE SET \"display_name\" = EXCLUDED.\"display_name\", \"status\" = $6 WHERE \"active\" = $7 RETURNING \"id\", \"organization_id\", \"email\", \"status\", \"display_name\", \"active\", \"created_at\""
    );
    assert_eq!(
        ignore_duplicate_sql.sql,
        "INSERT INTO \"sample\".\"app_users\" (\"id\", \"email\", \"display_name\") VALUES ($1, $2, $3) ON CONFLICT ON CONSTRAINT \"app_users_email_key\" DO NOTHING RETURNING \"id\""
    );
    assert_eq!(
        seed_open_orders_sql.sql,
        "INSERT INTO \"sample\".\"orders\" (\"id\", \"user_id\", \"status\", \"total_cents\") SELECT $1, \"id\", $2, $3 FROM \"sample\".\"app_users\" WHERE \"active\" = $4 RETURNING \"id\""
    );

    println!("{}", insert_sql.sql);
    println!("{}", upsert_user_sql.sql);
    Ok(())
}
