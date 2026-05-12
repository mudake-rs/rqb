# rqb-cli

`rqb-cli` introspects a Postgres database and emits a compact
`rqb::schema!` module for use with the `rqb` query builder.

The crate installs a binary named `rqb`.

## Install

Until crates.io publish, install from GitHub:

```bash
cargo install --git https://github.com/mudake-rs/rqb rqb-cli
```

The package name is `rqb-cli`; the installed binary is `rqb`.

After crates.io publish:

```bash
cargo install rqb-cli
```

When working inside the rqb repository, run it through Cargo instead:

```bash
cargo run -p rqb-cli -- generate --help
```

## Usage

Generate schema metadata from one Postgres schema:

```bash
rqb generate \
  --database-url postgres://user:pass@localhost:5432/dbname \
  --schema public \
  --out src/schema.rs
```

`--database-url` also reads `DATABASE_URL`:

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/dbname \
  rqb generate --schema public --out src/schema.rs
```

Limit output to selected tables, views, or materialized views with repeated
`--table` flags:

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --table users \
  --table orders \
  --out src/schema.rs
```

Preview generated code without writing a file:

```bash
rqb generate --database-url "$DATABASE_URL" --schema public --stdout
```

Fail CI when a checked-in schema file has drifted from the database:

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --out src/schema.rs \
  --check
```

Generated code is formatted with `rustfmt` by default. Pass `--no-rustfmt` when
`rustfmt` is not available on `PATH` or when you want raw generator output.
For drift checks, prefer the default formatted mode; `--check --no-rustfmt`
compares unformatted generator output against the file exactly as committed.

## Output

The output is one Rust file containing imports and a single `rqb::schema!`
invocation:

```rust
rqb::schema! {
    table public.users {
        id: uuid = Uuid,
        email: text = String,
    }
}
```

Known sqlx-supported Postgres types generate typed `Field<T>` constants.
Unknown or extension types stay raw-only metadata, which can still be used in
server-owned SQL expressions.

The generator annotates schema facts that matter at the `sqlx::FromRow` or
write boundary:

```rust
// Nullable in Postgres metadata. Use Option<T> in row structs.
paid_at: timestamptz = DateTime<Utc>,

// Generated: stored. Do not include in INSERT assignments.
line_total_cents: int8 = i64,

// Identity: always. Do not include in INSERT assignments; use OVERRIDING SYSTEM VALUE to override.
invoice_no: int8 = i64,

// Identity: by default. Explicit INSERT values are allowed.
sequence_no: int8 = i64,
```

Materialized views are emitted as `view` entries with a `// Materialized view.`
comment. `rqb` queries them like normal read sources.

## Boundaries

`rqb-cli` generates schema metadata, not application models. Row structs, API
DTOs, validation, redaction, and write commands stay in application code.

Nullable columns keep their normal `Field<T>` type in generated metadata. Use
`Option<T>` in `sqlx::FromRow` structs when the database column can return
`NULL`.

Primary keys, foreign keys, unique constraints, check constraints, and indexes
are not introspected into the generated API. They remain database constraints;
runtime violations are mapped by `rqb::Error` when sqlx returns them.

## Migrations

`rqb-cli` runs after migrations. It introspects the database that exists; it
does not generate migrations, diff schemas, or emit `ALTER TABLE`.

A typical CI check is:

1. Apply migrations with `sqlx::migrate!`, `sqlx migrate`, refinery, sqitch,
   psql, or your deployment system.
2. Verify the generated schema is current:

   ```bash
   rqb generate \
     --database-url "$DATABASE_URL" \
     --schema public \
     --out src/schema.rs \
     --check
   ```

## Versioning

`rqb-cli` follows the same 0.x versioning as `rqb`. Before 1.0, minor versions
may change CLI flags or generated output shape when that improves the API.
