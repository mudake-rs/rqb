use super::*;

impl OrderItem {
    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        self.expr.collect_params(params);
    }
}

impl Stmt {
    pub fn params(&self) -> Params {
        let mut params = Vec::new();
        self.collect_params(&mut params);
        Params::from_vec(params)
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Select(select) => select.collect_params(params),
            Self::Set(set) => set.collect_params(params),
            Self::Insert(insert) => insert.collect_params(params),
            Self::Update(update) => update.collect_params(params),
            Self::Delete(delete) => delete.collect_params(params),
            Self::Raw(raw_stmt) => params.extend(raw_stmt.params.iter().cloned()),
        }
    }
}

impl SetQuery {
    fn collect_params(&self, params: &mut Vec<Param>) {
        self.left.collect_params(params);
        self.right.collect_params(params);
        for item in &self.order {
            item.collect_params(params);
        }
        if let Some(limit) = &self.limit {
            params.push(limit.clone());
        }
        if let Some(offset) = &self.offset {
            params.push(offset.clone());
        }
    }
}

impl Select {
    fn collect_params(&self, params: &mut Vec<Param>) {
        for cte in &self.ctes {
            cte.collect_params(params);
        }
        for expr in &self.distinct_on {
            expr.collect_params(params);
        }
        for item in &self.projection {
            item.expr.collect_params(params);
        }
        self.source.collect_from_params(params);
        for join in &self.joins {
            join.collect_params(params);
        }
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        for expr in &self.group_by {
            expr.collect_params(params);
        }
        if let Some(having) = &self.having {
            having.collect_params(params);
        }
        for item in &self.order {
            item.collect_params(params);
        }
        if let Some(limit) = &self.limit {
            params.push(limit.clone());
        }
        if let Some(offset) = &self.offset {
            params.push(offset.clone());
        }
    }
}

impl Insert {
    fn collect_params(&self, params: &mut Vec<Param>) {
        if let Some(source) = &self.source {
            source.collect_params(params);
        } else {
            for assignment in &self.assignments {
                assignment.value.collect_params(params);
            }
        }
        if let Some(conflict) = &self.conflict {
            conflict.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl ConflictClause {
    fn collect_params(&self, params: &mut Vec<Param>) {
        if let ConflictTarget::Columns {
            predicate: Some(predicate),
            ..
        } = &self.target
        {
            predicate.collect_params(params);
        }
        if let ConflictAction::DoUpdate {
            assignments,
            filter,
        } = &self.action
        {
            for assignment in assignments {
                assignment.value.collect_params(params);
            }
            if let Some(filter) = filter {
                filter.collect_params(params);
            }
        }
    }
}

impl Update {
    fn collect_params(&self, params: &mut Vec<Param>) {
        for assignment in &self.assignments {
            assignment.value.collect_params(params);
        }
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl Delete {
    fn collect_params(&self, params: &mut Vec<Param>) {
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

fn collect_returning_params(returning: &[SelectItem], params: &mut Vec<Param>) {
    for item in returning {
        item.expr.collect_params(params);
    }
}
