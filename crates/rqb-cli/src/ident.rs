use std::collections::BTreeMap;

pub(crate) fn sanitize_ident(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if out.is_empty() {
        out.push('_');
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if is_rust_keyword(&out) {
        out.push('_');
    }
    out
}

pub(crate) fn unique_ident_strings<I, S>(values: I, reserved: &[&str]) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut seen = BTreeMap::<String, usize>::new();
    for reserved in reserved {
        seen.insert((*reserved).to_owned(), 1);
    }

    values
        .into_iter()
        .map(|value| unique_ident_string(value.into(), &mut seen))
        .collect()
}

fn unique_ident_string(name: String, seen: &mut BTreeMap<String, usize>) -> String {
    let count = seen.get(&name).copied().unwrap_or(0);
    if count == 0 {
        seen.insert(name.clone(), 1);
        return name;
    }

    let mut suffix = count;
    loop {
        let candidate = format!("{name}_{suffix}");
        if !seen.contains_key(&candidate) {
            seen.insert(name, suffix + 1);
            seen.insert(candidate.clone(), 1);
            return candidate;
        }
        suffix += 1;
    }
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "abstract"
            | "alignof"
            | "as"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "gen"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "offsetof"
            | "override"
            | "priv"
            | "proc"
            | "pure"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "sizeof"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "yield"
    )
}

#[cfg(test)]
mod tests {
    use super::{sanitize_ident, unique_ident_strings};

    #[test]
    fn sanitizes_identifiers() {
        assert_eq!(sanitize_ident("type"), "type_");
        assert_eq!(sanitize_ident("macro"), "macro_");
        assert_eq!(sanitize_ident("try"), "try_");
        assert_eq!(sanitize_ident("gen"), "gen_");
        assert_eq!(sanitize_ident("123bad-name"), "_123bad_name");
        assert_eq!(sanitize_ident(""), "_");
    }

    #[test]
    fn disambiguates_identifiers_with_reserved_names() {
        let names = unique_ident_strings(
            ["types".to_owned(), "orders".to_owned(), "orders".to_owned()],
            &["types"],
        );

        assert_eq!(names, vec!["types_1", "orders", "orders_1"]);
    }
}
