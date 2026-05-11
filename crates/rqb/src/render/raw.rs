use super::*;

impl Renderer {
    pub(super) fn render_raw_stmt(&mut self, raw: &RawStmt) -> Result<()> {
        self.render_raw(&raw.sql, &raw.params)
    }
    pub(super) fn render_raw(&mut self, sql: &str, params: &[Param]) -> Result<()> {
        debug_assert_eq!(crate::raw::count_placeholders(sql), params.len());
        self.cacheable = false;
        let mut bind_index = 0usize;
        let mut chars = sql.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '?' {
                self.sql.push(ch);
                continue;
            }
            if chars.peek() == Some(&'?') {
                chars.next();
                self.sql.push('?');
                continue;
            }
            self.push_param(params[bind_index].clone());
            bind_index += 1;
        }
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
