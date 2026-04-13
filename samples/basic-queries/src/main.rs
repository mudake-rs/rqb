use rqb::prelude::*;
use rqb_sample_base::{ACME_ORG_ID, UserStatus, schema::app_users};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    status: UserStatus,
    profile: serde_json::Value,
    tags: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct UserId {
    id: Uuid,
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let user_id = Uuid::new_v4();
    let new_user = NewUser {
        id: user_id,
        organization_id: rqb_sample_base::uuid(ACME_ORG_ID),
        email: format!("alice-{user_id}@example.com"),
        status: UserStatus::Active,
        profile: serde_json::json!({ "country": "NL", "score": 100 }),
        tags: vec!["sample".to_owned()],
    };

    insert(app_users::dataset())
        .value(&new_user)
        .execute(&db)
        .await?;

    let active_users = select(app_users::dataset())
        .filter(app_users::STATUS.eq(UserStatus::Active))
        .order_by(app_users::EMAIL.asc())
        .fetch_as::<User>(&db)
        .await?;
    println!("active users: {active_users:#?}");

    let matching_ids = select(app_users::dataset())
        .fields([app_users::ID])
        .filter(all([
            app_users::STATUS.eq(UserStatus::Active),
            any([
                app_users::EMAIL.ends_with("@example.com"),
                app_users::PROFILE.path("country").eq("NL"),
            ]),
            not(app_users::TAGS.has("blocked")),
        ]))
        .order_by(app_users::CREATED_AT.desc())
        .fetch_as::<UserId>(&db)
        .await?;
    let id_values = matching_ids.iter().map(|row| row.id).collect::<Vec<_>>();
    println!("matching user ids only: {id_values:#?}");

    let disabled = update(app_users::dataset())
        .set(app_users::STATUS, UserStatus::Disabled)
        .filter(app_users::ID.eq(user_id))
        .fetch_one_as::<User>(&db)
        .await?;
    println!("updated user: {disabled:#?}");

    delete(app_users::dataset())
        .filter(app_users::ID.eq(user_id))
        .execute(&db)
        .await?;
    println!("deleted user {user_id}");

    Ok(())
}
