# Writes And Types

Writes use typed field assignments. Any Rust type supported by sqlx for
Postgres can be bound through `Param::typed` or `Field<T>`.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.
