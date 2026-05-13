use super::*;

impl Renderer {
    pub(super) fn render_raw_stmt(&mut self, raw: &RawStmt) -> Result<()> {
        self.render_raw(&raw.sql, &raw.params)
    }
    pub(super) fn render_raw(&mut self, sql: &str, params: &[Param]) -> Result<()> {
        // Validation owns this check; keep the debug assertion to catch internal
        // callers that bypass the validated build path.
        debug_assert_eq!(crate::raw::count_placeholders(sql), params.len());
        self.cacheable = false;
        if params.is_empty() && !sql.as_bytes().contains(&b'?') {
            self.sql.push_str(sql);
            return Ok(());
        }
        let mut bind_index = 0usize;
        crate::raw::scan_raw_tokens(sql, |token| match token {
            crate::raw::RawToken::Text(text) => self.sql.push_str(text),
            crate::raw::RawToken::EscapedQuestion => self.sql.push('?'),
            crate::raw::RawToken::Placeholder => {
                self.push_param(params[bind_index].clone());
                bind_index += 1;
            }
        });
        debug_assert_eq!(bind_index, params.len());
        Ok(())
    }

    pub(super) fn push_param(&mut self, param: Param) {
        self.params.push(param);
        self.sql.push('$');
        let mut buffer = itoa::Buffer::new();
        self.sql.push_str(buffer.format(self.params.len()));
    }
}
