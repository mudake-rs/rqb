use rqb::prelude::*;
use rqb_sample_schema::app_users as users;
use rqb_sample_schema::orders;
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Database errors are normalized into structured variants so API code can
    // match by meaning instead of parsing message text.
    let unique = rqb::Error::UniqueViolation {
        constraint: Some("app_users_email_key".to_owned()),
        detail: Some("Key (email) already exists.".to_owned()),
        info: rqb::DbErrorInfo::default(),
    };
    assert_eq!(unique.code(), Some("23505"));
    assert_eq!(unique.constraint_name(), Some("app_users_email_key"));

    let foreign_key = rqb::Error::ForeignKeyViolation {
        constraint: Some("orders_user_fkey".to_owned()),
        detail: None,
        info: rqb::DbErrorInfo::default(),
    };
    assert_eq!(foreign_key.code(), Some("23503"));

    let retryable = rqb::Error::SerializationFailure {
        message: "could not serialize access".to_owned(),
        detail: None,
        hint: None,
        info: rqb::DbErrorInfo::default(),
    };
    assert!(retryable.is_retryable());

    let not_found = rqb::Error::from(sqlx::Error::RowNotFound);
    assert!(matches!(not_found, rqb::Error::NotFound));

    // Builder validation errors are raised before SQL is rendered.
    assert!(matches!(
        delete_from(orders::table()).build(),
        Err(rqb::Error::DeleteWithoutFilter)
    ));
    assert!(matches!(
        select(orders::table())
            .filter(orders::METADATA.gt(json!({ "tier": "gold" })))
            .build(),
        Err(rqb::Error::InvalidOperator { field, operator })
            if field == "metadata" && operator == "gt"
    ));

    let bad_cte = cte("bad", select(orders::table()).column(orders::ID), orders::ID)
        .columns(["id", "extra"]);
    assert!(matches!(
        select(bad_cte.source()).with(bad_cte).build(),
        Err(rqb::Error::InvalidCteShape { name, .. }) if name == "bad"
    ));

    // The focused sample stays database-free. The future below is created only
    // to compile-check the real service shape against a live sqlx executor.
    let pool = sqlx::PgPool::connect_lazy("postgres://rqb:rqb@localhost/rqb")?;
    let duplicate_insert_flow = create_user_and_match_db_error(&pool);
    drop(duplicate_insert_flow);

    println!("structured errors can be matched by variant");
    Ok(())
}

async fn create_user_and_match_db_error<'e>(db: impl PgExecutor<'e>) -> rqb::Result<()> {
    match insert(users::table())
        .set(users::ID.set(Uuid::nil()))
        .set(users::EMAIL.set("ada@example.com"))
        .set(users::STATUS.set("active"))
        .set(users::DISPLAY_NAME.set("Ada"))
        .execute(db)
        .await
    {
        Ok(_) => Ok(()),
        Err(rqb::Error::UniqueViolation { constraint, .. }) => {
            // Application code usually maps this to 409 Conflict; the optional
            // constraint name is useful for logs or field-specific API errors.
            let _constraint = constraint;
            Ok(())
        }
        Err(error) => Err(error),
    }
}
