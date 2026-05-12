use rqb::prelude::*;
use uuid::Uuid;

rqb::schema! {
    table public.users {
        id: uuid = Uuid,
        email: text = String,
        "type" as TYPE: text = String,
        "source" as SOURCE: text = String,
        "source_" as SOURCE_COL: text = String,
        tags: "text[]" = Vec<String>,
        embedding: vector,
        "clone" as CLONE: text = String,
        "into" as INTO: text = String,
    }

    view public.user_search {
        id: uuid = Uuid,
        email: text = String,
    }
}

#[allow(dead_code)]
struct UserStatus;

rqb::schema! {
    table public.custom_status {
        id: uuid = Uuid,
        status: text = UserStatus,
        state: invoice_state,
    }
}

rqb::schema! {
    view "audit-log.Event Stream" {
        "event-type": text = String,
    }
}

rqb::schema! {
    table public.method_names {
        id: uuid = Uuid,
        as_ref: text = String,
        eq: text = String,
        ne: text = String,
        from: text = String,
        hash: text = String,
    }
}

rqb::schema! {
    table public.users as public_users {
        id: uuid = Uuid,
    }

    table admin.users as admin_users {
        id: uuid = Uuid,
    }
}

#[test]
fn schema_macro_generates_table_metadata_and_fields() {
    let built = select(users::table())
        .column(users::ID)
        .column(users::EMAIL)
        .column(users::TYPE)
        .filter(users::ID.eq(Uuid::nil()))
        .filter(users::EMAIL.eq("ada@example.com"))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email\", \"type\" FROM \"public\".\"users\" WHERE (\"id\" = $1 AND \"email\" = $2)"
    );
    assert_eq!(built.params.len(), 2);
    assert_eq!(users::FIELDS.len(), 9);
    assert_eq!(users::FIELDS[6].api, "embedding");
    assert!(users::FIELDS[6].json.is_none());
    assert!(!users::FIELDS[6].ops.equality);
    assert_eq!(users::FIELDS[7].api, "clone");
    assert_eq!(users::FIELDS[8].api, "into");
    assert_eq!(users::ID.meta.json, Some(JsonKind::Uuid));
    assert!(users::EMAIL.meta.ops.ordering);
    assert!(users::EMAIL.meta.ops.pattern);
}

#[test]
fn schema_macro_generates_view_source() {
    let built = select(user_search::view())
        .column(user_search::ID)
        .build()
        .unwrap();

    assert_eq!(built.sql, "SELECT \"id\" FROM \"public\".\"user_search\"");
}

#[test]
fn schema_macro_generates_alias_bound_field_accessors() {
    let u = users::alias("u");
    let s = user_search::alias("s");

    let built = select(&u)
        .join(&s, u.id().eq_field(s.id()))
        .column(u.email())
        .column(u.type_())
        .column(u.source_())
        .column(u.source_col())
        .column(u.clone_())
        .column(u.into_())
        .filter(u.email().eq("ada@example.com"))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"email\" AS \"u_email\", \"u\".\"type\" AS \"u_type\", \"u\".\"source\" AS \"u_source\", \"u\".\"source_\" AS \"u_source_\", \"u\".\"clone\" AS \"u_clone\", \"u\".\"into\" AS \"u_into\" FROM \"public\".\"users\" AS \"u\" JOIN \"public\".\"user_search\" AS \"s\" ON \"u\".\"id\" = \"s\".\"id\" WHERE \"u\".\"email\" = $1"
    );
    assert_eq!(built.params.len(), 1);

    let source: Source = u.clone().into();
    let built = select(source).column(users::ID.at("u")).build().unwrap();
    assert_eq!(
        built.sql,
        "SELECT \"u\".\"id\" AS \"u_id\" FROM \"public\".\"users\" AS \"u\""
    );
}

#[test]
fn schema_macro_allows_explicit_module_aliases() {
    let public = select(public_users::table())
        .column(public_users::ID)
        .build()
        .unwrap();
    let admin = select(admin_users::table())
        .column(admin_users::ID)
        .build()
        .unwrap();

    assert_eq!(public.sql, "SELECT \"id\" FROM \"public\".\"users\"");
    assert_eq!(admin.sql, "SELECT \"id\" FROM \"admin\".\"users\"");
}

#[test]
fn schema_macro_allows_custom_rust_type_and_raw_only_columns() {
    let built = select(custom_status::table())
        .column(custom_status::ID)
        .column(custom_status::STATUS)
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\" FROM \"public\".\"custom_status\""
    );
    assert_eq!(custom_status::FIELDS.len(), 3);
    assert_eq!(custom_status::FIELDS[2].api, "state");
    assert_eq!(custom_status::FIELDS[2].pg, "invoice_state");
    assert!(custom_status::FIELDS[2].json.is_none());
}

#[test]
fn schema_macro_derives_modules_from_quoted_relation_names() {
    let e = event_stream::alias("e");

    let built = select(&e).column(e.event_type()).build().unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"e\".\"event-type\" AS \"e_event-type\" FROM \"audit-log\".\"Event Stream\" AS \"e\""
    );
}

#[test]
fn schema_macro_suffixes_alias_methods_that_collide_with_common_traits() {
    let m = method_names::alias("m");

    let built = select(&m)
        .column(m.as_ref_())
        .column(m.eq_())
        .column(m.ne_())
        .column(m.from_())
        .column(m.hash_())
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"m\".\"as_ref\" AS \"m_as_ref\", \"m\".\"eq\" AS \"m_eq\", \"m\".\"ne\" AS \"m_ne\", \"m\".\"from\" AS \"m_from\", \"m\".\"hash\" AS \"m_hash\" FROM \"public\".\"method_names\" AS \"m\""
    );
}
