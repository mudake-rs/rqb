use rqb::prelude::*;

mod users {
    use rqb::prelude::*;

    pub static ID_META: Meta = Meta::new("id", "id", "uuid")
        .ops(OpSet::ordered())
        .json(JsonKind::Uuid);
    pub static EMAIL_META: Meta = Meta::new("email", "email", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    pub static STATUS_META: Meta = Meta::new("status", "status", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    pub static CREATED_AT_META: Meta = Meta::new("createdAt", "created_at", "timestamptz")
        .ops(OpSet::ordered())
        .json(JsonKind::Timestamptz);

    pub const ID: Field<rqb::uuid::Uuid> = Field::new(&ID_META);
    pub const EMAIL: Field<String> = Field::new(&EMAIL_META);
    pub const STATUS: Field<String> = Field::new(&STATUS_META);
    pub const CREATED_AT: Field<rqb::chrono::DateTime<rqb::chrono::Utc>> =
        Field::new(&CREATED_AT_META);

    pub static FIELDS: [&Meta; 4] = [&ID_META, &EMAIL_META, &STATUS_META, &CREATED_AT_META];

    pub fn table() -> Source {
        rqb::table("public.app_users", &FIELDS)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct UserRow {
    id: rqb::uuid::Uuid,
    email: String,
    status: String,
    created_at: rqb::chrono::DateTime<rqb::chrono::Utc>,
}

fn main() -> rqb::Result<()> {
    let built = select(users::table())
        .column(users::ID)
        .column(users::EMAIL)
        .column(users::STATUS)
        .filter(users::STATUS.eq("active"))
        .order_desc(users::CREATED_AT)
        .limit(20)
        .build()?;

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"email\", \"status\" FROM \"public\".\"app_users\" WHERE \"status\" = $1 ORDER BY \"created_at\" DESC LIMIT $2"
    );
    assert_eq!(built.params.len(), 2);

    println!("{}", built.sql);
    Ok(())
}
