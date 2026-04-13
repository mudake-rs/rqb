mod collection;
mod comparison;
mod target;
mod text;

use rqb_core::ValidatedPredicate;

use crate::Result;

use super::Renderer;

impl Renderer {
    pub(super) fn render_predicate(&mut self, predicate: &ValidatedPredicate) -> Result<()> {
        match predicate {
            ValidatedPredicate::NullCheck { field, negated } => {
                self.render_null_check(field, *negated)
            }
            ValidatedPredicate::Binary { field, op, value } => {
                self.render_binary_predicate(field, *op, value)
            }
            ValidatedPredicate::NullSafeBinary { field, op, value } => {
                self.render_null_safe_binary_predicate(field, *op, value)
            }
            ValidatedPredicate::In {
                field,
                values,
                negated,
            } => self.render_inclusion_predicate(field, values, *negated),
            ValidatedPredicate::Between {
                field,
                lower,
                upper,
                negated,
            } => self.render_between_predicate(field, lower, upper, *negated),
            ValidatedPredicate::Like {
                field,
                pattern,
                value,
                negated,
            } => self.render_like_predicate(field, *pattern, value, *negated),
            ValidatedPredicate::Regex {
                field,
                value,
                negated,
            } => self.render_regex_predicate(field, value, *negated),
            ValidatedPredicate::TextSearch { field, value } => {
                self.render_text_search(field, value)
            }
            ValidatedPredicate::ArraySet { field, op, value } => {
                self.render_array_set_predicate(field, *op, value)
            }
            ValidatedPredicate::ArrayMembership {
                field,
                value,
                negated,
            } => self.render_array_membership_predicate(field, value, *negated),
            ValidatedPredicate::ArrayState { field, empty } => {
                self.render_array_state_predicate(field, *empty)
            }
            ValidatedPredicate::ArrayElemMatch { field, value } => {
                self.render_array_elem_match(field, value)
            }
            ValidatedPredicate::JsonKey { field, key } => self.render_json_key(field, key),
            ValidatedPredicate::JsonKeySet { field, keys, all } => {
                self.render_json_key_set(field, keys, *all)
            }
            ValidatedPredicate::Containment {
                field,
                op,
                target,
                value,
                negated,
            } => self.render_containment_predicate(field, *op, *target, value, *negated),
        }
        Ok(())
    }
}
