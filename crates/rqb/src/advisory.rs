use crate::{RawStmt, raw};

/// PostgreSQL advisory lock key.
///
/// Advisory locks accept either one signed 64-bit key or a pair of signed
/// 32-bit keys. The pair form is useful for `(namespace, id)` layouts that make
/// accidental key collisions easier to reason about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum AdvisoryLockKey {
    /// One `bigint` advisory lock key.
    BigInt(i64),
    /// Two `int4` advisory lock keys.
    Pair(i32, i32),
}

impl From<i64> for AdvisoryLockKey {
    #[inline]
    fn from(key: i64) -> Self {
        Self::BigInt(key)
    }
}

impl From<(i32, i32)> for AdvisoryLockKey {
    #[inline]
    fn from((left, right): (i32, i32)) -> Self {
        Self::Pair(left, right)
    }
}

/// Builds `SELECT pg_advisory_xact_lock(...)`.
///
/// The lock is released automatically when the current transaction commits or
/// rolls back. Prefer this transaction-scoped helper over session-level
/// advisory locks when using a connection pool.
///
/// Execute this inside `tx!` or an explicit transaction. In auto-commit mode,
/// PostgreSQL releases the transaction-level lock at the end of this statement,
/// so it provides no mutual exclusion for follow-up work.
pub fn advisory_xact_lock(key: impl Into<AdvisoryLockKey>) -> RawStmt {
    advisory_lock_stmt("pg_advisory_xact_lock", key.into())
}

/// Builds `SELECT pg_try_advisory_xact_lock(...)`.
///
/// Fetch the scalar boolean result to check whether the lock was acquired.
///
/// Execute this inside `tx!` or an explicit transaction. In auto-commit mode,
/// PostgreSQL releases the transaction-level lock at the end of this statement,
/// so it provides no mutual exclusion for follow-up work.
pub fn try_advisory_xact_lock(key: impl Into<AdvisoryLockKey>) -> RawStmt {
    advisory_lock_stmt("pg_try_advisory_xact_lock", key.into())
}

/// Builds `SELECT pg_advisory_xact_lock(...)` from a stable string key hash.
///
/// PostgreSQL advisory locks do not accept strings directly. rqb hashes the
/// name with a stable FNV-1a 64-bit hash and uses the resulting `bigint` key.
/// Collisions are possible; use numeric keys for collision-critical protocols.
///
/// Execute this inside `tx!` or an explicit transaction. In auto-commit mode,
/// PostgreSQL releases the transaction-level lock at the end of this statement,
/// so it provides no mutual exclusion for follow-up work.
pub fn advisory_xact_lock_named(name: impl AsRef<str>) -> RawStmt {
    advisory_xact_lock(advisory_name_key(name.as_ref()))
}

/// Builds `SELECT pg_try_advisory_xact_lock(...)` from a stable string key hash.
///
/// PostgreSQL advisory locks do not accept strings directly. rqb hashes the
/// name with the same stable key derivation as [`advisory_xact_lock_named`].
///
/// Execute this inside `tx!` or an explicit transaction. In auto-commit mode,
/// PostgreSQL releases the transaction-level lock at the end of this statement,
/// so it provides no mutual exclusion for follow-up work.
pub fn try_advisory_xact_lock_named(name: impl AsRef<str>) -> RawStmt {
    try_advisory_xact_lock(advisory_name_key(name.as_ref()))
}

fn advisory_lock_stmt(function: &'static str, key: AdvisoryLockKey) -> RawStmt {
    match key {
        AdvisoryLockKey::BigInt(key) => raw(format!("SELECT {function}(?)")).bind(key),
        AdvisoryLockKey::Pair(left, right) => raw(format!("SELECT {function}(?, ?)"))
            .bind(left)
            .bind(right),
    }
}

fn advisory_name_key(name: &str) -> i64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash as i64
}

#[cfg(test)]
mod tests {
    use super::{
        advisory_name_key, advisory_xact_lock, advisory_xact_lock_named, try_advisory_xact_lock,
        try_advisory_xact_lock_named,
    };

    #[test]
    fn advisory_xact_lock_helpers_render_bigint_and_pair_keys() {
        let bigint = advisory_xact_lock(42_i64).build().unwrap();
        assert_eq!(bigint.sql, "SELECT pg_advisory_xact_lock($1)");
        assert_eq!(bigint.params.len(), 1);

        let pair = try_advisory_xact_lock((10_i32, 42_i32)).build().unwrap();
        assert_eq!(pair.sql, "SELECT pg_try_advisory_xact_lock($1, $2)");
        assert_eq!(pair.params.len(), 2);
    }

    #[test]
    fn named_advisory_lock_helpers_use_stable_fnv1a_key() {
        assert_eq!(
            advisory_name_key("billing:invoice:123"),
            6695150957860309103
        );

        let lock = advisory_xact_lock_named("billing:invoice:123")
            .build()
            .unwrap();
        assert_eq!(lock.sql, "SELECT pg_advisory_xact_lock($1)");
        assert_eq!(lock.params.len(), 1);

        let try_lock = try_advisory_xact_lock_named("billing:invoice:123")
            .build()
            .unwrap();
        assert_eq!(try_lock.sql, "SELECT pg_try_advisory_xact_lock($1)");
        assert_eq!(try_lock.params.len(), 1);
    }
}
