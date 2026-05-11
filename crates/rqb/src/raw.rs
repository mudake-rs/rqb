use crate::{Error, Result};

#[inline]
pub(crate) fn validate_bind_count(sql: &str, binds: usize) -> Result<()> {
    let placeholders = count_placeholders(sql);
    if placeholders == binds {
        return Ok(());
    }
    Err(Error::RawBindMismatch {
        placeholders,
        binds,
    })
}

#[inline]
pub(crate) fn count_placeholders(sql: &str) -> usize {
    let mut count = 0;
    let mut chars = sql.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '?' {
            continue;
        }
        if chars.peek() == Some(&'?') {
            chars.next();
        } else {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::count_placeholders;

    #[test]
    fn question_mark_placeholders_support_escaped_literals() {
        assert_eq!(count_placeholders("a = ? and b = ?"), 2);
        assert_eq!(count_placeholders("jsonb_col ?? 'key' and x = ?"), 1);
    }

    #[test]
    fn escaped_question_marks_do_not_change_following_placeholder_numbers() {
        assert_eq!(
            count_placeholders("jsonb_col ?? 'key' and value ??| array[?] and id = ?"),
            2
        );
    }
}
