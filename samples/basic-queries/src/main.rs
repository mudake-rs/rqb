use chrono::{DateTime, Utc};
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
    // Multiple filters compose with AND. `and([...])` is the explicit helper
    // when a nested predicate group reads better than a chain of `.filter(...)`.
    // The projection is intentionally narrow for a small response DTO.
    let composed = select(users::table())
        .columns((users::ID, users::EMAIL))
        .filter(and([
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

    // `and([...])` and `or([...])` can be nested to express grouped boolean
    // logic without raw SQL: active users whose status is active, or invited
    // users that already have a display name.
    // `.columns(...)` keeps the rendered SQL tied to this smaller row shape.
    let nested = select(users::table())
        .columns((users::ID, users::EMAIL))
        .filter(and([
            users::ACTIVE.eq(true),
            or([
                users::STATUS.eq("active"),
                and([
                    users::STATUS.eq("invited"),
                    users::DISPLAY_NAME.is_not_null(),
                ]),
            ]),
        ]))
        .build()?;

    assert_eq!(
        nested.sql,
        "SELECT \"id\", \"email\" FROM \"sample\".\"app_users\" WHERE (\"active\" = $1 AND (\"status\" = $2 OR (\"status\" = $3 AND \"display_name\" IS NOT NULL)))"
    );
    assert_eq!(nested.params.len(), 3);

    // `.or_filter(...)` and `.or_filter_option(...)` are useful for fallback
    // search branches without building the OR tree by hand.
    let fallback_email = Some("ops@example.com");
    // This is another intentional subset projection, not required boilerplate.
    let disjunctive = select(users::table())
        .columns((users::ID, users::EMAIL))
        .filter(users::ACTIVE.eq(true))
        .or_filter(users::STATUS.eq("invited"))
        .or_filter_option(fallback_email, |email| users::EMAIL.eq(email))
        .offset(5)
        .build()?;

    assert_eq!(
        disjunctive.sql,
        "SELECT \"id\", \"email\" FROM \"sample\".\"app_users\" WHERE (\"active\" = $1 OR \"status\" = $2 OR \"email\" = $3) OFFSET $4"
    );
    assert_eq!(disjunctive.params.len(), 4);

    println!("{}", simple.sql);
    println!("{}", composed.sql);
    println!("{}", nested.sql);
    println!("{}", disjunctive.sql);
    Ok(())
}
