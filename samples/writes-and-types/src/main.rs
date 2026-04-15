use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use rqb::prelude::*;
use rqb_sample_schema::invoices;
use serde_json::{Value, json};
use sqlx::types::BigDecimal;
use uuid::Uuid;

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
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let invoice_id = Uuid::nil();
    let customer_id = Uuid::nil();
    let amount = BigDecimal::from_str("19.99").unwrap();
    let due_on = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    let paid_at = Utc::now();
    let new_invoice = NewInvoice {
        customer_id,
        amount: amount.clone(),
        due_on,
        metadata: json!({ "source": "sample" }),
    };
    let mark_paid = InvoiceChanges {
        paid_at: Some(paid_at),
    };

    let insert_sql = insert(invoices::table())
        .set(invoices::ID.set(invoice_id))
        .values(&new_invoice)
        .returning(invoices::ID)
        .build()?;

    let update_sql = update(invoices::table())
        .changes(&mark_paid)
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let delete_sql = delete_from(invoices::table())
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let raw_sql = raw("SELECT ?::numeric + ?::numeric")
        .bind(amount.clone())
        .bind(BigDecimal::from_str("5.00").unwrap())
        .build()?;

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"sample\".\"invoices\" (\"id\", \"customer_id\", \"amount\", \"due_on\", \"metadata\") VALUES ($1, $2, $3, $4, $5) RETURNING \"id\""
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

    println!("{}", insert_sql.sql);
    Ok(())
}
