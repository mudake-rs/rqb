# Changelog

All notable changes to rqb are tracked here before release entries are cut.

## [Unreleased]

- Cleaned up the pre-public builder surface: `apply_search` /
  `replace_search`, symmetric `or_filter*` / `replace_filter` on update and
  delete, a smaller row-lock shortcut set, and no aggregate-only `agg` aliases.
- Added explicit null writes with `null()` and `Field<T>::set_null()`.
- Made invalid aggregate-local modifiers fail validation instead of silently
  doing nothing on non-aggregate expressions.
- Reshaped `rqb::Error` so large database payloads are boxed, keeping
  application `Result<T, rqb::Error>` values small.
- Added competitor-parity helpers: `sum_distinct`, `avg_distinct`,
  `trim_array`, `range_merge`, and `multirange_merge`.
- Renamed collection predicate methods to receiver-disambiguated names such as
  `contains`, `overlaps`, `has_key`, `has_any_keys`, and `has_all_keys`.
- Introduced `FunctionSource` so `WITH ORDINALITY` is available only on
  table-valued function sources.
- Added pool-owned streaming methods:
  `fetch_stream_pool`, `fetch_stream_pool_as`, and
  `fetch_stream_pool_scalar`.
- Added `#[non_exhaustive]` armor to expanding public AST and payload types.
