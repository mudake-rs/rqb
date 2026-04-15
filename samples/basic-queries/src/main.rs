use chrono::{DateTime, Utc};
use rqb::prelude::*;
use uuid::Uuid;

mod schema;

use schema::app_users as users;

#[derive(Debug)]
#[allow(dead_code)]
struct UserRow {
    id: Uuid,
    email: String,
    status: String,
    created_at: DateTime<Utc>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let built = select(users::table())
        .column(users::ID)
        .column(users::EMAIL)
        .column(users::STATUS)
        .filter(users::STATUS.eq("active"))
        .order_desc(users::CREATED_AT)
        .limit(20)
        .build()?;

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email\", \"status\" FROM \"sample\".\"app_users\" WHERE \"status\" = $1 ORDER BY \"created_at\" DESC LIMIT $2"
    );
    assert_eq!(built.params.len(), 2);

    println!("{}", built.sql);
    Ok(())
}
