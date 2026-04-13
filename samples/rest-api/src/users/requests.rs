use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::{
    db::{
        UserStatus,
        users::{CreateUser, UserListQuery, UserPatch, UserProfile},
    },
    pagination::{DEFAULT_LIMIT, DEFAULT_OFFSET},
};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserListParams {
    pub status: Option<UserStatus>,
    #[validate(length(min = 1, max = 32))]
    pub tag: Option<String>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
    #[validate(range(min = 0))]
    pub offset: Option<u64>,
}

impl UserListParams {
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(DEFAULT_LIMIT)
    }

    pub fn offset(&self) -> u64 {
        self.offset.unwrap_or(DEFAULT_OFFSET)
    }
}

impl From<UserListParams> for UserListQuery {
    fn from(params: UserListParams) -> Self {
        let limit = params.limit();
        let offset = params.offset();
        Self {
            status: params.status,
            tag: params.tag,
            limit,
            offset,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub organization_id: Uuid,
    #[validate(email)]
    pub email: String,
    pub status: Option<UserStatus>,
    pub profile: Option<UserProfile>,
    #[validate(length(max = 16))]
    pub tags: Option<Vec<String>>,
}

impl From<CreateUserRequest> for CreateUser {
    fn from(request: CreateUserRequest) -> Self {
        Self {
            organization_id: request.organization_id,
            email: request.email,
            status: request.status.unwrap_or(UserStatus::Active),
            profile: request.profile.unwrap_or_default(),
            tags: request.tags.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_patch_user"))]
#[serde(rename_all = "camelCase")]
pub struct PatchUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(email)]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<UserStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<UserProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 16))]
    pub tags: Option<Vec<String>>,
}

impl From<PatchUserRequest> for UserPatch {
    fn from(request: PatchUserRequest) -> Self {
        Self {
            email: request.email,
            status: request.status,
            profile: request.profile,
            tags: request.tags,
        }
    }
}

fn validate_patch_user(patch: &PatchUserRequest) -> Result<(), ValidationError> {
    // The API rejects no-op PATCH requests before building an UPDATE.
    if patch.email.is_some()
        || patch.status.is_some()
        || patch.profile.is_some()
        || patch.tags.is_some()
    {
        Ok(())
    } else {
        let mut error = ValidationError::new("empty_patch");
        error.message = Some("patch body has no fields".into());
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Uuid {
        Uuid::parse_str(value).unwrap()
    }

    #[test]
    fn create_user_request_converts_to_db_model_with_defaults() {
        let request = CreateUserRequest {
            organization_id: id("00000000-0000-0000-0000-000000000001"),
            email: "ada@example.com".to_owned(),
            status: None,
            profile: None,
            tags: None,
        };
        request.validate().unwrap();

        let user = CreateUser::from(request);
        assert_eq!(user.status, UserStatus::Active);
        assert_eq!(user.tags, Vec::<String>::new());
        assert!(user.profile.country.is_none());
    }

    #[test]
    fn user_list_params_convert_to_query_with_defaults() {
        let params = UserListParams {
            status: Some(UserStatus::Active),
            tag: Some("vip".to_owned()),
            limit: None,
            offset: None,
        };
        params.validate().unwrap();

        let query = UserListQuery::from(params);
        assert_eq!(query.status, Some(UserStatus::Active));
        assert_eq!(query.tag.as_deref(), Some("vip"));
        assert_eq!(query.limit, DEFAULT_LIMIT);
        assert_eq!(query.offset, DEFAULT_OFFSET);
    }

    #[test]
    fn user_request_rejects_unknown_status_bad_email_and_empty_patch() {
        let create = serde_json::json!({
            "organizationId": "00000000-0000-0000-0000-000000000001",
            "email": "ada@example.com",
            "status": "paused"
        });
        assert!(serde_json::from_value::<CreateUserRequest>(create).is_err());

        let create = CreateUserRequest {
            organization_id: id("00000000-0000-0000-0000-000000000001"),
            email: "not-an-email".to_owned(),
            status: Some(UserStatus::Active),
            profile: None,
            tags: None,
        };
        assert!(create.validate().is_err());

        let patch = PatchUserRequest {
            email: None,
            status: None,
            profile: None,
            tags: None,
        };
        assert!(patch.validate().is_err());
    }
}
