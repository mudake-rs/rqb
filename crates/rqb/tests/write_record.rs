use rqb::{ElemType, Field, FieldType, Value, WriteRecord, insert, serde::Serialize};
use rqb_core::ValidatedInsert;

mod users {
    use super::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const EMAIL: Field = Field::new("email", FieldType::Text);
    pub const PROFILE: Field = Field::new("profile", FieldType::Jsonb);
    pub const AVATAR: Field = Field::new("avatar", FieldType::Bytea);
    pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text));
}

mod metrics {
    use super::*;

    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const RATIO: Field = Field::new("ratio", FieldType::Float);
}

#[derive(Clone, Serialize)]
struct Profile {
    country: String,
}

#[derive(WriteRecord)]
#[rqb(fields = users)]
struct NewUser {
    id: String,
    email: String,
    #[rqb(json)]
    profile: Profile,
    #[rqb(bytes)]
    avatar: Vec<u8>,
    tags: Vec<String>,
}

#[derive(WriteRecord)]
#[rqb(fields = users, skip_none)]
struct UserPatch {
    email: Option<String>,
    #[rqb(json)]
    profile: Option<Profile>,
}

#[derive(WriteRecord)]
#[rqb(fields = users)]
struct MixedUserUpdate {
    #[rqb(skip_none)]
    email: Option<String>,
    profile: Option<serde_json::Value>,
}

#[derive(WriteRecord)]
#[rqb(fields = users)]
struct NullableUser {
    #[rqb(json)]
    profile: Option<Profile>,
    #[rqb(bytes)]
    avatar: Option<Vec<u8>>,
}

#[derive(WriteRecord)]
#[rqb(fields = metrics)]
struct NewMetric {
    id: String,
    ratio: f64,
}

#[test]
fn write_record_derive_maps_fields_directly() {
    let row = NewUser {
        id: "10000000-0000-0000-0000-000000000001".to_owned(),
        email: "ada@example.com".to_owned(),
        profile: Profile {
            country: "NL".to_owned(),
        },
        avatar: vec![1, 2, 3],
        tags: vec!["admin".to_owned(), "beta".to_owned()],
    };

    let fields = row.write_fields().unwrap();

    assert_eq!(fields[0], (users::ID, Value::from(row.id)));
    assert_eq!(fields[1], (users::EMAIL, Value::from(row.email)));
    assert_eq!(
        fields[2],
        (
            users::PROFILE,
            Value::Json(serde_json::json!({ "country": "NL" }))
        )
    );
    assert_eq!(fields[3], (users::AVATAR, Value::Bytes(vec![1, 2, 3])));
    assert_eq!(
        fields[4],
        (
            users::TAGS,
            Value::Array(vec![Value::from("admin"), Value::from("beta")])
        )
    );
}

#[test]
fn write_record_derive_skip_none_skips_absent_option_fields() {
    let patch = UserPatch {
        email: None,
        profile: Some(Profile {
            country: "DE".to_owned(),
        }),
    };

    assert_eq!(
        patch.write_fields().unwrap(),
        vec![(
            users::PROFILE,
            Value::Json(serde_json::json!({ "country": "DE" }))
        )]
    );
}

#[test]
fn write_record_derive_field_skip_none_skips_only_marked_option_fields() {
    let fields = MixedUserUpdate {
        email: None,
        profile: None,
    }
    .write_fields()
    .unwrap();

    assert_eq!(fields, vec![(users::PROFILE, Value::Null)]);
}

#[test]
fn write_record_derive_maps_optional_json_and_bytes_to_sql_null() {
    let fields = NullableUser {
        profile: None,
        avatar: None,
    }
    .write_fields()
    .unwrap();

    assert_eq!(
        fields,
        vec![(users::PROFILE, Value::Null), (users::AVATAR, Value::Null)]
    );

    let fields = NullableUser {
        profile: Some(Profile {
            country: "FR".to_owned(),
        }),
        avatar: Some(vec![4, 5, 6]),
    }
    .write_fields()
    .unwrap();

    assert_eq!(
        fields,
        vec![
            (
                users::PROFILE,
                Value::Json(serde_json::json!({ "country": "FR" }))
            ),
            (users::AVATAR, Value::Bytes(vec![4, 5, 6])),
        ]
    );
}

#[test]
fn write_record_keeps_non_finite_float_for_validation() {
    let query = insert(rqb::Dataset::table("metrics").fields([metrics::ID, metrics::RATIO]))
        .value(&NewMetric {
            id: "10000000-0000-0000-0000-000000000001".to_owned(),
            ratio: f64::NAN,
        })
        .build()
        .unwrap();

    let err = ValidatedInsert::new(query).unwrap_err();

    assert!(matches!(err, rqb::CoreError::InvalidValue { .. }));
}
