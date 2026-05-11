pub(crate) fn write_quoted_ident(output: &mut String, ident: &str) {
    if !ident.contains('"') {
        output.reserve(ident.len() + 2);
        output.push('"');
        output.push_str(ident);
        output.push('"');
        return;
    }

    output.push('"');
    for ch in ident.chars() {
        if ch == '"' {
            output.push('"');
        }
        output.push(ch);
    }
    output.push('"');
}

pub(crate) fn write_quoted_qualified(output: &mut String, name: &str) {
    for (idx, part) in name.split('.').enumerate() {
        if idx > 0 {
            output.push('.');
        }
        write_quoted_ident(output, part);
    }
}

#[cfg(test)]
mod tests {
    use super::{write_quoted_ident, write_quoted_qualified};

    #[test]
    fn quoted_ident_doubles_embedded_quotes() {
        let mut sql = String::new();

        write_quoted_ident(&mut sql, "strange\"name");

        assert_eq!(sql, "\"strange\"\"name\"");
    }

    #[test]
    fn qualified_ident_quotes_each_path_component() {
        let mut sql = String::new();

        write_quoted_qualified(&mut sql, "public.order");

        assert_eq!(sql, "\"public\".\"order\"");
    }

    #[test]
    fn qualified_ident_escapes_quotes_per_path_component() {
        let mut sql = String::new();

        write_quoted_qualified(&mut sql, "weird.schema\"name.table");

        assert_eq!(sql, "\"weird\".\"schema\"\"name\".\"table\"");
    }

    #[test]
    fn empty_ident_is_still_quoted_not_dropped() {
        let mut sql = String::new();

        write_quoted_ident(&mut sql, "");

        assert_eq!(sql, "\"\"");
    }
}
