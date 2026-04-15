use std::str::FromStr;

use rqb::prelude::*;

mod invoices {
    use rqb::prelude::*;

    pub static ID_META: Meta = Meta::new("id", "id", "uuid")
        .ops(OpSet::ordered())
        .json(JsonKind::Uuid);
    pub static CUSTOMER_ID_META: Meta = Meta::new("customerId", "customer_id", "uuid")
        .ops(OpSet::ordered())
        .json(JsonKind::Uuid);
    pub static AMOUNT_META: Meta = Meta::new("amount", "amount", "numeric")
        .ops(OpSet::ordered())
        .json(JsonKind::NumericString);
    pub static DUE_ON_META: Meta = Meta::new("dueOn", "due_on", "date")
        .ops(OpSet::ordered())
        .json(JsonKind::Date);
    pub static PAID_AT_META: Meta = Meta::new("paidAt", "paid_at", "timestamptz")
        .ops(OpSet::ordered())
        .json(JsonKind::Timestamptz);
    pub static METADATA_META: Meta = Meta::new("metadata", "metadata", "jsonb")
        .ops(OpSet::equality())
        .json(JsonKind::Jsonb);

    pub const ID: Field<rqb::uuid::Uuid> = Field::new(&ID_META);
    pub const CUSTOMER_ID: Field<rqb::uuid::Uuid> = Field::new(&CUSTOMER_ID_META);
    pub const AMOUNT: Field<rqb::sqlx::types::BigDecimal> = Field::new(&AMOUNT_META);
    pub const DUE_ON: Field<rqb::chrono::NaiveDate> = Field::new(&DUE_ON_META);
    pub const PAID_AT: Field<rqb::chrono::DateTime<rqb::chrono::Utc>> =
        Field::new(&PAID_AT_META);
    pub const METADATA: Field<rqb::serde_json::Value> = Field::new(&METADATA_META);

    pub static FIELDS: [&Meta; 6] = [
        &ID_META,
        &CUSTOMER_ID_META,
        &AMOUNT_META,
        &DUE_ON_META,
        &PAID_AT_META,
        &METADATA_META,
    ];

    pub fn table() -> Source {
        rqb::table("public.invoices", &FIELDS)
    }
}

fn main() -> rqb::Result<()> {
    let invoice_id = rqb::uuid::Uuid::nil();
    let customer_id = rqb::uuid::Uuid::nil();
    let amount = rqb::sqlx::types::BigDecimal::from_str("19.99").unwrap();
    let due_on = rqb::chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    let paid_at = rqb::chrono::Utc::now();

    let insert_sql = insert(invoices::table())
        .set(invoices::ID.set(invoice_id))
        .set(invoices::CUSTOMER_ID.set(customer_id))
        .set(invoices::AMOUNT.set(amount.clone()))
        .set(invoices::DUE_ON.set(due_on))
        .set(invoices::METADATA.set(rqb::serde_json::json!({ "source": "sample" })))
        .returning(invoices::ID)
        .build()?;

    let update_sql = update(invoices::table())
        .set(invoices::PAID_AT.set(paid_at))
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let delete_sql = delete_from(invoices::table())
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let raw_sql = raw("SELECT ?::numeric + ?::numeric")
        .bind(amount.clone())
        .bind(rqb::sqlx::types::BigDecimal::from_str("5.00").unwrap())
        .build()?;

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"public\".\"invoices\" (\"id\", \"customer_id\", \"amount\", \"due_on\", \"metadata\") VALUES ($1, $2, $3, $4, $5) RETURNING \"id\""
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"public\".\"invoices\" SET \"paid_at\" = $1 WHERE \"id\" = $2 RETURNING \"id\""
    );
    assert_eq!(
        delete_sql.sql,
        "DELETE FROM \"public\".\"invoices\" WHERE \"id\" = $1 RETURNING \"id\""
    );
    assert_eq!(raw_sql.sql, "SELECT $1::numeric + $2::numeric");

    println!("{}", insert_sql.sql);
    Ok(())
}
