# Numeric Policy

rqb treats exact numbers and floating-point numbers as different things.

This is intentional. Postgres `numeric`, numeric domains, `uint_256`-style
values, money-like amounts, balances, and IDs encoded as decimal strings must not
silently pass through `f64`.

## Model

| rqb type | Postgres type | Rust input | Output mapping | Precision policy |
| --- | --- | --- | --- | --- |
| `FieldType::Integer` | `int4` | integer values fitting `i32` | JSON number / `i32` | exact |
| `FieldType::BigInt` | `int8` | integer values fitting `i64` | JSON number / `i64` | exact within `i64` |
| `FieldType::Float` | `double precision` | `f32`, `f64`, integer literals | JSON number / `f64` | floating-point |
| `FieldType::Numeric` | `numeric` | integer literals, decimal strings | JSON string / `String` | exact transport |
| `FieldType::Custom(TypeSpec)` with `ValueRepr::DecimalString` | domain / custom numeric | integer literals, decimal strings | JSON string / `String` or newtype | exact transport |

The user-facing rule is:

```rust
RATIO.eq(0.75)       // Float: OK
PRICE.eq("19.99")   // Numeric: OK, exact
PRICE.eq(1999)      // Numeric: OK, exact integer
PRICE.eq(19.99)     // Rejected: f64 already lost decimal intent before rqb sees it
```

Exact numeric fields reject implicit `f64` values. If lossy floating-point
behavior is intentional, use a `Float` field or an explicit cast / raw SQL escape
hatch. Decimal strings must be finite decimal literals; `NaN`, `Infinity`, and
`-Infinity` are rejected on metadata-backed exact numeric fields.

## Why Not BigDecimal By Default?

Diesel and sqlx map Postgres `numeric` through decimal crates:

- Diesel uses `bigdecimal` behind its `numeric` feature.
- sqlx supports `rust_decimal` and `BigDecimal`.
- SeaQuery has optional `with-rust_decimal` and `with-bigdecimal` value variants.

rqb uses a different default because it is a runtime query builder for services
and JSON APIs:

- no mandatory decimal dependency;
- no forced application-level decimal type;
- no `rust_decimal` 29-digit ceiling;
- no failed decode for valid Postgres `numeric` values wider than the chosen Rust
  decimal type;
- JSON responses can safely carry exact numbers as strings.

Applications that need arithmetic can parse the string into the decimal type they
want at the application boundary. rqb's job is to avoid losing precision while
building, executing, and mapping queries.

Optional decimal integrations can be added later, but the default path stays
lossless string transport.

## Rendering

For exact numeric fields, rqb binds a text representation and casts in SQL:

```sql
$1::text::numeric
$2::text::public.uint_256
```

That SQL is more verbose than Diesel/sqlx because rqb is metadata-driven at
runtime. The tradeoff is explicit and acceptable: validation knows the field type,
the bind value stays parameterized, and no runtime catalog lookup is required for
domains.

For integer and float fields, rqb uses concrete Postgres bind holders:

```text
Integer      -> BindParam::Int4      -> $1::int
BigInt       -> BindParam::Int8      -> $1::bigint
Float        -> BindParam::Float8    -> $1::double precision
Array(Int)   -> BindParam::Int4Array -> $1::int[]
```

## Expressions And Promotion

rqb supports server-owned value expressions such as `coalesce`, `case_when`,
`greatest`, `least`, set queries, and window defaults. These expressions need a
common output type.

The target promotion rules are conservative:

- `Integer + BigInt -> BigInt`
- `Integer/BigInt + Numeric -> Numeric`
- `Numeric + Float -> reject` unless the user casts explicitly
- `Custom numeric domain + integer -> custom numeric domain`
- `Custom decimal-string domain + Float -> reject`
- text families can coerce to text where Postgres semantics are still clear

The expression validator follows these rules for server-owned expressions.

## Aggregates

`SUM` and `AVG` must preserve exactness when their input is exact.

Target aggregate behavior:

| Input field | `sum` output | `avg` output |
| --- | --- | --- |
| `Integer` | `BigInt` | `Numeric` |
| `BigInt` | `Numeric` | `Numeric` |
| `Numeric` | `Numeric` | `Numeric` |
| decimal-string custom domain | exact string-backed numeric output | exact string-backed numeric output |
| `Float` | `Float` | `Float` |

The renderer casts exact aggregate outputs to text for serde mapping, so
`SUM(numeric)` and `AVG(bigint)` can deserialize into `String` without a decimal
crate or precision loss.

## Raw SQL

`raw_query` does not use dataset metadata. Cast raw outputs to the type you want:

```sql
SELECT SUM(amount)::text AS "totalAmount"   -- exact
SELECT AVG(score)::float8 AS "averageScore" -- intentionally lossy float
```

For raw inputs, cast placeholders explicitly:

```sql
WHERE amount > ?::text::numeric
WHERE ratio > ?::double precision
```

This keeps raw SQL honest: the SQL author owns the type decision.
