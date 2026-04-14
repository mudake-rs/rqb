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
    #[error("organization does not exist")]
    MissingOrganization,
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
    let third_id = Uuid::new_v4();

    insert(app_users::dataset())
        .value(&new_user(first_id, rqb_sample_base::uuid(ACME_ORG_ID), &email))
        .execute(&db)
        .await?;

    let duplicate = insert(app_users::dataset())
        .value(&new_user(second_id, rqb_sample_base::uuid(ACME_ORG_ID), &email))
        .execute(&db)
        .await
        .on_constraint("app_users_email_key", map_email_taken);
    println!("duplicate insert mapped to: {:?}", duplicate.unwrap_err());

    let bad_organization = insert(app_users::dataset())
        .value(&new_user(third_id, Uuid::new_v4(), "missing-org@example.com"))
        .execute(&db)
        .await
        .on_constraint("app_users_organization_id_fkey", map_missing_organization);
    println!(
        "foreign key violation mapped to: {:?}",
        bad_organization.unwrap_err()
    );

    let missing = select(app_users::dataset())
        .filter(app_users::ID.eq(second_id))
        .fetch_one_as::<User>(&db)
        .await
        .optional()?;
    println!("missing user as option: {missing:?}");

    retry_serializable_profile_update(&db, first_id).await?;

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

async fn retry_serializable_profile_update(db: &Db, user_id: Uuid) -> rqb::Result<()> {
    const MAX_ATTEMPTS: usize = 3;

    for attempt in 0..MAX_ATTEMPTS {
        let result = async {
            let tx = db.begin().serializable().await?;
            update(app_users::dataset())
                .set(
                    app_users::PROFILE,
                    serde_json::json!({
                        "source": "error-handling-sample",
                        "retryAttempt": attempt,
                    }),
                )
                .filter(app_users::ID.eq(user_id))
                .execute(&tx)
                .await?;
            tx.commit().await
        }
        .await;

        match result {
            Ok(()) => return Ok(()),
            Err(error) if error.is_retryable() && attempt + 1 < MAX_ATTEMPTS => continue,
            Err(error) => return Err(error),
        }
    }

    unreachable!("retry loop always returns success or the final error")
}

fn map_email_taken(_: &rqb::Error) -> AppError {
    AppError::EmailTaken
}

fn map_missing_organization(_: &rqb::Error) -> AppError {
    AppError::MissingOrganization
}

fn new_user(id: Uuid, organization_id: Uuid, email: &str) -> NewUser {
    NewUser {
        id,
        organization_id,
        email: email.to_owned(),
        status: UserStatus::Active,
        profile: serde_json::json!({ "source": "error-handling-sample" }),
        tags: vec!["sample".to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_constraints_to_application_errors() {
        let duplicate = Err::<u64, _>(rqb::Error::UniqueViolation {
            constraint: Some("app_users_email_key".to_owned()),
            detail: None,
            info: Default::default(),
        })
        .on_constraint("app_users_email_key", map_email_taken)
        .unwrap_err();
        assert!(matches!(duplicate, AppError::EmailTaken));

        let missing_org = Err::<u64, _>(rqb::Error::ForeignKeyViolation {
            constraint: Some("app_users_organization_id_fkey".to_owned()),
            detail: None,
            info: Default::default(),
        })
        .on_constraint("app_users_organization_id_fkey", map_missing_organization)
        .unwrap_err();
        assert!(matches!(missing_org, AppError::MissingOrganization));
    }

    #[test]
    fn unrelated_constraints_stay_database_errors() {
        let error = Err::<u64, _>(rqb::Error::UniqueViolation {
            constraint: Some("other_key".to_owned()),
            detail: None,
            info: Default::default(),
        })
        .on_constraint("app_users_email_key", map_email_taken)
        .unwrap_err();

        assert!(matches!(error, AppError::Db(rqb::Error::UniqueViolation { .. })));
    }
}
