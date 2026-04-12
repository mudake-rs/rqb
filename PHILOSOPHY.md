# rqb Philosophy

rqb is a Postgres-first runtime query builder for Rust services.

It exists for one reason: real application queries should be easy to write, easy to compose, and safe to expose through constrained JSON search APIs. rqb is not an ORM, not a Diesel clone, and not a generic SQL abstraction for every database.

## The Core Idea

Application code owns the query shape.

The server builds tables, views, joins, CTEs, subqueries, raw server-owned sources, aggregates, locks, and writes in Rust. A client JSON `SearchRequest` can then choose fields, filters, sort, limit, and offset inside that server-owned scope.

That gives Knex-like runtime composition without handing arbitrary SQL to clients.

## Ergonomics First

The primary goal is maximum application-code ergonomics.

Rust query code should be readable and direct:

```rust
select(orders())
    .filter(STATUS.eq("paid"))
    .filter_option(params.min_total, |value| TOTAL_CENTS.gte(value))
    .order_by(CREATED_AT.desc())
    .page_as::<Order>(&db)
    .await?;
```

If the user has to fight generic types, wrapper structs, derive maze, or trait-bound errors for ordinary queries, the API is wrong.

rqb is not public-stable yet. Break APIs freely when it improves ergonomics, clarity, or architecture. Do not keep awkward APIs for compatibility theater. When an API changes, update all call sites, docs, examples, samples, generated flow, and tests in the same change.

## Runtime Validation, Not Type Gymnastics

rqb chooses runtime validation over Diesel-style compile-time proof.

This is intentional. Runtime query composition, JSON search requests, dynamic fields, raw server-owned sources, and CTE-heavy application queries should be first-class. The tradeoff is acceptable only because validation is strict:

- dataset metadata describes allowed fields, types, capabilities, enum values, JSON path policy, and limits
- invalid field/operator/type combinations fail before rendering
- user values become Postgres parameters
- SQL rendering assumes validated input
- validation errors must be structured and understandable

The goal is not "anything goes at runtime". The goal is flexible composition with a strong metadata contract.

## Postgres First

rqb should use Postgres well instead of flattening it into lowest-common-denominator SQL.

First-class Postgres features include:

- JSONB and JSON paths
- arrays
- Postgres enums
- UUID and timestamptz
- CTEs
- views and raw server-owned sources
- subqueries and `EXISTS`
- `DISTINCT ON`
- row locks
- `RETURNING`
- `ON CONFLICT`
- aggregates such as `json_agg`

If generic SQL support conflicts with Postgres ergonomics, choose Postgres ergonomics.

## Type Ownership

rqb should own broad Postgres type support.

Core Postgres types and widely used Postgres extension types are library responsibility, not application boilerplate. Application code should not need to invent custom adapters for normal Postgres features.

Examples of library-owned types:

- `uuid`
- `jsonb`
- `date`, `timestamp`, and `timestamptz`
- exact `numeric` / decimal values
- arrays
- Postgres enums
- domains
- `citext`
- `inet`, `cidr`, and `macaddr`
- `ltree`
- ranges and multiranges
- full-text search types such as `tsvector` and `tsquery`
- `hstore`
- `pgvector`
- PostGIS geometry/geography when the project is ready for that surface area

Project-specific types are different. A domain such as `uint_256`, a business-specific scalar, or an application-specific wrapper should not become a hardcoded rqb variant. rqb must instead provide a clean metadata extension point that lets application schema declare:

- Postgres type identity
- base family, such as numeric, text, json, vector, range, or geometry
- accepted input representation
- selected output representation
- allowed operators
- cast / selection behavior
- validation rules that protect JSON requests

The target shape is declarative. This is architecture direction, not a stable API promise; exact names should change if better ergonomics emerge before public release.

```rust
pub const UINT_256: TypeSpec = TypeSpec::domain(Some("public"), "uint_256")
    .base(TypeFamily::Numeric)
    .value_repr(ValueRepr::DecimalString)
    .select_repr(SelectRepr::Text)
    .operators(OperatorSet::numeric());

pub const AMOUNT: Field = Field::new("amount", FieldType::Custom(&UINT_256));
```

Application code should then stay ordinary:

```rust
select(withdrawals())
    .filter(AMOUNT.gte("7000000"))
    .order_by(AMOUNT.desc())
    .fetch_as::<Withdrawal>(&db)
    .await?;
```

Do not solve project-specific types by forcing every field to carry Diesel-style serialize/deserialize attributes, by requiring `Box<dyn ToSql>` in normal query code, or by treating unknown database types as text.

## Metadata Is The Contract

`Dataset`, `Field`, `FieldType`, capabilities, enum metadata, JSON path policy, limits, and relations are the contract between:

- Rust builder API
- JSON request API
- validation
- Postgres rendering
- execution / row mapping
- generated schema
- docs and samples

New features must flow through that contract. A feature that only renders SQL but cannot be validated is incomplete.

## Extensibility Is Core

rqb must not become a closed list of hardcoded types and operators.

Adding new library-owned Postgres types, project-specific scalar/domain types, Rust mappings, operators, and error mappings should have an obvious path:

```text
type metadata
-> value conversion
-> capability/type validation
-> Postgres cast / parameter binding
-> row mapping
-> tests and docs
```

The architecture should make room for:

- built-in Postgres type support in the library
- popular extension type support in the library
- project-specific domains and scalars through metadata
- custom Rust enum/scalar mappings without per-field boilerplate
- custom operators when metadata declares them safe
- backend-specific fast paths without leaking backend details into `rqb-core`

Extension points must preserve the safety model: server-owned SQL shape, metadata-constrained client input, and parameterized values. If adding a type or operator requires scattered hacks, the architecture is wrong.

Exact values must stay exact. Postgres `numeric`, numeric-like domains, uint256-style values, and other high-precision scalars must not silently pass through `f64`. Prefer lossless string/decimal representations first; add binary codecs later only when they improve performance without weakening the API.

## Errors Are Part Of Ergonomics

Errors are user-facing API, not an afterthought.

rqb errors must be:

- typed
- structured
- match-friendly
- clear in human messages
- easy to map into HTTP/application errors
- usable without parsing strings

Database errors should expose useful structure such as constraint, column, table, code, detail, and hint when available.

Validation errors should identify the dataset/source, field, operator, expected capability/type, actual value/type, and whether the issue came from request or builder context when possible.

Bad:

```text
invalid query
```

Good:

```text
field `metadata.score` on `orders` cannot be sorted because JSON paths are not sortable
```

Common application mapping must stay simple:

```rust
match err {
    rqb::postgres::Error::NotFound => AppError::not_found(),
    rqb::postgres::Error::UniqueViolation { constraint, .. }
        if constraint.as_deref() == Some("users_email_key") =>
    {
        AppError::conflict("email already exists")
    }
    _ => AppError::internal(err),
}
```

Helper methods are welcome when they remove real boilerplate.

## Escape Hatches Are Explicit

Raw SQL is necessary. It must be safe by design:

- raw SQL is server-owned
- values are bound, not interpolated
- bind counts are validated
- raw sources declare their fields
- JSON requests cannot introduce raw SQL

Escape hatches should expand what trusted server code can express without weakening untrusted input boundaries.

## Generated Code Should Be Usable

CLI codegen is part of the product.

Generated schema can be verbose, but it must be readable enough to trust and ergonomic enough to use directly. If the sample has to manually patch generated schema, that is usually a generator/design bug.

Generated code should expose useful metadata, Postgres enums, relation helpers, JSONB policy, array defaults, and dataset constructors.

Generated code must not lie about database types. Unknown types should produce a clear generation error or an explicit opt-in fallback, not silent `Text`. Domains and extension types should be introspected as their own schema contract whenever Postgres exposes enough metadata.

## Sample Is Product Proof

The sample app is not decorative. It is a pressure test for the API.

If the sample needs ugly DTO conversions, service contortions, manual enum validation that metadata could provide, or awkward transaction patterns, treat that as evidence against the library API.

The sample should show realistic service code without becoming a full framework.

## Performance Matters After Shape Is Right

rqb should not be slow by accident. Hot paths matter:

- validation / field resolution
- rendering
- placeholder renumbering
- parameter conversion
- row mapping
- executor and pool paths

But before public release, API shape and ergonomics are more important than micro-optimizations. Optimize the hot path without making the public API worse.

Performance claims need evidence or a benchmark plan. Do not invent percentages.

## What rqb Is Not

rqb is not:

- an ORM
- a migration framework
- a Diesel type-system clone
- a generic SQL dialect abstraction
- a replacement for all raw SQL
- a framework that owns application architecture

rqb should compose with normal Rust services, not force the service to become an rqb application.

## Short Version

rqb is a Postgres-first runtime query builder for Rust that makes complex server-owned SQL shapes and safe JSON search ergonomic.

Optimize for application developer happiness, strict runtime validation, explicit escape hatches, structured errors, and extensibility. Break APIs freely before public release to get there.
