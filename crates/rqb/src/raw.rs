use crate::{Error, Result, sql_scan};

// SQL boundaries live in `sql_scan`, shared with the summary tokenizer. Raw
// remains the bind-count authority for execution paths.
pub(crate) enum RawToken<'a> {
    Text(&'a str),
    EscapedQuestion,
    Placeholder,
}

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
    if !sql.as_bytes().contains(&b'?') {
        return 0;
    }
    let mut count = 0;
    scan_raw_tokens(sql, |token| {
        if matches!(token, RawToken::Placeholder) {
            count += 1;
        }
    });
    count
}

pub(crate) fn scan_raw_tokens<'a>(sql: &'a str, mut f: impl FnMut(RawToken<'a>)) {
    let mut pos = 0usize;
    let mut text_start = 0usize;
    let bytes = sql.as_bytes();

    while pos < sql.len() {
        match bytes[pos] {
            b'\'' => {
                pos = sql_scan::skip_single_quoted(
                    sql,
                    pos,
                    sql_scan::has_escape_string_prefix(sql, pos),
                );
                continue;
            }
            b'"' => {
                pos = sql_scan::skip_double_quoted(sql, pos);
                continue;
            }
            b'-' if bytes.get(pos + 1) == Some(&b'-') => {
                pos = sql_scan::skip_line_comment(sql, pos);
                continue;
            }
            b'/' if bytes.get(pos + 1) == Some(&b'*') => {
                pos = sql_scan::skip_block_comment(sql, pos);
                continue;
            }
            b'$' => {
                if let Some(end) = sql_scan::skip_dollar_quoted(sql, pos) {
                    pos = end;
                    continue;
                }
                pos += 1;
            }
            b'?' => {
                if text_start < pos {
                    f(RawToken::Text(&sql[text_start..pos]));
                }
                if bytes.get(pos + 1) == Some(&b'?') {
                    f(RawToken::EscapedQuestion);
                    pos += 2;
                } else {
                    f(RawToken::Placeholder);
                    pos += 1;
                }
                text_start = pos;
                continue;
            }
            byte if byte.is_ascii() => pos += 1,
            _ => {
                let ch = sql[pos..].chars().next().expect("pos is in bounds");
                pos += ch.len_utf8();
            }
        }
    }

    if text_start < sql.len() {
        f(RawToken::Text(&sql[text_start..]));
    }
}

#[cfg(test)]
mod tests {
    use super::count_placeholders;

    #[test]
    fn question_mark_placeholders_support_escaped_literals() {
        assert_eq!(count_placeholders("SELECT id, email FROM users"), 0);
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

    #[test]
    fn placeholders_ignore_sql_quoted_contexts_and_comments() {
        assert_eq!(count_placeholders("x = '?' and y = ?"), 1);
        assert_eq!(count_placeholders("x = 'it''s ?' and y = ?"), 1);
        assert_eq!(count_placeholders("x = E'escaped \\' ?' and y = ?"), 1);
        assert_eq!(count_placeholders("x = 'backslash \\\\' and y = ?"), 1);
        assert_eq!(
            count_placeholders("SELECT \"weird?col\" FROM t WHERE id = ?"),
            1
        );
        assert_eq!(count_placeholders("SELECT $$?$$, $tag$?$tag$, ?"), 1);
        assert_eq!(count_placeholders("-- ?\nSELECT ?"), 1);
        assert_eq!(count_placeholders("/* ? /* ? */ ? */ SELECT ?"), 1);
    }
}
