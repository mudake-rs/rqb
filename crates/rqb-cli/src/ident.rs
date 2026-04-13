use std::collections::BTreeMap;

use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span};

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

pub(crate) fn unique_enum_variant_idents(variants: &[String]) -> Vec<Ident> {
    let mut seen = BTreeMap::<String, usize>::new();
    variants
        .iter()
        .map(|variant| {
            let mut name = sanitize_ident(&variant.to_upper_camel_case());
            let count = seen.entry(name.clone()).or_insert(0);
            if *count > 0 {
                name.push('_');
                name.push_str(&count.to_string());
            }
            *count += 1;
            Ident::new(&name, Span::call_site())
        })
        .collect()
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

#[cfg(test)]
mod tests {
    use super::{sanitize_ident, unique_enum_variant_idents};

    #[test]
    fn sanitizes_identifiers_and_disambiguates_enum_variants() {
        assert_eq!(sanitize_ident("type"), "type_");
        assert_eq!(sanitize_ident("123bad-name"), "_123bad_name");
        assert_eq!(sanitize_ident(""), "_");

        let variants = vec![
            "foo-bar".to_owned(),
            "foo_bar".to_owned(),
            "foo bar".to_owned(),
        ];
        let names = unique_enum_variant_idents(&variants)
            .into_iter()
            .map(|ident| ident.to_string())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["FooBar", "FooBar_1", "FooBar_2"]);
    }
}
