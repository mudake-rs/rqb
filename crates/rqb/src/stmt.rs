pub(super) use crate::{
    BindValue, BoolExpr, Cte, Field, FieldRef, IntoFieldMetas, Join, JoinKind, Meta, Param, Source,
    ValueExpr, cte, raw as raw_sql, subquery,
};
pub(super) use crate::{Error, Result};

macro_rules! impl_filter_methods {
    ($ty:ty) => {
        impl $ty {
            /// Adds a `WHERE` predicate, composing with existing predicates using `AND`.
            #[inline]
            pub fn filter(mut self, filter: BoolExpr) -> Self {
                self.filter = Some(BoolExpr::and_option(self.filter, filter));
                self
            }

            /// Adds a `WHERE` predicate, composing with existing predicates using `OR`.
            ///
            /// Use `filter(or([...]))` when only part of the current predicate tree
            /// should be OR-grouped.
            #[inline]
            pub fn or_filter(mut self, filter: BoolExpr) -> Self {
                self.filter = Some(BoolExpr::or_option(self.filter, filter));
                self
            }

            /// Replaces the entire `WHERE` predicate.
            #[inline]
            pub fn replace_filter(mut self, filter: BoolExpr) -> Self {
                self.filter = Some(filter);
                self
            }

            /// Adds a `WHERE` predicate only when `condition` is true.
            #[inline]
            pub fn filter_if(self, condition: bool, filter: BoolExpr) -> Self {
                if condition { self.filter(filter) } else { self }
            }

            /// Adds a `WHERE` predicate built from an optional value.
            pub fn filter_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> BoolExpr) -> Self {
                match value {
                    Some(value) => self.filter(f(value)),
                    None => self,
                }
            }

            /// Adds an OR-composed `WHERE` predicate only when `condition` is true.
            #[inline]
            pub fn or_filter_if(self, condition: bool, filter: BoolExpr) -> Self {
                if condition {
                    self.or_filter(filter)
                } else {
                    self
                }
            }

            /// Adds an OR-composed `WHERE` predicate built from an optional value.
            pub fn or_filter_option<T>(
                self,
                value: Option<T>,
                f: impl FnOnce(T) -> BoolExpr,
            ) -> Self {
                match value {
                    Some(value) => self.or_filter(f(value)),
                    None => self,
                }
            }
        }
    };
}

mod ast;
mod conflict;
mod constructors;
mod delete;
mod helpers;
mod insert;
mod items;
mod merge;
mod raw_stmt;
mod select;
mod set;
mod update;
mod validate;

pub use ast::{
    Assignment, Changeset, ColumnConflictBuilder, ConflictAction, ConflictClause, ConflictFields,
    ConflictTarget, ConstraintConflictBuilder, Delete, FetchClause, GroupByItem, Insert,
    Insertable, IntoAssignments, LockMode, LockWait, MatchedMergeBuilder, Merge, MergeAction,
    MergeWhen, NotMatchedBySourceMergeBuilder, NotMatchedMergeBuilder, NullsPosition,
    OrderDirection, OrderItem, RawStmt, RowLock, Select, SelectItem, SetOperator, SetQuery, Stmt,
    Update,
};
pub use constructors::{
    delete_from, except, except_all, insert, intersect, intersect_all, merge_into, raw, select,
    union, union_all, update,
};
pub use items::IntoSelectItems;

use helpers::*;

#[cfg(test)]
mod tests;
