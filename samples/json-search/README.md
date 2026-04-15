# JSON Search

`SearchRequest` accepts client filters, sort, limit, and offset. Rust still owns
the table, projection, joins, raw SQL, and server filters.

The schema comes from the shared `rqb-sample-schema` crate generated from
`../schema.sql`.
