use rqb::prelude::*;

mod users {
    use rqb::prelude::*;

    pub static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    pub static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::text());
    pub static STATUS_META: Meta = Meta::new("status", "status", "text").ops(OpSet::text());
    pub static NICKNAME_META: Meta = Meta::new("nickname", "nickname", "text").ops(OpSet::text());
    pub static TYPE_META: Meta = Meta::new("type", "type", "text").ops(OpSet::text());

    pub const ID: Field<i32> = Field::new(&ID_META);
    pub const EMAIL: Field<String> = Field::new(&EMAIL_META);
    pub const STATUS: Field<String> = Field::new(&STATUS_META);
    pub const NICKNAME: Field<String> = Field::new(&NICKNAME_META);
    pub const TYPE: Field<String> = Field::new(&TYPE_META);

    pub static FIELDS: [&Meta; 5] = [
        &ID_META,
        &EMAIL_META,
        &STATUS_META,
        &NICKNAME_META,
        &TYPE_META,
    ];

    pub fn table() -> Source {
        rqb::table("public.users", &FIELDS)
    }
}

#[derive(Insertable)]
#[rqb(table = users)]
struct NewUser {
    email: String,
    #[rqb(field = STATUS)]
    state: String,
    r#type: String,
    #[rqb(skip_none)]
    nickname: Option<String>,
    #[rqb(skip)]
    _local_note: String,
}

#[derive(Insertable)]
#[rqb(table = users)]
struct OptionalInsert {
    #[rqb(skip_none)]
    nickname: Option<String>,
}

#[derive(Changeset)]
#[rqb(table = users)]
struct UserChanges {
    email: Option<String>,
    status: Option<String>,
    #[rqb(field = NICKNAME)]
    display_name: String,
}

#[test]
fn insertable_derive_maps_struct_fields_to_assignments() {
    let new_user = NewUser {
        email: "ada@example.com".to_owned(),
        state: "active".to_owned(),
        r#type: "admin".to_owned(),
        nickname: None,
        _local_note: "not persisted".to_owned(),
    };

    let built = insert(users::table())
        .set(users::ID.set(1))
        .values(&new_user)
        .returning(users::ID)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"users\" (\"id\", \"email\", \"status\", \"type\") VALUES ($1, $2, $3, $4) RETURNING \"id\""
    );
    assert_eq!(built.params.len(), 4);
}

#[test]
fn insertable_derive_outputs_expected_assignments() {
    let new_user = NewUser {
        email: "ada@example.com".to_owned(),
        state: "active".to_owned(),
        r#type: "admin".to_owned(),
        nickname: Some("Ada".to_owned()),
        _local_note: "not persisted".to_owned(),
    };

    let assignments = new_user.insert_assignments();
    let fields = assignments
        .iter()
        .map(|assignment| assignment.field.db)
        .collect::<Vec<_>>();

    assert_eq!(fields, ["email", "status", "type", "nickname"]);
    assert!(
        assignments.iter().all(|assignment| matches!(
            assignment.value,
            AssignmentValue::Expr(ValueExpr::Param(_))
        ))
    );
}

#[test]
fn insertable_derive_includes_skip_none_fields_when_present() {
    let new_user = NewUser {
        email: "ada@example.com".to_owned(),
        state: "active".to_owned(),
        r#type: "admin".to_owned(),
        nickname: Some("Ada".to_owned()),
        _local_note: "not persisted".to_owned(),
    };

    let built = insert(users::table())
        .values(&new_user)
        .returning(users::NICKNAME)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"users\" (\"email\", \"status\", \"type\", \"nickname\") VALUES ($1, $2, $3, $4) RETURNING \"nickname\""
    );
    assert_eq!(built.params.len(), 4);
}

#[test]
fn insertable_batch_insert_builds_values_source_from_dtos() {
    let users = vec![
        NewUser {
            email: "ada@example.com".to_owned(),
            state: "active".to_owned(),
            r#type: "admin".to_owned(),
            nickname: Some("Ada".to_owned()),
            _local_note: "not persisted".to_owned(),
        },
        NewUser {
            email: "grace@example.com".to_owned(),
            state: "active".to_owned(),
            r#type: "member".to_owned(),
            nickname: Some("Grace".to_owned()),
            _local_note: "not persisted".to_owned(),
        },
    ];

    let built = insert(users::table())
        .values_many(&users, "incoming")
        .unwrap()
        .returning(users::ID)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "INSERT INTO \"public\".\"users\" (\"email\", \"status\", \"type\", \"nickname\") SELECT \"incoming\".\"email\", \"incoming\".\"status\", \"incoming\".\"type\", \"incoming\".\"nickname\" FROM (VALUES ($1, $2, $3, $4), ($5, $6, $7, $8)) AS \"incoming\" (\"email\", \"status\", \"type\", \"nickname\") RETURNING \"id\""
    );
    assert_eq!(built.params.len(), 8);
}

#[test]
fn insertable_batch_insert_rejects_empty_batches() {
    let users = Vec::<NewUser>::new();

    let err = insert(users::table())
        .values_many(users, "incoming")
        .err()
        .unwrap();

    assert!(matches!(
        err,
        rqb::Error::InvalidInsertShape { message }
            if message == "batch insert requires at least one row"
    ));
}

#[test]
fn insertable_batch_insert_rejects_different_row_shapes() {
    let users = vec![
        NewUser {
            email: "ada@example.com".to_owned(),
            state: "active".to_owned(),
            r#type: "admin".to_owned(),
            nickname: Some("Ada".to_owned()),
            _local_note: "not persisted".to_owned(),
        },
        NewUser {
            email: "grace@example.com".to_owned(),
            state: "active".to_owned(),
            r#type: "member".to_owned(),
            nickname: None,
            _local_note: "not persisted".to_owned(),
        },
    ];

    let err = insert(users::table())
        .values_many(&users, "incoming")
        .err()
        .unwrap();

    assert!(matches!(
        err,
        rqb::Error::InvalidInsertShape { message }
            if message == "batch insert rows must use the same fields in the same order"
    ));
}

#[test]
fn insertable_batch_insert_rejects_empty_row_assignments() {
    let users = [OptionalInsert { nickname: None }];

    let err = insert(users::table())
        .values_many(&users, "incoming")
        .err()
        .unwrap();

    assert!(matches!(
        err,
        rqb::Error::InvalidInsertShape { message }
            if message == "batch insert rows must contain at least one assignment"
    ));
}

#[test]
fn insertable_batch_insert_rejects_existing_insert_values() {
    let users = [NewUser {
        email: "ada@example.com".to_owned(),
        state: "active".to_owned(),
        r#type: "admin".to_owned(),
        nickname: Some("Ada".to_owned()),
        _local_note: "not persisted".to_owned(),
    }];

    let err = insert(users::table())
        .set(users::ID.set(1))
        .values_many(&users, "incoming")
        .err()
        .unwrap();

    assert!(matches!(
        err,
        rqb::Error::InvalidInsertShape { message }
            if message == "batch insert cannot be combined with existing insert values or source"
    ));
}

#[test]
fn changeset_derive_outputs_expected_assignments() {
    let changes = UserChanges {
        email: Some("ada@example.com".to_owned()),
        status: None,
        display_name: "Ada".to_owned(),
    };

    let assignments = changes.changeset_assignments();
    let fields = assignments
        .iter()
        .map(|assignment| assignment.field.db)
        .collect::<Vec<_>>();

    assert_eq!(fields, ["email", "nickname"]);
    assert!(
        assignments.iter().all(|assignment| matches!(
            assignment.value,
            AssignmentValue::Expr(ValueExpr::Param(_))
        ))
    );
}

#[test]
fn changeset_derive_skips_none_option_fields() {
    let changes = UserChanges {
        email: Some("ada@example.com".to_owned()),
        status: None,
        display_name: "Ada".to_owned(),
    };

    let built = update(users::table())
        .patch(&changes)
        .filter(users::ID.eq(1))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "UPDATE \"public\".\"users\" SET \"email\" = $1, \"nickname\" = $2 WHERE \"id\" = $3"
    );
    assert_eq!(built.params.len(), 3);
}
