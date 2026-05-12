use crate::{Error, Result};

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

    while pos < sql.len() {
        if sql[pos..].starts_with('\'') {
            pos = skip_single_quoted(sql, pos + 1, has_escape_string_prefix(sql, pos));
            continue;
        }
        if sql[pos..].starts_with('"') {
            pos = skip_double_quoted(sql, pos + 1);
            continue;
        }
        if sql[pos..].starts_with("--") {
            pos = skip_line_comment(sql, pos + 2);
            continue;
        }
        if sql[pos..].starts_with("/*") {
            pos = skip_block_comment(sql, pos + 2);
            continue;
        }
        if sql.as_bytes()[pos] == b'$'
            && let Some(open_end) = dollar_quote_open_end(sql, pos)
        {
            pos = skip_dollar_quoted(sql, pos, open_end);
            continue;
        }
        if sql[pos..].starts_with('?') {
            if text_start < pos {
                f(RawToken::Text(&sql[text_start..pos]));
            }
            if sql[pos + 1..].starts_with('?') {
                f(RawToken::EscapedQuestion);
                pos += 2;
            } else {
                f(RawToken::Placeholder);
                pos += 1;
            }
            text_start = pos;
            continue;
        }

        let ch = sql[pos..].chars().next().expect("pos is in bounds");
        pos += ch.len_utf8();
    }

    if text_start < sql.len() {
        f(RawToken::Text(&sql[text_start..]));
    }
}

fn has_escape_string_prefix(sql: &str, quote_pos: usize) -> bool {
    if quote_pos == 0 {
        return false;
    }
    let bytes = sql.as_bytes();
    if !matches!(bytes[quote_pos - 1], b'e' | b'E') {
        return false;
    }
    quote_pos == 1 || !is_dollar_tag_char(char::from(bytes[quote_pos - 2]))
}

fn skip_single_quoted(sql: &str, mut pos: usize, escape_backslash: bool) -> usize {
    while pos < sql.len() {
        let ch = sql[pos..].chars().next().expect("pos is in bounds");
        pos += ch.len_utf8();
        match ch {
            '\'' if sql[pos..].starts_with('\'') => pos += 1,
            '\'' => return pos,
            '\\' if escape_backslash => {
                if let Some(next) = sql[pos..].chars().next() {
                    pos += next.len_utf8();
                }
            }
            _ => {}
        }
    }
    pos
}

fn skip_double_quoted(sql: &str, mut pos: usize) -> usize {
    while pos < sql.len() {
        let ch = sql[pos..].chars().next().expect("pos is in bounds");
        pos += ch.len_utf8();
        if ch == '"' {
            if sql[pos..].starts_with('"') {
                pos += 1;
            } else {
                return pos;
            }
        }
    }
    pos
}

fn skip_line_comment(sql: &str, mut pos: usize) -> usize {
    while pos < sql.len() {
        let ch = sql[pos..].chars().next().expect("pos is in bounds");
        pos += ch.len_utf8();
        if ch == '\n' {
            return pos;
        }
    }
    pos
}

fn skip_block_comment(sql: &str, mut pos: usize) -> usize {
    let mut depth = 1usize;
    while pos < sql.len() {
        if sql[pos..].starts_with("/*") {
            depth += 1;
            pos += 2;
            continue;
        }
        if sql[pos..].starts_with("*/") {
            depth -= 1;
            pos += 2;
            if depth == 0 {
                return pos;
            }
            continue;
        }
        let ch = sql[pos..].chars().next().expect("pos is in bounds");
        pos += ch.len_utf8();
    }
    pos
}

fn dollar_quote_open_end(sql: &str, start: usize) -> Option<usize> {
    let mut pos = start + 1;
    while pos < sql.len() {
        let ch = sql[pos..].chars().next().expect("pos is in bounds");
        if ch == '$' {
            let tag = &sql[start + 1..pos];
            return valid_dollar_tag(tag).then_some(pos + 1);
        }
        if !is_dollar_tag_char(ch) {
            return None;
        }
        pos += ch.len_utf8();
    }
    None
}

fn valid_dollar_tag(tag: &str) -> bool {
    if tag.is_empty() {
        return true;
    }
    let mut chars = tag.chars();
    let first = chars.next().expect("tag is not empty");
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_dollar_tag_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn skip_dollar_quoted(sql: &str, start: usize, open_end: usize) -> usize {
    let delimiter = &sql[start..open_end];
    sql[open_end..]
        .find(delimiter)
        .map_or(sql.len(), |idx| open_end + idx + delimiter.len())
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
