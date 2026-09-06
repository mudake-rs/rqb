use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use rqb::dsl::{isempty, range_lower, to_char};
use rqb::prelude::*;
use rqb_sample_schema::InvoiceState;
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
    #[rqb(field = invoices::CUSTOMER_ID)]
    customer: Uuid,
    amount: BigDecimal,
    due_on: NaiveDate,
    metadata: Value,
    #[rqb(skip_none)]
    paid_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    #[rqb(skip)]
    client_note: Option<String>,
}

#[derive(Changeset)]
#[rqb(table = invoices)]
struct InvoiceChanges {
    paid_at: Option<DateTime<Utc>>,
    grace_period: Option<sqlx::postgres::types::PgInterval>,
}

#[derive(Insertable)]
#[rqb(table = users)]
struct NewUser {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: String,
    display_name: String,
}

#[derive(serde::Deserialize, Changeset)]
#[serde(deny_unknown_fields)]
#[rqb(table = users)]
struct OrganizationPatch {
    // Missing key leaves membership unchanged; JSON null removes membership.
    #[serde(default, deserialize_with = "present_nullable")]
    organization_id: Option<Option<Uuid>>,
}

fn present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    <Option<T> as serde::Deserialize>::deserialize(deserializer).map(Some)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let leave: OrganizationPatch = serde_json::from_str("{}")?;
    let clear: OrganizationPatch = serde_json::from_str(r#"{"organization_id":null}"#)?;
    let set: OrganizationPatch = serde_json::from_value(json!({"organization_id": Uuid::nil()}))?;
    assert!(leave.changeset_assignments().is_empty());
    assert!(
        update(users::table())
            .patch(&clear)
            .build()?
            .sql
            .ends_with("SET \"organization_id\" = NULL")
    );
    assert!(
        update(users::table())
            .patch(&set)
            .build()?
            .sql
            .ends_with("SET \"organization_id\" = $1")
    );
    const ACTIVE_IDS: &str = "active_ids";
    const INCOMING: &str = "incoming";
    const ORDERS_ALIAS: &str = "o";
    const USERS_ALIAS: &str = "u";

    let invoice_id = Uuid::nil();
    let customer_id = Uuid::nil();
    let amount = BigDecimal::from_str("19.99")?;
    let due_on = NaiveDate::from_ymd_opt(2026, 5, 1).ok_or("invalid sample due date")?;
    let paid_at = Utc::now();
    let new_invoice = NewInvoice {
        customer: customer_id,
        amount: amount.clone(),
        due_on,
        metadata: json!({ "source": "sample" }),
        paid_at: None,
        client_note: Some("local-only note".to_owned()),
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

    // NULL writes are explicit assignments; rqb keeps nullability checks in
    // Postgres because generated field metadata models SQL type, not optionality.
    let clear_paid_sql = update(invoices::table())
        .set(invoices::PAID_AT.set_null())
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    // DEFAULT writes let Postgres apply column defaults without modeling them
    // as Rust values. Use `default_values()` only for tables where every
    // omitted column has a database default or is nullable.
    let reset_state_sql = update(invoices::table())
        .set(invoices::STATE.set_default())
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    // PostgreSQL enum columns are generated as typed sqlx-backed Rust enums.
    let paid_invoices_sql = select(invoices::table())
        .filter(invoices::STATE.eq(InvoiceState::Paid))
        .build()?;
    assert!(invoices::STATE.meta.json.is_none());

    let delete_sql = delete_from(invoices::table())
        .filter(invoices::ID.eq(invoice_id))
        .returning(invoices::ID)
        .build()?;

    let raw_sql = raw("SELECT ?::numeric + ?::numeric")
        .bind(amount.clone())
        .bind(BigDecimal::from_str("5.00")?)
        .build()?;

    // Conflict targets accept fields or generated constraint name constants.
    // Predicates are allowed only for column targets.
    let upsert_user_sql = insert(users::table())
        .set_many((
            users::ID.set(Uuid::nil()),
            users::EMAIL.set("ada@example.com"),
            users::DISPLAY_NAME.set("Ada"),
            users::STATUS.set("active"),
        ))
        .on_conflict(users::EMAIL)
        .target_where(users::ACTIVE.eq(true))
        .do_update_set_where(
            (
                users::DISPLAY_NAME.set_excluded(),
                users::STATUS.set("active"),
            ),
            users::ACTIVE.eq(true),
        )
        .returning_all()
        .build()?;

    let ignore_duplicate_sql = insert(users::table())
        .set_many((
            users::ID.set(Uuid::nil()),
            users::EMAIL.set("ada@example.com"),
            users::DISPLAY_NAME.set("Ada"),
        ))
        .on_conflict_constraint(users::constraints::APP_USERS_EMAIL_KEY)
        .do_nothing()
        .returning((users::ID, users::EMAIL))
        .build()?;

    // `INSERT ... SELECT` stays typed on both sides and validates target column
    // count against the select projection before rendering SQL.
    let seed_open_orders_sql = insert(orders::table())
        .from_select(
            (
                orders::ID,
                orders::USER_ID,
                orders::STATUS,
                orders::TOTAL_CENTS,
            ),
            select(users::table())
                .expr(Uuid::nil())
                .column(users::ID)
                .expr("open")
                .expr(1000_i64)
                .filter(users::ACTIVE.eq(true)),
        )
        .returning(orders::ID)
        .build()?;

    assert_eq!(
        insert_sql.sql,
        "INSERT INTO \"sample\".\"invoices\" (\"id\", \"customer_id\", \"amount\", \"due_on\", \"metadata\") VALUES ($1, $2, $3, $4, $5) RETURNING \"invoices\".\"id\", \"invoices\".\"invoice_no\", \"invoices\".\"customer_id\", \"invoices\".\"state\", \"invoices\".\"amount\", \"invoices\".\"tax_rate\", \"invoices\".\"amount_history\", \"invoices\".\"due_on\", \"invoices\".\"issued_at\", \"invoices\".\"paid_at\", \"invoices\".\"reminder_time\", \"invoices\".\"cutoff_time\", \"invoices\".\"grace_period\", \"invoices\".\"service_days\", \"invoices\".\"billing_window\", \"invoices\".\"client_ip\", \"invoices\".\"client_network\", \"invoices\".\"pdf\", \"invoices\".\"tags\", \"invoices\".\"metadata\""
    );
    assert_eq!(
        update_sql.sql,
        "UPDATE \"sample\".\"invoices\" SET \"paid_at\" = $1 WHERE \"id\" = $2 RETURNING \"id\""
    );
    assert_eq!(
        clear_paid_sql.sql,
        "UPDATE \"sample\".\"invoices\" SET \"paid_at\" = NULL WHERE \"id\" = $1 RETURNING \"id\""
    );
    assert_eq!(
        reset_state_sql.sql,
        "UPDATE \"sample\".\"invoices\" SET \"state\" = DEFAULT WHERE \"id\" = $1 RETURNING \"id\""
    );
    assert_eq!(reset_state_sql.params.len(), 1);
    assert_eq!(
        paid_invoices_sql.sql,
        "SELECT \"id\", \"invoice_no\", \"customer_id\", \"state\", \"amount\", \"tax_rate\", \"amount_history\", \"due_on\", \"issued_at\", \"paid_at\", \"reminder_time\", \"cutoff_time\", \"grace_period\", \"service_days\", \"billing_window\", \"client_ip\", \"client_network\", \"pdf\", \"tags\", \"metadata\" FROM \"sample\".\"invoices\" WHERE \"state\" = $1"
    );
    assert_eq!(paid_invoices_sql.params.len(), 1);
    assert_eq!(
        delete_sql.sql,
        "DELETE FROM \"sample\".\"invoices\" WHERE \"id\" = $1 RETURNING \"id\""
    );
    assert_eq!(raw_sql.sql, "SELECT $1::numeric + $2::numeric");

    // Conditional assignment helpers keep optional write branches in the
    // builder chain. Skipped branches do not leave dummy SQL behind.
    let maybe_display_name = Some("Ada Lovelace");
    let should_activate = true;
    let conditional_update_sql = update(users::table())
        .set_if(should_activate, users::STATUS.set("active"))
        .set_option(maybe_display_name, |name| users::DISPLAY_NAME.set(name))
        .filter(users::ID.eq(Uuid::nil()))
        .build()?;

    // For straightforward upserts, `do_update_excluded((...))` updates several
    // columns from the proposed row without repeating `.set_excluded()`.
    let excluded_upsert_sql = insert(users::table())
        .set_many((
            users::ID.set(Uuid::nil()),
            users::EMAIL.set("ada@example.com"),
            users::DISPLAY_NAME.set("Ada"),
            users::STATUS.set("active"),
        ))
        .on_conflict_constraint(users::constraints::APP_USERS_EMAIL_KEY)
        .do_update_excluded((users::DISPLAY_NAME, users::STATUS))
        .returning(users::ID)
        .build()?;

    // Conflict updates read the proposed row through PostgreSQL's EXCLUDED.
    let incoming_users = [NewUser {
        id: Uuid::nil(),
        organization_id: Uuid::nil(),
        email: "grace@example.com".to_owned(),
        status: "active".to_owned(),
        display_name: "Grace".to_owned(),
    }];
    let bulk_upsert_sql = insert(users::table())
        .values_many(&incoming_users)?
        .on_conflict_constraint(users::constraints::APP_USERS_EMAIL_KEY)
        .do_update_excluded((users::STATUS, users::DISPLAY_NAME))
        .returning(users::ID)
        .build()?;

    // MERGE is useful when the incoming relation drives both matched updates
    // and inserts. The source can be a typed VALUES source, a CTE, a table, or
    // a subquery.
    let merge_incoming = values_source(
        [(Uuid::nil(), "ada@example.com", "Ada Lovelace", "active")],
        INCOMING,
        (users::ID, users::EMAIL, users::DISPLAY_NAME, users::STATUS),
    );
    let merge_users_sql = merge_into(
        users::table().alias(USERS_ALIAS),
        merge_incoming,
        users::EMAIL
            .at(USERS_ALIAS)
            .eq_field(users::EMAIL.at(INCOMING)),
    )
    .when_matched()
    .update((
        users::DISPLAY_NAME.set_from(INCOMING),
        users::STATUS.set_from(INCOMING),
    ))
    .when_not_matched()
    .insert((
        users::ID.set_from(INCOMING),
        users::EMAIL.set_from(INCOMING),
        users::DISPLAY_NAME.set_from(INCOMING),
        users::STATUS.set_from(INCOMING),
    ))
    .returning_as(users::ID.at(USERS_ALIAS), "id")
    .returning_as(users::EMAIL.at(USERS_ALIAS), "email")
    .build()?;

    // Write CTEs and UPDATE ... FROM keep staging/query shape server-owned.
    let active_ids = select(users::table())
        .column(users::ID)
        .filter(users::ACTIVE.eq(true))
        .infer_cte(ACTIVE_IDS)?;
    let active_ids_source = active_ids.source();
    let update_from_cte_sql = update(users::table().alias(USERS_ALIAS))
        .with(active_ids)
        .set(users::STATUS.set("active"))
        .from(active_ids_source)
        .filter(users::ID.at(USERS_ALIAS).eq_field(users::ID.at(ACTIVE_IDS)))
        .returning_as(users::ID.at(USERS_ALIAS), "id")
        .build()?;

    let delete_using_sql = delete_from(orders::table().alias(ORDERS_ALIAS))
        .using(users::table().alias(USERS_ALIAS))
        .filter(
            orders::USER_ID
                .at(ORDERS_ALIAS)
                .eq_field(users::ID.at(USERS_ALIAS)),
        )
        .filter(users::STATUS.at(USERS_ALIAS).eq("disabled"))
        .returning_as(orders::ID.at(ORDERS_ALIAS), "id")
        .build()?;

    // Optimistic compare-and-swap is just an UPDATE with the expected current
    // state in WHERE and optional returning to distinguish a miss.
    let optimistic_status_sql = update(orders::table())
        .set(orders::STATUS.set("paid"))
        .filter(orders::ID.eq(Uuid::nil()))
        .filter(orders::STATUS.eq("open"))
        .returning((orders::ID, orders::STATUS))
        .build()?;

    // PostgreSQL 18 exposes old/new values in DML RETURNING; generated fields
    // can qualify those pseudo-relations without raw SQL.
    let old_new_returning_sql = update(orders::table())
        .set(orders::STATUS.set("paid"))
        .filter(orders::ID.eq(Uuid::nil()))
        .returning_as(orders::STATUS.old_value(), "old_status")
        .returning_as(orders::STATUS.new_value(), "new_status")
        .build()?;

    assert_eq!(
        conditional_update_sql.sql,
        "UPDATE \"sample\".\"app_users\" SET \"status\" = $1, \"display_name\" = $2 WHERE \"id\" = $3"
    );
    assert_eq!(
        excluded_upsert_sql.sql,
        "INSERT INTO \"sample\".\"app_users\" (\"id\", \"email\", \"display_name\", \"status\") VALUES ($1, $2, $3, $4) ON CONFLICT ON CONSTRAINT \"app_users_email_key\" DO UPDATE SET \"display_name\" = EXCLUDED.\"display_name\", \"status\" = EXCLUDED.\"status\" RETURNING \"id\""
    );
    assert_eq!(
        bulk_upsert_sql.sql,
        "INSERT INTO \"sample\".\"app_users\" (\"id\", \"organization_id\", \"email\", \"status\", \"display_name\") SELECT \"incoming\".\"id\", \"incoming\".\"organization_id\", \"incoming\".\"email\", \"incoming\".\"status\", \"incoming\".\"display_name\" FROM (VALUES ($1, $2, $3, $4, $5)) AS \"incoming\" (\"id\", \"organization_id\", \"email\", \"status\", \"display_name\") ON CONFLICT ON CONSTRAINT \"app_users_email_key\" DO UPDATE SET \"status\" = EXCLUDED.\"status\", \"display_name\" = EXCLUDED.\"display_name\" RETURNING \"id\""
    );
    assert_eq!(
        merge_users_sql.sql,
        "MERGE INTO \"sample\".\"app_users\" AS \"u\" USING (VALUES ($1, $2, $3, $4)) AS \"incoming\" (\"id\", \"email\", \"display_name\", \"status\") ON \"u\".\"email\" = \"incoming\".\"email\" WHEN MATCHED THEN UPDATE SET \"display_name\" = \"incoming\".\"display_name\", \"status\" = \"incoming\".\"status\" WHEN NOT MATCHED THEN INSERT (\"id\", \"email\", \"display_name\", \"status\") VALUES (\"incoming\".\"id\", \"incoming\".\"email\", \"incoming\".\"display_name\", \"incoming\".\"status\") RETURNING \"u\".\"id\" AS \"id\", \"u\".\"email\" AS \"email\""
    );
    assert_eq!(merge_users_sql.params.len(), 4);
    assert_eq!(
        update_from_cte_sql.sql,
        "WITH \"active_ids\" (\"id\") AS (SELECT \"id\" FROM \"sample\".\"app_users\" WHERE \"active\" = $1) UPDATE \"sample\".\"app_users\" AS \"u\" SET \"status\" = $2 FROM \"active_ids\" WHERE \"u\".\"id\" = \"active_ids\".\"id\" RETURNING \"u\".\"id\" AS \"id\""
    );
    assert_eq!(update_from_cte_sql.params.len(), 2);
    assert_eq!(
        delete_using_sql.sql,
        "DELETE FROM \"sample\".\"orders\" AS \"o\" USING \"sample\".\"app_users\" AS \"u\" WHERE (\"o\".\"user_id\" = \"u\".\"id\" AND \"u\".\"status\" = $1) RETURNING \"o\".\"id\" AS \"id\""
    );
    assert_eq!(delete_using_sql.params.len(), 1);
    assert_eq!(
        optimistic_status_sql.sql,
        "UPDATE \"sample\".\"orders\" SET \"status\" = $1 WHERE (\"id\" = $2 AND \"status\" = $3) RETURNING \"id\", \"status\""
    );
    assert_eq!(optimistic_status_sql.params.len(), 3);
    assert_eq!(
        old_new_returning_sql.sql,
        "UPDATE \"sample\".\"orders\" SET \"status\" = $1 WHERE \"id\" = $2 RETURNING \"old\".\"status\" AS \"old_status\", \"new\".\"status\" AS \"new_status\""
    );
    assert_eq!(old_new_returning_sql.params.len(), 2);

    // Formatting and range helpers stay in the typed expression layer; no raw
    // SQL is needed for common report columns.
    let invoice_report_sql = select(invoices::table())
        .columns((invoices::ID, invoices::DUE_ON))
        .expr_as(to_char(invoices::DUE_ON, "YYYY-MM-DD"), "due_day")
        .expr_as(range_lower(invoices::SERVICE_DAYS), "service_start")
        .filter(isempty(invoices::SERVICE_DAYS))
        .build()?;

    assert_eq!(
        invoice_report_sql.sql,
        "SELECT \"id\", \"due_on\", to_char(\"due_on\", $1) AS \"due_day\", lower(\"service_days\") AS \"service_start\" FROM \"sample\".\"invoices\" WHERE isempty(\"service_days\") IS TRUE"
    );
    assert_eq!(invoice_report_sql.params.len(), 1);
    assert_eq!(
        upsert_user_sql.sql,
        "INSERT INTO \"sample\".\"app_users\" (\"id\", \"email\", \"display_name\", \"status\") VALUES ($1, $2, $3, $4) ON CONFLICT (\"email\") WHERE \"active\" = $5 DO UPDATE SET \"display_name\" = EXCLUDED.\"display_name\", \"status\" = $6 WHERE \"active\" = $7 RETURNING \"app_users\".\"id\", \"app_users\".\"organization_id\", \"app_users\".\"email\", \"app_users\".\"status\", \"app_users\".\"display_name\", \"app_users\".\"active\", \"app_users\".\"created_at\""
    );
    assert_eq!(
        ignore_duplicate_sql.sql,
        "INSERT INTO \"sample\".\"app_users\" (\"id\", \"email\", \"display_name\") VALUES ($1, $2, $3) ON CONFLICT ON CONSTRAINT \"app_users_email_key\" DO NOTHING RETURNING \"id\", \"email\""
    );
    assert_eq!(
        seed_open_orders_sql.sql,
        "INSERT INTO \"sample\".\"orders\" (\"id\", \"user_id\", \"status\", \"total_cents\") SELECT $1, \"id\", $2, $3 FROM \"sample\".\"app_users\" WHERE \"active\" = $4 RETURNING \"id\""
    );

    println!("{}", insert_sql.sql);
    println!("{}", clear_paid_sql.sql);
    println!("{}", conditional_update_sql.sql);
    println!("{}", excluded_upsert_sql.sql);
    println!("{}", bulk_upsert_sql.sql);
    println!("{}", merge_users_sql.sql);
    println!("{}", update_from_cte_sql.sql);
    println!("{}", optimistic_status_sql.sql);
    println!("{}", invoice_report_sql.sql);
    println!("{}", upsert_user_sql.sql);
    Ok(())
}
