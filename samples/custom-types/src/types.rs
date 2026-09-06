/// Integer-cent codec; PostgreSQL's domain constraint rejects negative balances.
#[derive(Clone, Debug, PartialEq, sqlx::Type)]
#[sqlx(transparent)]
pub struct Cents(pub i64);
