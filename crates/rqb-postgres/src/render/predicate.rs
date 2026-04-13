mod collection;
mod comparison;
mod target;
mod text;

use rqb_core::{Operator, OperatorCategory, ResolvedField, Value};

use crate::Result;

use super::Renderer;

impl Renderer {
    pub(super) fn render_predicate(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) -> Result<()> {
        match operator.category() {
            OperatorCategory::NullCheck => self.render_null_check(field, operator),
            OperatorCategory::Contains => self.render_contains_operator(field, operator, value),
            OperatorCategory::TextAffix => self.render_text_affix_operator(field, operator, value),
            OperatorCategory::Equality => self.render_equality_operator(field, operator, value),
            OperatorCategory::NullSafeEquality => {
                self.render_null_safe_equality_operator(field, operator, value)
            }
            OperatorCategory::Ordering => self.render_ordering_operator(field, operator, value),
            OperatorCategory::Inclusion => self.render_inclusion_operator(field, operator, value),
            OperatorCategory::Between => self.render_between_operator(field, operator, value),
            OperatorCategory::ArraySet => self.render_array_set_operator(field, operator, value),
            OperatorCategory::ArrayMembership => {
                self.render_array_membership_operator(field, operator, value)
            }
            OperatorCategory::ArrayState => self.render_array_state_operator(field, operator),
            OperatorCategory::ArrayElementMatch => self.render_array_elem_match(field, value),
            OperatorCategory::JsonKey => self.render_json_key(field, value),
            OperatorCategory::JsonKeySet => self.render_json_key_set(field, operator, value),
            OperatorCategory::Containment => {
                self.render_containment_operator(field, operator, value)
            }
            OperatorCategory::Regex => self.render_regex_operator(field, operator, value),
            OperatorCategory::TextSearch => self.render_text_search(field, value),
        }
        Ok(())
    }
}
