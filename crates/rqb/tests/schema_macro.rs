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
    }

    view public.user_search {
        id: uuid = Uuid,
        email: text = String,
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
    assert_eq!(users::FIELDS.len(), 7);
    assert_eq!(users::FIELDS[6].api, "embedding");
    assert!(users::FIELDS[6].json.is_none());
    assert!(!users::FIELDS[6].ops.equality);
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
        .filter(u.email().eq("ada@example.com"))
        .build()
        .unwrap();

    assert_eq!(
        built.sql,
        "SELECT \"u\".\"email\" AS \"u_email\", \"u\".\"type\" AS \"u_type\", \"u\".\"source\" AS \"u_source\", \"u\".\"source_\" AS \"u_source_\" FROM \"public\".\"users\" AS \"u\" JOIN \"public\".\"user_search\" AS \"s\" ON \"u\".\"id\" = \"s\".\"id\" WHERE \"u\".\"email\" = $1"
    );
    assert_eq!(built.params.len(), 1);
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
