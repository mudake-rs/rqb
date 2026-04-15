use crate::typed::{BoolExpr, Field, Meta, Param, Params, Source, ValueExpr, raw as raw_sql};
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct SelectItem {
    pub expr: ValueExpr,
    pub alias: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub struct OrderItem {
    pub expr: ValueExpr,
    pub direction: OrderDirection,
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub field: Meta,
    pub value: ValueExpr,
}

#[derive(Clone, Debug)]
pub struct Select {
    pub source: Source,
    pub projection: Vec<SelectItem>,
    pub filter: Option<BoolExpr>,
    pub order: Vec<OrderItem>,
    pub limit: Option<Param>,
    pub offset: Option<Param>,
}

#[derive(Clone, Debug)]
pub struct Insert {
    pub target: Source,
    pub assignments: Vec<Assignment>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct Update {
    pub target: Source,
    pub assignments: Vec<Assignment>,
    pub filter: Option<BoolExpr>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct Delete {
    pub target: Source,
    pub filter: Option<BoolExpr>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct RawStmt {
    pub sql: String,
    pub params: Vec<Param>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Select(Select),
    Insert(Insert),
    Update(Update),
    Delete(Delete),
    Raw(RawStmt),
}

impl SelectItem {
    pub fn new(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            alias: None,
        }
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

impl Assignment {
    pub fn new<T>(field: Field<T>, value: impl Into<ValueExpr>) -> Self {
        Self {
            field: *field.meta,
            value: value.into(),
        }
    }
}

impl OrderDirection {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl OrderItem {
    pub fn asc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Asc,
        }
    }

    pub fn desc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Desc,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.expr.validate()?;
        if let Some(meta) = self.expr.field_meta()
            && !meta.ops.ordering
        {
            return Err(Error::InvalidTypedSort {
                field: meta.api.to_owned(),
            });
        }
        Ok(())
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        self.expr.collect_params(params);
    }
}

pub fn select(source: Source) -> Select {
    Select::from(source)
}

pub fn insert(target: Source) -> Insert {
    Insert::into(target)
}

pub fn update(target: Source) -> Update {
    Update::table(target)
}

pub fn delete_from(target: Source) -> Delete {
    Delete::from(target)
}

pub fn raw(sql: impl Into<String>) -> RawStmt {
    RawStmt::new(sql)
}

impl Stmt {
    pub fn raw(sql: impl Into<String>) -> Self {
        Self::Raw(RawStmt::new(sql))
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Select(select) => select.validate(),
            Self::Insert(insert) => insert.validate(),
            Self::Update(update) => update.validate(),
            Self::Delete(delete) => delete.validate(),
            Self::Raw(raw_stmt) => raw_stmt.validate(),
        }
    }

    pub fn params(&self) -> Params {
        let mut params = Vec::new();
        self.collect_params(&mut params);
        Params::from_vec(params)
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Select(select) => select.collect_params(params),
            Self::Insert(insert) => insert.collect_params(params),
            Self::Update(update) => update.collect_params(params),
            Self::Delete(delete) => delete.collect_params(params),
            Self::Raw(raw_stmt) => params.extend(raw_stmt.params.iter().cloned()),
        }
    }
}

impl Select {
    pub fn from(source: Source) -> Self {
        Self {
            source,
            projection: Vec::new(),
            filter: None,
            order: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn column<T>(mut self, field: Field<T>) -> Self {
        self.projection.push(select_item_for_field(field));
        self
    }

    pub fn expr(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.projection.push(SelectItem {
            expr: expr.into(),
            alias: None,
        });
        self
    }

    pub fn item(mut self, item: SelectItem) -> Self {
        self.projection.push(item);
        self
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
        self
    }

    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order.push(item);
        self
    }

    pub fn order_asc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc(expr));
        self
    }

    pub fn order_desc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc(expr));
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(Param::typed(i64::from(limit)));
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(Param::typed(i64::from(offset)));
        self
    }

    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        for item in &self.projection {
            item.expr.validate()?;
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        for item in &self.order {
            item.validate()?;
        }
        Ok(())
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        self.source.collect_prefix_params(params);
        for item in &self.projection {
            item.expr.collect_params(params);
        }
        self.source.collect_from_params(params);
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
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
    pub fn into(target: Source) -> Self {
        Self {
            target,
            assignments: Vec::new(),
            returning: Vec::new(),
        }
    }

    pub fn set(mut self, assignment: Assignment) -> Self {
        self.assignments.push(assignment);
        self
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_table_target("insert", &self.target)?;
        validate_nonempty_assignments("insert", &self.assignments)?;
        for assignment in &self.assignments {
            assignment.value.validate()?;
        }
        validate_returning(&self.returning)
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        for assignment in &self.assignments {
            assignment.value.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl Update {
    pub fn table(target: Source) -> Self {
        Self {
            target,
            assignments: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn set(mut self, assignment: Assignment) -> Self {
        self.assignments.push(assignment);
        self
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
        self
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_table_target("update", &self.target)?;
        validate_nonempty_assignments("update", &self.assignments)?;
        for assignment in &self.assignments {
            assignment.value.validate()?;
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        validate_returning(&self.returning)
    }

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
    pub fn from(target: Source) -> Self {
        Self {
            target,
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
        self
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_table_target("delete", &self.target)?;
        let Some(filter) = &self.filter else {
            return Err(Error::TypedDeleteWithoutFilter);
        };
        filter.validate()?;
        validate_returning(&self.returning)
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl RawStmt {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        raw_sql::validate_bind_count(&self.sql, self.params.len())
    }

    pub fn bind<T>(mut self, value: T) -> Self
    where
        T: Clone
            + Send
            + Sync
            + 'static
            + for<'q> sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>,
    {
        self.params.push(Param::typed(value));
        self
    }
}

fn validate_table_target(statement: &'static str, target: &Source) -> Result<()> {
    if target.is_table() {
        return Ok(());
    }
    Err(Error::InvalidTypedWriteTarget {
        statement,
        source_kind: target.kind(),
    })
}

fn validate_nonempty_assignments(
    statement: &'static str,
    assignments: &[Assignment],
) -> Result<()> {
    if assignments.is_empty() {
        return Err(Error::EmptyTypedAssignments { statement });
    }
    Ok(())
}

fn validate_returning(returning: &[SelectItem]) -> Result<()> {
    for item in returning {
        item.expr.validate()?;
    }
    Ok(())
}

fn collect_returning_params(returning: &[SelectItem], params: &mut Vec<Param>) {
    for item in returning {
        item.expr.collect_params(params);
    }
}

fn select_item_for_field<T>(field: Field<T>) -> SelectItem {
    let alias = field_alias(field.meta);
    SelectItem {
        expr: field.expr(),
        alias,
    }
}

fn field_alias(meta: &Meta) -> Option<String> {
    (meta.api != meta.db).then(|| meta.api.to_owned())
}

#[cfg(test)]
mod tests {
    use crate::typed::{
        BoolExpr, BoolOp, Field, Meta, OpSet, OrderItem, Select, SelectItem, Source, ValueExpr,
    };

    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static ID_FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<i32> = Field::new(&ID_META);

    fn users() -> Source {
        Source::Table {
            name: "app_users",
            fields: &ID_FIELDS,
        }
    }

    #[test]
    fn subquery_value_expr_collects_nested_params_at_expression_position() {
        let subquery = crate::typed::Stmt::Select(Select {
            source: users(),
            projection: vec![SelectItem {
                expr: ID.expr(),
                alias: None,
            }],
            filter: Some(ID.eq(10)),
            order: Vec::new(),
            limit: None,
            offset: None,
        });
        let outer = ValueExpr::Subquery(Box::new(subquery));

        let mut params = Vec::new();
        outer.collect_params(&mut params);

        assert_eq!(params.len(), 1);
    }

    #[test]
    fn select_params_follow_sql_text_order() {
        let source = Source::Raw {
            sql: "select ?::int4 as id".to_owned(),
            alias: "generated".to_owned(),
            params: vec![crate::typed::Param::typed(1_i32)],
            fields: vec![ID_META],
        };
        let stmt = crate::typed::Stmt::Select(Select {
            source,
            projection: vec![SelectItem {
                expr: ValueExpr::Param(crate::typed::Param::typed(2_i32)),
                alias: Some("projected".to_owned()),
            }],
            filter: Some(BoolExpr::Compare {
                left: ID.expr(),
                op: BoolOp::Eq,
                right: ValueExpr::Param(crate::typed::Param::typed(3_i32)),
            }),
            order: vec![OrderItem::asc(ID)],
            limit: Some(crate::typed::Param::typed(10_i64)),
            offset: Some(crate::typed::Param::typed(5_i64)),
        });

        let params = stmt.params();

        assert_eq!(params.len(), 5);
        stmt.validate().unwrap();
    }

    #[test]
    fn delete_requires_filter() {
        let stmt = crate::typed::Stmt::Delete(crate::typed::Delete {
            target: users(),
            filter: None,
            returning: Vec::new(),
        });

        assert!(matches!(
            stmt.validate().unwrap_err(),
            crate::Error::TypedDeleteWithoutFilter
        ));
    }
}
