use super::*;

impl Renderer {
    pub(super) fn render_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Select(select) => self.render_select(select),
            Stmt::Set(set) => self.render_set(set),
            Stmt::Insert(insert) => self.render_insert(insert),
            Stmt::Update(update) => self.render_update(update),
            Stmt::Delete(delete) => self.render_delete(delete),
            Stmt::Raw(raw) => self.render_raw_stmt(raw),
        }
    }

    pub(super) fn render_select(&mut self, select: &Select) -> Result<()> {
        self.render_ctes(&select.ctes)?;

        self.sql.push_str("SELECT ");
        self.render_distinct(select)?;
        self.render_projection(select)?;
        self.sql.push_str(" FROM ");
        self.render_source(&select.source)?;
        for join in &select.joins {
            self.render_join(join)?;
        }
        if let Some(filter) = &select.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_group_by(&select.group_by)?;
        if let Some(having) = &select.having {
            self.sql.push_str(" HAVING ");
            self.render_bool(having)?;
        }
        self.render_order(&select.order)?;
        if let Some(limit) = &select.limit {
            self.sql.push_str(" LIMIT ");
            self.push_param(limit.clone());
        }
        if let Some(offset) = &select.offset {
            self.sql.push_str(" OFFSET ");
            self.push_param(offset.clone());
        }
        self.render_lock(select.lock);
        Ok(())
    }
    pub(super) fn render_set(&mut self, set: &SetQuery) -> Result<()> {
        self.sql.push('(');
        self.render_stmt(&set.left)?;
        self.sql.push_str(") ");
        self.sql.push_str(set.operator.as_sql());
        self.sql.push_str(" (");
        self.render_stmt(&set.right)?;
        self.sql.push(')');
        self.render_order(&set.order)?;
        if let Some(limit) = &set.limit {
            self.sql.push_str(" LIMIT ");
            self.push_param(limit.clone());
        }
        if let Some(offset) = &set.offset {
            self.sql.push_str(" OFFSET ");
            self.push_param(offset.clone());
        }
        Ok(())
    }
    pub(super) fn render_projection(&mut self, select: &Select) -> Result<()> {
        if select.projection.is_empty() {
            self.render_source_fields(&select.source);
            return Ok(());
        }
        for (idx, item) in select.projection.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_select_item(item)?;
        }
        Ok(())
    }

    pub(super) fn render_distinct(&mut self, select: &Select) -> Result<()> {
        if !select.distinct_on.is_empty() {
            self.sql.push_str("DISTINCT ON (");
            for (idx, expr) in select.distinct_on.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.render_value(expr)?;
            }
            self.sql.push_str(") ");
        } else if select.distinct {
            self.sql.push_str("DISTINCT ");
        }
        Ok(())
    }
    pub(super) fn render_select_item(&mut self, item: &SelectItem) -> Result<()> {
        self.render_value(&item.expr)?;
        if let Some(alias) = &item.alias {
            self.sql.push_str(" AS ");
            write_quoted_ident(&mut self.sql, alias);
        }
        Ok(())
    }
    pub(super) fn render_order(&mut self, order: &[crate::typed::OrderItem]) -> Result<()> {
        if order.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" ORDER BY ");
        for (idx, item) in order.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(&item.expr)?;
            self.sql.push(' ');
            self.sql.push_str(item.direction.as_sql());
        }
        Ok(())
    }

    pub(super) fn render_group_by(&mut self, group_by: &[ValueExpr]) -> Result<()> {
        if group_by.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" GROUP BY ");
        for (idx, expr) in group_by.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(expr)?;
        }
        Ok(())
    }

    pub(super) fn render_lock(&mut self, lock: Option<crate::typed::RowLock>) {
        let Some(lock) = lock else {
            return;
        };
        self.sql.push(' ');
        self.sql.push_str(lock.mode.as_sql());
        if let Some(wait) = lock.wait.as_sql() {
            self.sql.push(' ');
            self.sql.push_str(wait);
        }
    }
}
