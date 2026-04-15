use rqb::prelude::*;
use rqb_sample_base::{ACME_ORG_ID, UserStatus, schema::app_users};
use uuid::Uuid;

#[derive(Debug, WriteRecord)]
#[rqb(fields = app_users)]
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
    let organization_id = rqb_sample_base::uuid(ACME_ORG_ID);
    let user_id = Uuid::new_v4();
    let email = format!("taken-{user_id}@example.com");

    // 1. Turn "no rows" into a normal Option for read endpoints.
    let missing_user = select(app_users::dataset())
        .fields([app_users::ID, app_users::EMAIL, app_users::STATUS])
        .filter(app_users::ID.eq(Uuid::new_v4()))
        .fetch_one(&db)
        .await
        .optional()?;
    println!("1. missing user exists: {}", missing_user.is_some());

    // 2. Bad query shapes fail during validation, before SQL reaches Postgres.
    if let Err(rqb::Error::Core(error)) = select(app_users::dataset())
        .filter(field("doesNotExist").eq("x"))
        .build_pg()
    {
        println!("2. request validation failed: {error}");
    }

    // 3. Constraint errors come back as structured rqb::Error variants.
    insert(app_users::dataset())
        .value(&new_user(user_id, organization_id, &email))
        .execute(&db)
        .await?;

    let duplicate = insert(app_users::dataset())
        .value(&new_user(Uuid::new_v4(), organization_id, &email))
        .execute(&db)
        .await;

    if let Err(error) = duplicate {
        if let rqb::Error::UniqueViolation { .. } = error {
            println!("3. duplicate email rejected: {}", error);
            println!("   sqlstate: {}", error.code().unwrap_or("unknown"));
            println!(
                "   constraint: {}",
                error.constraint_name().unwrap_or("unknown")
            );
        } else {
            return Err(error.into());
        }
    }

    // 4. Foreign key errors are also structured. The constraint name is useful
    // for logs or for rare cases where one statement can hit several constraints.
    let missing_organization = insert(app_users::dataset())
        .value(&new_user(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "missing-org@example.com",
        ))
        .execute(&db)
        .await;

    if let Err(error) = missing_organization {
        if let rqb::Error::ForeignKeyViolation { .. } = error {
            println!("4. missing organization rejected: {}", error);
            println!("   sqlstate: {}", error.code().unwrap_or("unknown"));
            println!(
                "   constraint: {}",
                error.constraint_name().unwrap_or("unknown")
            );
        } else {
            return Err(error.into());
        }
    }

    delete(app_users::dataset())
        .filter(app_users::ID.eq(user_id))
        .execute(&db)
        .await?;

    Ok(())
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
