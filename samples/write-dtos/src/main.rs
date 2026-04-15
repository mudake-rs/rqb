use rqb::prelude::*;
use rqb_sample_base::{ACME_ORG_ID, UserStatus, schema::app_users};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct UserProfile {
    country: String,
    score: i64,
}

#[derive(Debug, WriteRecord)]
#[rqb(fields = app_users)]
struct CreateUserRequest {
    id: Uuid,
    organization_id: Uuid,
    #[rqb(field = app_users::EMAIL)]
    login: String,
    status: UserStatus,
    #[rqb(json)]
    profile: UserProfile,
    tags: Vec<String>,
    #[rqb(skip)]
    request_id: String,
}

#[derive(Debug, WriteRecord)]
#[rqb(fields = app_users, skip_none)]
struct PatchUserRequest {
    #[rqb(field = app_users::STATUS)]
    new_status: Option<UserStatus>,
    #[rqb(json)]
    profile: Option<UserProfile>,
    #[rqb(skip)]
    request_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserRow {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: UserStatus,
    profile: serde_json::Value,
    tags: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;
    let user_id = Uuid::new_v4();

    let create = CreateUserRequest {
        id: user_id,
        organization_id: rqb_sample_base::uuid(ACME_ORG_ID),
        login: format!("write-dto-{user_id}@example.com"),
        status: UserStatus::Active,
        profile: UserProfile {
            country: "NL".to_owned(),
            score: 90,
        },
        tags: vec!["sample".to_owned(), "write-dto".to_owned()],
        request_id: "req-create-1".to_owned(),
    };

    println!("handling {}", create.request_id);
    let inserted = insert(app_users::dataset())
        .value(&create)
        .returning([
            app_users::ID.into(),
            app_users::ORGANIZATION_ID.alias("organization_id"),
            app_users::EMAIL.into(),
            app_users::STATUS.into(),
            app_users::PROFILE.into(),
            app_users::TAGS.into(),
            app_users::CREATED_AT.alias("created_at"),
        ])
        .fetch_one_as::<UserRow>(&db)
        .await?;
    println!("inserted: {}", serde_json::to_string_pretty(&inserted)?);

    let patch = PatchUserRequest {
        new_status: Some(UserStatus::Disabled),
        profile: Some(UserProfile {
            country: "DE".to_owned(),
            score: 75,
        }),
        request_id: "req-patch-1".to_owned(),
    };

    println!("handling {}", patch.request_id);
    let updated = update(app_users::dataset())
        .set_from(&patch)
        .filter(app_users::ID.eq(user_id))
        .returning([
            app_users::ID.into(),
            app_users::ORGANIZATION_ID.alias("organization_id"),
            app_users::EMAIL.into(),
            app_users::STATUS.into(),
            app_users::PROFILE.into(),
            app_users::TAGS.into(),
            app_users::CREATED_AT.alias("created_at"),
        ])
        .fetch_one_as::<UserRow>(&db)
        .await?;
    println!("updated: {}", serde_json::to_string_pretty(&updated)?);

    delete(app_users::dataset())
        .filter(app_users::ID.eq(user_id))
        .execute(&db)
        .await?;

    Ok(())
}
