pub(crate) fn write_quoted_ident(output: &mut String, ident: &str) {
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
