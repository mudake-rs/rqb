use crate::{built::BuiltQuery, sql_scan};

#[must_use]
pub(crate) fn format_query_summary(query: &BuiltQuery) -> String {
    let mut output = String::new();
    output.push_str("SQL:\n");
    output.push_str(&format_query_sql(&query.sql));
    output.push_str("\n\n");

    let params = query.params.as_slice();
    if params.is_empty() {
        output.push_str("Params: none\n");
    } else {
        output.push_str(&format!("Params ({}):\n", params.len()));
        for (index, param) in params.iter().enumerate() {
            output.push_str(&format!(
                "${}: {}\n",
                index + 1,
                display_param_type(param.debug_name())
            ));
        }
    }

    output.push('\n');
    output.push_str(&format!("Cacheable: {}", query.cacheable));
    output
}

fn display_param_type(name: &'static str) -> &'static str {
    match name {
        name if name == std::any::type_name::<String>() => "String",
        name if name == std::any::type_name::<Vec<u8>>() => "Vec<u8>",
        name if name == std::any::type_name::<Vec<String>>() => "Vec<String>",
        name if name == std::any::type_name::<bool>() => "bool",
        name if name == std::any::type_name::<i16>() => "i16",
        name if name == std::any::type_name::<i32>() => "i32",
        name if name == std::any::type_name::<i64>() => "i64",
        name if name == std::any::type_name::<f32>() => "f32",
        name if name == std::any::type_name::<f64>() => "f64",
        name if name == std::any::type_name::<uuid::Uuid>() => "Uuid",
        name if name == std::any::type_name::<std::time::Duration>() => "Duration",
        name if name == std::any::type_name::<sqlx::postgres::types::PgInterval>() => "PgInterval",
        name if name == std::any::type_name::<sqlx::types::BigDecimal>() => "BigDecimal",
        name if name == std::any::type_name::<chrono::Duration>() => "chrono::Duration",
        name if name == std::any::type_name::<chrono::DateTime<chrono::Utc>>() => "DateTime<Utc>",
        name if name == std::any::type_name::<chrono::DateTime<chrono::FixedOffset>>() => {
            "DateTime<FixedOffset>"
        }
        name if name == std::any::type_name::<chrono::NaiveDate>() => "NaiveDate",
        name if name == std::any::type_name::<chrono::NaiveDateTime>() => "NaiveDateTime",
        name if name == std::any::type_name::<chrono::NaiveTime>() => "NaiveTime",
        name if name == std::any::type_name::<serde_json::Value>() => "serde_json::Value",
        _ => name,
    }
}

pub(crate) fn format_query_sql(sql: &str) -> String {
    let tokens = tokenize_sql(sql);
    if tokens.is_empty() {
        return String::new();
    }

    SqlFormatter::new(tokens).format()
}

// Keep SQL literal/comment boundary handling aligned with `raw`; this formatter
// is display-only, while raw owns bind-count validation for execution paths.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SqlTokenKind {
    Word,
    Quoted,
    Literal,
    Comment,
    Placeholder,
    Number,
    Symbol,
    Operator,
}

#[derive(Clone, Copy)]
struct SqlToken<'a> {
    text: &'a str,
    kind: SqlTokenKind,
}

impl SqlToken<'_> {
    fn is_word(self, word: &str) -> bool {
        self.kind == SqlTokenKind::Word && self.text.eq_ignore_ascii_case(word)
    }

    fn is_symbol(self, symbol: &str) -> bool {
        self.kind == SqlTokenKind::Symbol && self.text == symbol
    }
}

fn tokenize_sql(sql: &str) -> Vec<SqlToken<'_>> {
    let bytes = sql.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let start = index;
        let byte = bytes[index];

        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if sql_scan::starts_escape_string(sql, index) {
            index = sql_scan::skip_single_quoted(sql, index + 1, true);
            tokens.push(SqlToken {
                text: &sql[start..index],
                kind: SqlTokenKind::Literal,
            });
            continue;
        }

        if sql_scan::starts_unicode_escape_string(sql, index) {
            index = sql_scan::skip_single_quoted(sql, index + 2, false);
            tokens.push(SqlToken {
                text: &sql[start..index],
                kind: SqlTokenKind::Literal,
            });
            continue;
        }

        match byte {
            b'"' => {
                index = sql_scan::skip_double_quoted(sql, index);
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Quoted,
                });
            }
            b'\'' => {
                index = sql_scan::skip_single_quoted(sql, index, false);
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Literal,
                });
            }
            b'$' => {
                if let Some(end) = sql_scan::skip_dollar_quoted(sql, index) {
                    index = end;
                    tokens.push(SqlToken {
                        text: &sql[start..index],
                        kind: SqlTokenKind::Literal,
                    });
                } else {
                    index += 1;
                    while index < bytes.len() && bytes[index].is_ascii_digit() {
                        index += 1;
                    }
                    tokens.push(SqlToken {
                        text: &sql[start..index],
                        kind: SqlTokenKind::Placeholder,
                    });
                }
            }
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index = sql_scan::skip_line_comment(sql, index);
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Comment,
                });
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = sql_scan::skip_block_comment(sql, index);
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Comment,
                });
            }
            b if is_ident_start(b) => {
                index += 1;
                while index < bytes.len() && is_ident_continue(bytes[index]) {
                    index += 1;
                }
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Word,
                });
            }
            b if b.is_ascii_digit() => {
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'_' | b'.'))
                {
                    index += 1;
                }
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Number,
                });
            }
            b':' if bytes.get(index + 1) == Some(&b':') => {
                index += 2;
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Symbol,
                });
            }
            b',' | b'(' | b')' | b'.' | b';' | b'[' | b']' => {
                index += 1;
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Symbol,
                });
            }
            _ => {
                index = scan_operator_or_char(sql, index);
                tokens.push(SqlToken {
                    text: &sql[start..index],
                    kind: SqlTokenKind::Operator,
                });
            }
        }
    }

    tokens
}

fn scan_operator_or_char(sql: &str, mut index: usize) -> usize {
    let bytes = sql.as_bytes();
    if !is_operator_byte(bytes[index]) {
        return index + sql[index..].chars().next().map_or(1, char::len_utf8);
    }

    index += 1;
    while index < bytes.len() && is_operator_byte(bytes[index]) {
        if matches!(
            (bytes.get(index), bytes.get(index + 1)),
            (Some(b'-'), Some(b'-')) | (Some(b'/'), Some(b'*'))
        ) {
            break;
        }
        index += 1;
    }
    index
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_operator_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'+' | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'='
            | b'<'
            | b'>'
            | b'!'
            | b'~'
            | b'@'
            | b'#'
            | b'?'
            | b'&'
            | b'|'
            | b'^'
    )
}

struct SqlFormatter<'a> {
    tokens: Vec<SqlToken<'a>>,
    out: String,
    depth: usize,
    subquery_depths: Vec<usize>,
    list_split_depth: Option<usize>,
    list_stack: Vec<Option<usize>>,
    previous: Option<WrittenToken>,
    line_has_token: bool,
}

struct WrittenToken {
    text: String,
    kind: SqlTokenKind,
}

impl<'a> SqlFormatter<'a> {
    fn new(tokens: Vec<SqlToken<'a>>) -> Self {
        Self {
            tokens,
            out: String::new(),
            depth: 0,
            subquery_depths: Vec::new(),
            list_split_depth: None,
            list_stack: Vec::new(),
            previous: None,
            line_has_token: false,
        }
    }

    fn format(mut self) -> String {
        let mut index = 0;
        while index < self.tokens.len() {
            let token = self.tokens[index];

            if token.is_symbol("(") {
                let starts_subquery = self.starts_subquery(index + 1);
                self.write_token(token.text, token.kind);
                self.depth += 1;
                if starts_subquery {
                    self.subquery_depths.push(self.depth);
                    self.list_stack.push(self.list_split_depth);
                    self.list_split_depth = None;
                    self.newline(self.base_indent());
                }
                index += 1;
                continue;
            }

            if token.is_symbol("[") {
                self.write_token(token.text, token.kind);
                self.depth += 1;
                index += 1;
                continue;
            }

            if token.is_symbol("]") {
                self.depth = self.depth.saturating_sub(1);
                self.write_token(token.text, token.kind);
                index += 1;
                continue;
            }

            if token.is_symbol(")") {
                if self.subquery_depths.last() == Some(&self.depth) {
                    self.newline(self.base_indent().saturating_sub(4));
                    self.subquery_depths.pop();
                    self.list_split_depth = self.list_stack.pop().unwrap_or(None);
                }
                self.depth = self.depth.saturating_sub(1);
                self.write_token(token.text, token.kind);
                index += 1;
                continue;
            }

            if self.depth == self.current_format_depth()
                && let Some(clause) = self.match_clause(index)
            {
                self.write_clause(clause);
                index += clause.len;
                continue;
            }

            if self.should_split_comma(token) {
                self.write_token(token.text, token.kind);
                self.newline(self.base_indent() + 4);
                index += 1;
                continue;
            }

            if token.kind == SqlTokenKind::Comment && token.text.starts_with("--") {
                self.write_token(token.text, token.kind);
                self.newline(self.base_indent());
                index += 1;
                continue;
            }

            self.write_token(token.text, token.kind);
            index += 1;
        }

        self.out.trim_end().to_owned()
    }

    fn starts_subquery(&self, index: usize) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| token.is_word("SELECT") || token.is_word("WITH"))
    }

    fn current_format_depth(&self) -> usize {
        self.subquery_depths.last().copied().unwrap_or(0)
    }

    fn base_indent(&self) -> usize {
        self.current_format_depth() * 4
    }

    fn should_split_comma(&self, token: SqlToken<'_>) -> bool {
        token.is_symbol(",") && self.list_split_depth == Some(self.depth)
    }

    fn match_clause(&self, index: usize) -> Option<Clause> {
        if self.is_distinct_on(index) {
            return None;
        }

        const CLAUSES: &[(&[&str], &str, ClauseKind)] = &[
            (
                &["FOR", "NO", "KEY", "UPDATE"],
                "FOR NO KEY UPDATE",
                ClauseKind::Line,
            ),
            (&["FOR", "KEY", "SHARE"], "FOR KEY SHARE", ClauseKind::Line),
            (
                &["DO", "UPDATE", "SET"],
                "DO UPDATE SET",
                ClauseKind::BreakList,
            ),
            (
                &["THEN", "UPDATE", "SET"],
                "THEN UPDATE SET",
                ClauseKind::BreakList,
            ),
            (
                &["THEN", "DO", "NOTHING"],
                "THEN DO NOTHING",
                ClauseKind::Line,
            ),
            (&["THEN", "INSERT"], "THEN INSERT", ClauseKind::Line),
            (&["THEN", "DELETE"], "THEN DELETE", ClauseKind::Line),
            (
                &["WHEN", "NOT", "MATCHED"],
                "WHEN NOT MATCHED",
                ClauseKind::Line,
            ),
            (
                &["LEFT", "OUTER", "JOIN"],
                "LEFT OUTER JOIN",
                ClauseKind::Line,
            ),
            (
                &["RIGHT", "OUTER", "JOIN"],
                "RIGHT OUTER JOIN",
                ClauseKind::Line,
            ),
            (
                &["FULL", "OUTER", "JOIN"],
                "FULL OUTER JOIN",
                ClauseKind::Line,
            ),
            (&["WITH", "RECURSIVE"], "WITH RECURSIVE", ClauseKind::Line),
            (&["INSERT", "INTO"], "INSERT INTO", ClauseKind::Line),
            (&["DELETE", "FROM"], "DELETE FROM", ClauseKind::Line),
            (&["MERGE", "INTO"], "MERGE INTO", ClauseKind::Line),
            (&["GROUP", "BY"], "GROUP BY", ClauseKind::InlineList),
            (&["ORDER", "BY"], "ORDER BY", ClauseKind::InlineList),
            (&["ON", "CONFLICT"], "ON CONFLICT", ClauseKind::Line),
            (&["DO", "NOTHING"], "DO NOTHING", ClauseKind::Line),
            (&["UNION", "ALL"], "UNION ALL", ClauseKind::Line),
            (&["DEFAULT", "VALUES"], "DEFAULT VALUES", ClauseKind::Line),
            (&["FETCH", "FIRST"], "FETCH FIRST", ClauseKind::Line),
            (&["FOR", "UPDATE"], "FOR UPDATE", ClauseKind::Line),
            (&["FOR", "SHARE"], "FOR SHARE", ClauseKind::Line),
            (&["WHEN", "MATCHED"], "WHEN MATCHED", ClauseKind::Line),
            (&["SELECT"], "SELECT", ClauseKind::BreakList),
            (&["RETURNING"], "RETURNING", ClauseKind::BreakList),
            (&["UPDATE"], "UPDATE", ClauseKind::Line),
            (&["FROM"], "FROM", ClauseKind::Line),
            (&["USING"], "USING", ClauseKind::Line),
            (&["WHERE"], "WHERE", ClauseKind::Line),
            (&["HAVING"], "HAVING", ClauseKind::Line),
            (&["VALUES"], "VALUES", ClauseKind::Line),
            (&["SET"], "SET", ClauseKind::BreakList),
            (&["JOIN"], "JOIN", ClauseKind::Line),
            (&["LEFT", "JOIN"], "LEFT JOIN", ClauseKind::Line),
            (&["RIGHT", "JOIN"], "RIGHT JOIN", ClauseKind::Line),
            (&["FULL", "JOIN"], "FULL JOIN", ClauseKind::Line),
            (&["CROSS", "JOIN"], "CROSS JOIN", ClauseKind::Line),
            (&["INNER", "JOIN"], "INNER JOIN", ClauseKind::Line),
            (&["ON"], "ON", ClauseKind::Line),
            (&["UNION"], "UNION", ClauseKind::Line),
            (&["INTERSECT"], "INTERSECT", ClauseKind::Line),
            (&["EXCEPT"], "EXCEPT", ClauseKind::Line),
            (&["LIMIT"], "LIMIT", ClauseKind::Line),
            (&["OFFSET"], "OFFSET", ClauseKind::Line),
            (&["WITH"], "WITH", ClauseKind::Line),
        ];

        CLAUSES.iter().find_map(|(words, text, kind)| {
            self.matches_words(index, words).then_some(Clause {
                text,
                len: words.len(),
                kind: *kind,
            })
        })
    }

    fn is_distinct_on(&self, index: usize) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| token.is_word("ON"))
            && self
                .tokens
                .get(index.saturating_sub(1))
                .is_some_and(|token| token.is_word("DISTINCT"))
    }

    fn matches_words(&self, index: usize, words: &[&str]) -> bool {
        words.iter().enumerate().all(|(offset, word)| {
            self.tokens
                .get(index + offset)
                .is_some_and(|token| token.is_word(word))
        })
    }

    fn write_clause(&mut self, clause: Clause) {
        let indent = self.base_indent();
        if !self.out.is_empty() {
            self.newline(indent);
        }

        self.write_token(clause.text, SqlTokenKind::Word);
        match clause.kind {
            ClauseKind::BreakList => {
                self.list_split_depth = Some(self.depth);
                self.newline(indent + 4);
            }
            ClauseKind::InlineList => {
                self.list_split_depth = Some(self.depth);
            }
            ClauseKind::Line => {
                self.list_split_depth = None;
            }
        }
    }

    fn newline(&mut self, indent: usize) {
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        if self.out.is_empty() || self.out.ends_with('\n') {
            self.out.extend(std::iter::repeat_n(' ', indent));
        }
        self.previous = None;
        self.line_has_token = false;
    }

    fn write_token(&mut self, text: &str, kind: SqlTokenKind) {
        if self.line_has_token && self.needs_space(text, kind) {
            self.out.push(' ');
        }
        self.out.push_str(text);
        self.previous = Some(WrittenToken {
            text: text.to_owned(),
            kind,
        });
        self.line_has_token = true;
    }

    fn needs_space(&self, text: &str, kind: SqlTokenKind) -> bool {
        let Some(previous) = &self.previous else {
            return false;
        };

        if matches!(text, "," | ";" | "." | ")" | "]" | "::") {
            return false;
        }
        if matches!(previous.text.as_str(), "(" | "[" | "." | "::") {
            return false;
        }
        if text == "[" {
            return false;
        }
        if text == "(" {
            return needs_space_before_open_paren(previous);
        }
        if kind == SqlTokenKind::Operator || previous.kind == SqlTokenKind::Operator {
            return true;
        }

        true
    }
}

#[derive(Clone, Copy)]
struct Clause {
    text: &'static str,
    len: usize,
    kind: ClauseKind,
}

#[derive(Clone, Copy)]
enum ClauseKind {
    BreakList,
    InlineList,
    Line,
}

fn needs_space_before_open_paren(previous: &WrittenToken) -> bool {
    if previous.kind != SqlTokenKind::Word {
        return true;
    }

    matches!(
        previous.text.to_ascii_uppercase().as_str(),
        "IN" | "VALUES"
            | "ON"
            | "AS"
            | "OVER"
            | "FILTER"
            | "PARTITION"
            | "BY"
            | "FROM"
            | "USING"
            | "INTO"
            | "CONFLICT"
            | "SET"
            | "WHERE"
            | "THEN"
            | "UPDATE"
            | "INSERT"
            | "DELETE"
            | "SELECT"
            | "DISTINCT"
            | "CASE"
            | "WHEN"
    )
}

#[cfg(test)]
mod tests {
    use crate::{BuiltQuery, Param, Params};

    #[test]
    fn query_summary_formats_sql_params_and_cacheability() {
        let built = BuiltQuery {
            sql: concat!(
                "SELECT \"id\", \"email\" FROM \"public\".\"users\" ",
                "WHERE \"email\" = $1 ORDER BY \"email\" ASC LIMIT $2"
            )
            .to_owned(),
            params: Params::from_vec(vec![Param::typed(1_i32), Param::typed("x".to_owned())]),
            cacheable: false,
        };

        let rendered = built.summary();

        assert_eq!(
            rendered,
            concat!(
                "SQL:\n",
                "SELECT\n",
                "    \"id\",\n",
                "    \"email\"\n",
                "FROM \"public\".\"users\"\n",
                "WHERE \"email\" = $1\n",
                "ORDER BY \"email\" ASC\n",
                "LIMIT $2\n",
                "\n",
                "Params (2):\n",
                "$1: i32\n",
                "$2: String\n",
                "\n",
                "Cacheable: false"
            )
        );
        assert_eq!(
            built.pretty_sql(),
            concat!(
                "SELECT\n",
                "    \"id\",\n",
                "    \"email\"\n",
                "FROM \"public\".\"users\"\n",
                "WHERE \"email\" = $1\n",
                "ORDER BY \"email\" ASC\n",
                "LIMIT $2"
            )
        );
    }

    #[test]
    fn query_summary_formats_empty_params() {
        let built = BuiltQuery {
            sql: "select 1".to_owned(),
            params: Params::new(),
            cacheable: true,
        };

        assert_eq!(
            built.summary(),
            concat!(
                "SQL:\n",
                "SELECT\n",
                "    1\n",
                "\n",
                "Params: none\n",
                "\n",
                "Cacheable: true"
            )
        );
    }

    #[test]
    fn summary_type_display_shortens_common_array_types() {
        assert_eq!(
            super::display_param_type(std::any::type_name::<Vec<String>>()),
            "Vec<String>"
        );
        assert_eq!(
            super::display_param_type("my_crate::DomainId"),
            "my_crate::DomainId"
        );
    }

    #[test]
    fn pretty_sql_preserves_literals_quoted_identifiers_and_dollar_quotes() {
        assert_eq!(
            super::format_query_sql(concat!(
                "SELECT 'FROM x, WHERE y' AS label, ",
                "$$ORDER BY z$$ AS body, ",
                "\"weird FROM\" FROM \"public\".\"users\" WHERE \"email\" = $1"
            )),
            concat!(
                "SELECT\n",
                "    'FROM x, WHERE y' AS label,\n",
                "    $$ORDER BY z$$ AS body,\n",
                "    \"weird FROM\"\n",
                "FROM \"public\".\"users\"\n",
                "WHERE \"email\" = $1"
            )
        );
    }

    #[test]
    fn pretty_sql_uses_raw_boundaries_for_nested_block_comments() {
        assert_eq!(
            super::format_query_sql(concat!(
                "SELECT \"id\" FROM \"users\" ",
                "WHERE /* outer /* inner */ still comment */ \"email\" = $1"
            )),
            concat!(
                "SELECT\n",
                "    \"id\"\n",
                "FROM \"users\"\n",
                "WHERE /* outer /* inner */ still comment */ \"email\" = $1"
            )
        );
    }

    #[test]
    fn pretty_sql_uses_raw_boundaries_for_escape_strings() {
        assert_eq!(
            super::format_query_sql(
                "SELECT E'can\\'t WHERE $1' AS label FROM \"users\" WHERE \"id\" = $1",
            ),
            concat!(
                "SELECT\n",
                "    E'can\\'t WHERE $1' AS label\n",
                "FROM \"users\"\n",
                "WHERE \"id\" = $1"
            )
        );
    }

    #[test]
    fn pretty_sql_uses_raw_boundaries_for_unicode_escape_strings() {
        assert_eq!(
            super::format_query_sql(
                "SELECT U&'it''s \\0441 FROM x' AS label FROM \"users\" WHERE \"id\" = $1",
            ),
            concat!(
                "SELECT\n",
                "    U&'it''s \\0441 FROM x' AS label\n",
                "FROM \"users\"\n",
                "WHERE \"id\" = $1"
            )
        );
    }

    #[test]
    fn pretty_sql_formats_nested_selects() {
        assert_eq!(
            super::format_query_sql(concat!(
                "SELECT \"id\" FROM (",
                "SELECT \"id\", \"email\" FROM \"public\".\"users\" WHERE \"active\" = $1",
                ") AS \"u\" WHERE \"id\" > $2"
            )),
            concat!(
                "SELECT\n",
                "    \"id\"\n",
                "FROM (\n",
                "    SELECT\n",
                "        \"id\",\n",
                "        \"email\"\n",
                "    FROM \"public\".\"users\"\n",
                "    WHERE \"active\" = $1\n",
                ") AS \"u\"\n",
                "WHERE \"id\" > $2"
            )
        );
    }

    #[test]
    fn pretty_sql_restores_projection_list_after_nested_select() {
        assert_eq!(
            super::format_query_sql(concat!(
                "SELECT \"a\", ",
                "(SELECT \"id\" FROM \"users\" WHERE \"id\" = $1) AS \"s\", ",
                "\"b\", \"c\" FROM \"outer\""
            )),
            concat!(
                "SELECT\n",
                "    \"a\",\n",
                "    (\n",
                "    SELECT\n",
                "        \"id\"\n",
                "    FROM \"users\"\n",
                "    WHERE \"id\" = $1\n",
                ") AS \"s\",\n",
                "    \"b\",\n",
                "    \"c\"\n",
                "FROM \"outer\""
            )
        );
    }

    #[test]
    fn pretty_sql_keeps_expression_keywords_inside_select_list() {
        assert_eq!(
            super::format_query_sql(concat!(
                "SELECT DISTINCT ON (\"email\") \"email\", ",
                "CASE WHEN \"score\" > $1 THEN $2 ELSE $3 END AS \"bucket\" ",
                "FROM \"public\".\"users\""
            )),
            concat!(
                "SELECT\n",
                "    DISTINCT ON (\"email\") \"email\",\n",
                "    CASE WHEN \"score\" > $1 THEN $2 ELSE $3 END AS \"bucket\"\n",
                "FROM \"public\".\"users\""
            )
        );
    }

    #[test]
    fn pretty_sql_does_not_split_array_or_subscript_commas_as_projection_commas() {
        assert_eq!(
            super::format_query_sql(
                "SELECT \"tags\"[$1] AS \"tag\", ARRAY[$2, $3] AS \"values\" FROM \"orders\""
            ),
            concat!(
                "SELECT\n",
                "    \"tags\"[$1] AS \"tag\",\n",
                "    ARRAY[$2, $3] AS \"values\"\n",
                "FROM \"orders\""
            )
        );
    }
}
