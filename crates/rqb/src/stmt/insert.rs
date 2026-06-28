use super::*;

impl Insert {
    /// Creates an insert statement for a table or view source.
    pub(crate) fn into(target: impl Into<Source>) -> Self {
        Self {
            ctes: Vec::new(),
            target: target.into(),
            columns: Vec::new(),
            assignments: Vec::new(),
            source: None,
            default_values: false,
            conflict: None,
            returning: Vec::new(),
        }
    }

    /// Adds a CTE to the insert statement.
    #[inline]
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Adds one column assignment. If the same database column was assigned
    /// earlier, this assignment replaces the earlier value.
    ///
    /// This makes it safe to layer server-owned values around a DTO mapping:
    /// call `values(&dto)` for request-owned fields and use `set(...)` for
    /// generated IDs, tenant IDs, status defaults, or explicit overrides.
    #[inline]
    pub fn set(mut self, assignment: Assignment) -> Self {
        push_column(&mut self.columns, assignment.field);
        push_assignment(&mut self.assignments, assignment);
        self
    }

    /// Adds multiple column assignments. Later assignments for the same
    /// database column replace earlier values.
    pub fn set_many(mut self, assignments: impl IntoAssignments) -> Self {
        extend_insert_assignments(
            &mut self.columns,
            &mut self.assignments,
            assignments.into_assignments(),
        );
        self
    }

    /// Adds one assignment only when `condition` is true.
    #[inline]
    pub fn set_if(self, condition: bool, assignment: Assignment) -> Self {
        if condition {
            self.set(assignment)
        } else {
            self
        }
    }

    /// Adds one assignment built from an optional value.
    pub fn set_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> Assignment) -> Self {
        match value {
            Some(value) => self.set(f(value)),
            None => self,
        }
    }

    /// Adds assignments produced by an [`Insertable`] DTO.
    ///
    /// The assignments participate in the same replacement rules as `set`.
    /// Call `values(&dto)` first, then `set(...)` for IDs, tenant fields, or
    /// other values owned by the server.
    pub fn values(mut self, values: impl Insertable) -> Self {
        extend_insert_assignments(
            &mut self.columns,
            &mut self.assignments,
            values.insert_assignments(),
        );
        self
    }

    /// Adds many [`Insertable`] DTO rows through `INSERT ... SELECT` over an
    /// inline `VALUES` source.
    ///
    /// Every row must produce the same insert fields in the same order. This is
    /// usually true for DTOs without `#[rqb(skip_none)]`; if optional insert
    /// fields can be omitted per row, normalize the input first or build an
    /// explicit `values_source(...)`.
    ///
    /// `alias` names the generated `VALUES` source. Use the same alias with
    /// [`Field::set_from`] when an upsert needs to copy incoming values in
    /// `DO UPDATE SET`.
    pub fn values_many<I, R>(self, rows: I, alias: impl Into<String>) -> Result<Self>
    where
        I: IntoIterator<Item = R>,
        R: Insertable,
    {
        if !self.columns.is_empty()
            || !self.assignments.is_empty()
            || self.source.is_some()
            || self.default_values
        {
            return Err(Error::InvalidInsertShape {
                message: "batch insert cannot be combined with existing insert values or source",
            });
        }

        let mut columns = None::<Vec<Meta>>;
        let mut values: Vec<Vec<ValueExpr>> = Vec::new();
        for row in rows {
            let assignments = normalized_batch_assignments(row.insert_assignments())?;
            match &columns {
                Some(columns) if !same_batch_fields(columns, &assignments) => {
                    return Err(Error::InvalidInsertShape {
                        message: "batch insert rows must use the same fields in the same order",
                    });
                }
                Some(_) => {}
                None => {
                    columns = Some(
                        assignments
                            .iter()
                            .map(|assignment| assignment.field)
                            .collect(),
                    );
                }
            }
            values.push(
                assignments
                    .into_iter()
                    .map(|assignment| match assignment.value {
                        AssignmentValue::Expr(expr) => Ok(expr),
                        AssignmentValue::Default => Err(Error::InvalidInsertShape {
                            message: "batch insert rows cannot use DEFAULT assignments",
                        }),
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
        }

        let Some(columns) = columns else {
            return Err(Error::InvalidInsertShape {
                message: "batch insert requires at least one row",
            });
        };
        Ok(self.from_select_all(crate::values_source(values, alias, columns)))
    }

    /// Adds a target column for `INSERT ... SELECT`.
    pub fn column<T>(mut self, field: Field<T>) -> Self {
        push_column(&mut self.columns, *field.meta);
        self
    }

    /// Adds multiple target columns for `INSERT ... SELECT`.
    pub fn columns(mut self, fields: impl IntoFieldMetas) -> Self {
        for field in fields.into_field_metas() {
            push_column(&mut self.columns, field);
        }
        self
    }

    /// Uses a select statement as the insert source.
    #[inline]
    pub fn from_select(mut self, select: Select) -> Self {
        self.source = Some(Box::new(select));
        self
    }

    /// Uses every exposed field from `source` as both target columns and the
    /// `INSERT ... SELECT` projection.
    ///
    /// This is the compact path for bulk loads from `values_source(...)` or
    /// staging sources whose exposed metadata already matches the insert target.
    pub fn from_select_all(mut self, source: impl Into<Source>) -> Self {
        let source = source.into();
        let qualifier = source.explicit_alias().map(str::to_owned);
        let mut projection = Vec::new();
        source.for_each_field(|field| {
            push_column(&mut self.columns, *field);
            projection.push(SelectItem {
                expr: ValueExpr::Field {
                    meta: *field,
                    qualifier: qualifier.clone(),
                },
                alias: None,
            });
        });
        let mut select = Select::from(source);
        select.projection = projection;
        self.source = Some(Box::new(select));
        self
    }

    /// Uses PostgreSQL `DEFAULT VALUES` instead of explicit insert values.
    ///
    /// This is for rows where every target column should be populated by its
    /// database default or remain nullable. It can still be combined with
    /// `RETURNING` and `ON CONFLICT`; validation rejects mixing it with
    /// `set(...)`, `values(...)`, or `from_select(...)`.
    #[inline]
    pub fn default_values(mut self) -> Self {
        self.default_values = true;
        self
    }

    /// Starts an `ON CONFLICT (columns...)` clause.
    ///
    /// Use this for column/index targets. For a named database constraint, use
    /// [`Insert::on_conflict_constraint`] with a generated constraint constant.
    pub fn on_conflict(self, fields: impl ConflictFields) -> ColumnConflictBuilder {
        let mut target_fields = Vec::with_capacity(fields.conflict_field_count());
        fields.push_conflict_fields(&mut target_fields);
        ColumnConflictBuilder {
            insert: self,
            fields: target_fields,
            predicate: None,
        }
    }

    /// Starts an `ON CONFLICT ON CONSTRAINT` clause.
    ///
    /// Generated schema exposes unique constraint names under
    /// `relation::constraints`.
    pub fn on_conflict_constraint(
        self,
        constraint: impl Into<String>,
    ) -> ConstraintConflictBuilder {
        ConstraintConflictBuilder {
            insert: self,
            constraint: constraint.into(),
        }
    }

    /// Adds one field to `RETURNING`.
    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    /// Replaces `RETURNING` with every field exposed by the target source.
    #[inline]
    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    /// Adds an arbitrary item to `RETURNING`.
    #[inline]
    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }
}

fn normalized_batch_assignments(assignments: Vec<Assignment>) -> Result<Vec<Assignment>> {
    let mut normalized = Vec::new();
    for assignment in assignments {
        push_assignment(&mut normalized, assignment);
    }
    if normalized.is_empty() {
        return Err(Error::InvalidInsertShape {
            message: "batch insert rows must contain at least one assignment",
        });
    }
    Ok(normalized)
}

fn same_batch_fields(columns: &[Meta], assignments: &[Assignment]) -> bool {
    columns.len() == assignments.len()
        && columns
            .iter()
            .zip(assignments)
            .all(|(column, assignment)| column.db == assignment.field.db)
}
