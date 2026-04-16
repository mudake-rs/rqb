use chrono::{DateTime, Utc};
use rqb::dsl::all;
use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use uuid::Uuid;

#[derive(Debug)]
#[allow(dead_code)]
struct UserRow {
    id: Uuid,
    email: String,
    status: String,
    created_at: DateTime<Utc>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // No projection calls: rqb renders all known root fields instead of `SELECT *`.
    let simple = select(users::table())
        .filter(users::STATUS.eq("active"))
        .order_desc(users::CREATED_AT)
        .limit(20)
        .build()?;

    assert_eq!(
        simple.sql,
        "SELECT \"id\", \"organization_id\", \"email\", \"status\", \"display_name\", \"active\", \"created_at\" FROM \"sample\".\"app_users\" WHERE \"status\" = $1 ORDER BY \"created_at\" DESC LIMIT $2"
    );
    assert_eq!(simple.params.len(), 2);

    let email_fragment = Some("@example.com");
    let composed = select(users::table())
        .column(users::ID)
        .column(users::EMAIL)
        .filter(all([
            users::STATUS.in_list(["active", "invited"]),
            users::DISPLAY_NAME.is_not_null(),
        ]))
        .filter_if(true, users::ACTIVE.eq(true))
        .filter_option(email_fragment, |value| users::EMAIL.contains(value))
        .order_asc_nulls_last(users::CREATED_AT)
        .limit(10)
        .build()?;

    assert_eq!(
        composed.sql,
        "SELECT \"id\", \"email\" FROM \"sample\".\"app_users\" WHERE (\"status\" IN ($1, $2) AND \"display_name\" IS NOT NULL AND \"active\" = $3 AND \"email\" ILIKE $4 ESCAPE '\\') ORDER BY \"created_at\" ASC NULLS LAST LIMIT $5"
    );
    assert_eq!(composed.params.len(), 5);

    println!("{}", simple.sql);
    println!("{}", composed.sql);
    Ok(())
}
