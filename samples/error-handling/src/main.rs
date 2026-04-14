#![allow(clippy::result_large_err)]

use rqb::prelude::*;
use rqb_sample_base::{ACME_ORG_ID, UserStatus, schema::app_users};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
enum AppError {
    #[error("email is already taken")]
    EmailTaken,
    #[error(transparent)]
    Db(#[from] rqb::Error),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: UserStatus,
    profile: serde_json::Value,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewUser {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: UserStatus,
    profile: serde_json::Value,
    tags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let db = rqb_sample_base::connect().await?;
    let email = format!("taken-{}@example.com", Uuid::new_v4());
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();

    insert(app_users::dataset())
        .value(&new_user(first_id, &email))
        .execute(&db)
        .await?;

    let duplicate = insert(app_users::dataset())
        .value(&new_user(second_id, &email))
        .execute(&db)
        .await
        .on_constraint("app_users_email_key", |_| AppError::EmailTaken);
    println!("duplicate insert mapped to: {:?}", duplicate.unwrap_err());

    let missing = select(app_users::dataset())
        .filter(app_users::ID.eq(second_id))
        .fetch_one_as::<User>(&db)
        .await
        .optional()?;
    println!("missing user as option: {missing:?}");

    let validation = select(app_users::dataset())
        .filter(field("doesNotExist").eq("x"))
        .build_pg()
        .unwrap_err();
    println!("validation error before SQL execution: {validation}");

    delete(app_users::dataset())
        .filter(app_users::ID.eq(first_id))
        .execute(&db)
        .await?;

    Ok(())
}

fn new_user(id: Uuid, email: &str) -> NewUser {
    NewUser {
        id,
        organization_id: rqb_sample_base::uuid(ACME_ORG_ID),
        email: email.to_owned(),
        status: UserStatus::Active,
        profile: serde_json::json!({ "source": "error-handling-sample" }),
        tags: vec!["sample".to_owned()],
    }
}
