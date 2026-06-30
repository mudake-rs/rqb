# rqb-cli

`rqb-cli` introspects a Postgres database and emits a compact
`rqb::schema!` module for use with the `rqb` query builder.

The crate installs a binary named `rqb`.

## Install

Install from crates.io:

```bash
cargo install rqb-cli
```

The package name is `rqb-cli`; the installed binary is `rqb`.

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

Use `--config` when the generated schema needs project-owned type mappings:

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --config rqb.toml \
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

Add `--report` when CI logs should show generation counts, raw-only columns,
and unused `type_map` entries:

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --config rqb.toml \
  --out src/schema.rs \
  --check \
  --report
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
Postgres enum types generate Rust enums with `sqlx::Type`, then enum columns
use those generated types:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "public.invoice_state")]
pub enum InvoiceState {
    #[sqlx(rename = "draft")]
    Draft,
    #[sqlx(rename = "paid")]
    Paid,
}

rqb::schema! {
    table public.invoices {
        state: "public.invoice_state" = InvoiceState,
    }
}
```

Schema crates that include generated enums need a direct `sqlx` dependency with
the `derive` and `postgres` features enabled. Enum typing is scoped to the
generated Postgres schema; enum types from other schemas safely fall back to
raw-only metadata.

Unknown domains and extension types stay raw-only metadata, which can still be
used in server-owned SQL expressions through `*_META.expr()` or
`*_META.at("alias")`. This keeps extension columns available for server-owned
operators without pretending they have a portable Rust `Field<T>` mapping.

Map project-owned domains, extensions, or other PostgreSQL types in TOML:

```toml
[type_map."bitcoin.uint256"]
rust = "crate::types::PgU256"
ops = "ordered"        # none | equality | ordered | text; default none
json = "text"          # optional; omit to hide from SearchRequest
array = true           # also map bitcoin.uint256[] to Vec<crate::types::PgU256>

[type_map."public.vector"]
rust = "pgvector::Vector"
ops = "none"

[raw_only]
allow = [
  "public.vector_documents.search_index",
]
```

`rust` must be a qualified Rust type path and is emitted inline; the generator
does not add imports. The Rust type must implement the sqlx Postgres traits
required by the way it is used (`Type`, `Encode`, `Decode`, and array support
for `array = true`). rqb does not perform custom conversion.
`json` exposes only scalar columns to `SearchRequest`; generated array fields
stay JSON-hidden even when `array = true`.

When raw-only columns are generated, `rqb-cli` prints a stderr summary with the
relation, column, and Postgres type name. Treat that as a review queue for
project-specific enums, domains, ranges, and extension types that may deserve
manual raw helpers or future generator support.

`--deny-raw-only` turns that review queue into a CI failure unless every
raw-only column is listed in `[raw_only].allow` as `schema.relation.column`.
`--deny-unused-type-map` fails when a configured `[type_map."schema.type"]`
entry is not used by the selected `--schema` / `--table` scope. By default,
unused entries are warnings so stale config is visible without breaking local
generation.

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

Non-deferrable primary-key and unique constraints generate a relation-local
`constraints` module:

```rust
rqb::schema! {
    table public.users {
        id: uuid = Uuid,
        email: text = String,
        constraints {
            USERS_EMAIL_KEY: "users_email_key",
        }
    }
}

insert(users::table())
    .on_conflict_constraint(users::constraints::USERS_EMAIL_KEY)
    .do_nothing();
```

## Boundaries

`rqb-cli` generates schema metadata, not application models. Row structs, API
DTOs, validation, redaction, and write commands stay in application code.

Nullable columns keep their normal `Field<T>` type in generated metadata. Use
`Option<T>` in `sqlx::FromRow` structs when the database column can return
`NULL`.

Foreign keys, check constraints, expression indexes, partial indexes, and
non-unique indexes are not introspected into the generated API. They remain
database constraints; runtime violations are mapped by `rqb::Error` when sqlx
returns them.

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
