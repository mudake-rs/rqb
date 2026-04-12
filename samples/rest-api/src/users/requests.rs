use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::{
    db::{
        USER_STATUS,
        users::{CreateUser, UserListQuery, UserPatch, UserProfile},
    },
    pagination::{DEFAULT_LIMIT, DEFAULT_OFFSET},
};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserListParams {
    #[validate(length(min = 1, max = 32), custom(function = "validate_user_status"))]
    pub status: Option<String>,
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
    #[validate(length(min = 1, max = 32), custom(function = "validate_user_status"))]
    pub status: Option<String>,
    pub profile: Option<UserProfile>,
    #[validate(length(max = 16))]
    pub tags: Option<Vec<String>>,
}

impl From<CreateUserRequest> for CreateUser {
    fn from(request: CreateUserRequest) -> Self {
        Self {
            organization_id: request.organization_id,
            email: request.email,
            status: request.status.unwrap_or_else(|| "active".to_owned()),
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
    #[validate(length(min = 1, max = 32), custom(function = "validate_user_status"))]
    pub status: Option<String>,
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

fn validate_user_status(status: &str) -> Result<(), ValidationError> {
    // USER_STATUS is re-exported from the CLI-generated schema, not duplicated by hand here.
    if USER_STATUS.contains(status) {
        Ok(())
    } else {
        let mut error = ValidationError::new("unknown_user_status");
        error.message = Some(format!("unknown user status `{status}`").into());
        Err(error)
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
