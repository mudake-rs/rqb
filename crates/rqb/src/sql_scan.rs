pub(crate) fn starts_escape_string(sql: &str, index: usize) -> bool {
    let bytes = sql.as_bytes();
    matches!(bytes.get(index), Some(b'e' | b'E'))
        && bytes.get(index + 1) == Some(&b'\'')
        && starts_token_boundary(bytes, index)
}

pub(crate) fn starts_unicode_escape_string(sql: &str, index: usize) -> bool {
    let bytes = sql.as_bytes();
    matches!(bytes.get(index), Some(b'u' | b'U'))
        && bytes.get(index + 1) == Some(&b'&')
        && bytes.get(index + 2) == Some(&b'\'')
        && starts_token_boundary(bytes, index)
}

pub(crate) fn has_escape_string_prefix(sql: &str, quote_start: usize) -> bool {
    quote_start >= 1 && starts_escape_string(sql, quote_start - 1)
}

pub(crate) fn skip_single_quoted(sql: &str, quote_start: usize, escape_backslash: bool) -> usize {
    let mut pos = quote_start + 1;
    while pos < sql.len() {
        let Some(ch) = sql[pos..].chars().next() else {
            return pos;
        };
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

pub(crate) fn skip_double_quoted(sql: &str, quote_start: usize) -> usize {
    let mut pos = quote_start + 1;
    while pos < sql.len() {
        let Some(ch) = sql[pos..].chars().next() else {
            return pos;
        };
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

pub(crate) fn skip_line_comment(sql: &str, comment_start: usize) -> usize {
    sql[comment_start + 2..]
        .find('\n')
        .map_or(sql.len(), |offset| comment_start + 2 + offset)
}

pub(crate) fn skip_block_comment(sql: &str, comment_start: usize) -> usize {
    let mut pos = comment_start + 2;
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
        let Some(ch) = sql[pos..].chars().next() else {
            return pos;
        };
        pos += ch.len_utf8();
    }
    pos
}

pub(crate) fn skip_dollar_quoted(sql: &str, start: usize) -> Option<usize> {
    let open_end = dollar_quote_open_end(sql, start)?;
    let delimiter = &sql[start..open_end];
    Some(
        sql[open_end..]
            .find(delimiter)
            .map_or(sql.len(), |idx| open_end + idx + delimiter.len()),
    )
}

fn starts_token_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0 || !is_dollar_tag_char(char::from(bytes[index - 1]))
}

fn dollar_quote_open_end(sql: &str, start: usize) -> Option<usize> {
    let mut pos = start + 1;
    while pos < sql.len() {
        let ch = sql[pos..].chars().next()?;
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
    let mut chars = tag.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_dollar_tag_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_string_prefix_uses_one_boundary_rule() {
        let sql = "xE'not_escape' E'escape'";
        let first = sql.find("E'not_escape'").unwrap();
        let second = sql.find("E'escape'").unwrap();

        assert!(!starts_escape_string(sql, first));
        assert!(!has_escape_string_prefix(sql, first + 1));
        assert!(starts_escape_string(sql, second));
        assert!(has_escape_string_prefix(sql, second + 1));
    }

    #[test]
    fn skip_primitives_tolerate_unterminated_input() {
        assert_eq!(skip_single_quoted("'unterminated", 0, false), 13);
        assert_eq!(skip_single_quoted("E'can\\", 1, true), 6);
        assert_eq!(skip_double_quoted("\"unterminated", 0), 13);
        assert_eq!(skip_line_comment("-- comment", 0), 10);
        assert_eq!(skip_block_comment("/* outer /* inner */", 0), 20);
        assert_eq!(skip_dollar_quoted("$tag$body", 0), Some(9));
    }

    #[test]
    fn dollar_quote_tags_follow_postgres_shape() {
        let quoted = "$tag$body$tag$";

        assert_eq!(skip_dollar_quoted(quoted, 0), Some(quoted.len()));
        assert_eq!(skip_dollar_quoted("$1$not a dollar quote$1$", 0), None);
    }
}
