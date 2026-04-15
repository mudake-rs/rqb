use crate::Result;
use crate::typed::ident::{write_quoted_ident, write_quoted_qualified};
use crate::typed::{
    Assignment, BoolExpr, BuiltQuery, Delete, Insert, Param, Params, RawStmt, Select, SelectItem,
    Source, Stmt, ValueExpr, ValueOp,
};

#[derive(Default)]
struct Renderer {
    sql: String,
    params: Vec<Param>,
    cacheable: bool,
}

impl Renderer {
    fn new() -> Self {
        Self {
            cacheable: true,
            ..Self::default()
        }
    }

    fn finish(self) -> BuiltQuery {
        BuiltQuery {
            sql: self.sql,
            params: Params::from_vec(self.params),
            cacheable: self.cacheable,
        }
    }

    fn render_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Select(select) => self.render_select(select),
            Stmt::Insert(insert) => self.render_insert(insert),
            Stmt::Update(update) => self.render_update(update),
            Stmt::Delete(delete) => self.render_delete(delete),
            Stmt::Raw(raw) => self.render_raw_stmt(raw),
        }
    }

    fn render_select(&mut self, select: &Select) -> Result<()> {
        if let Source::Cte { name, stmt, .. } = &select.source {
            self.sql.push_str("WITH ");
            write_quoted_ident(&mut self.sql, name);
            self.sql.push_str(" AS (");
            self.render_stmt(stmt)?;
            self.sql.push_str(") ");
        }

        self.sql.push_str("SELECT ");
        self.render_projection(select)?;
        self.sql.push_str(" FROM ");
        self.render_source(&select.source)?;
        if let Some(filter) = &select.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
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
        Ok(())
    }

    fn render_insert(&mut self, insert: &Insert) -> Result<()> {
        self.sql.push_str("INSERT INTO ");
        self.render_write_target(&insert.target);
        self.sql.push_str(" (");
        for (idx, assignment) in insert.assignments.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, assignment.field.db);
        }
        self.sql.push_str(") VALUES (");
        for (idx, assignment) in insert.assignments.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(&assignment.value)?;
        }
        self.sql.push(')');
        self.render_returning(&insert.returning)?;
        Ok(())
    }

    fn render_update(&mut self, update: &crate::typed::Update) -> Result<()> {
        self.sql.push_str("UPDATE ");
        self.render_write_target(&update.target);
        self.sql.push_str(" SET ");
        self.render_assignments(&update.assignments)?;
        if let Some(filter) = &update.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_returning(&update.returning)?;
        Ok(())
    }

    fn render_delete(&mut self, delete: &Delete) -> Result<()> {
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&delete.target);
        if let Some(filter) = &delete.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_returning(&delete.returning)?;
        Ok(())
    }

    fn render_raw_stmt(&mut self, raw: &RawStmt) -> Result<()> {
        self.render_raw(&raw.sql, &raw.params)
    }

    fn render_projection(&mut self, select: &Select) -> Result<()> {
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

    fn render_source_fields(&mut self, source: &Source) {
        let mut rendered = 0usize;
        source.for_each_field(|field| {
            if rendered > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, field.db);
            if field.api != field.db {
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, field.api);
            }
            rendered += 1;
        });
        if rendered == 0 {
            self.sql.push('*');
        }
    }

    fn render_select_item(&mut self, item: &SelectItem) -> Result<()> {
        self.render_value(&item.expr)?;
        if let Some(alias) = &item.alias {
            self.sql.push_str(" AS ");
            write_quoted_ident(&mut self.sql, alias);
        }
        Ok(())
    }

    fn render_assignments(&mut self, assignments: &[Assignment]) -> Result<()> {
        for (idx, assignment) in assignments.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, assignment.field.db);
            self.sql.push_str(" = ");
            self.render_value(&assignment.value)?;
        }
        Ok(())
    }

    fn render_returning(&mut self, returning: &[SelectItem]) -> Result<()> {
        if returning.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" RETURNING ");
        for (idx, item) in returning.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_select_item(item)?;
        }
        Ok(())
    }

    fn render_order(&mut self, order: &[crate::typed::OrderItem]) -> Result<()> {
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

    fn render_source(&mut self, source: &Source) -> Result<()> {
        match source {
            Source::Table { name, .. } | Source::View { name, .. } => {
                write_quoted_qualified(&mut self.sql, name);
            }
            Source::Cte { name, .. } => {
                write_quoted_ident(&mut self.sql, name);
            }
            Source::Subquery { stmt, alias, .. } => {
                self.sql.push('(');
                self.render_stmt(stmt)?;
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            Source::Raw {
                sql, alias, params, ..
            } => {
                self.cacheable = false;
                self.sql.push('(');
                self.render_raw(sql, params)?;
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
        }
        Ok(())
    }

    fn render_write_target(&mut self, source: &Source) {
        match source {
            Source::Table { name, .. } | Source::View { name, .. } => {
                write_quoted_qualified(&mut self.sql, name);
            }
            Source::Cte { name, .. } => write_quoted_ident(&mut self.sql, name),
            Source::Subquery { .. } | Source::Raw { .. } => {
                unreachable!("write target validated as table")
            }
        }
    }

    fn render_bool(&mut self, expr: &BoolExpr) -> Result<()> {
        match expr {
            BoolExpr::Compare { left, op, right } => {
                self.render_value(left)?;
                self.sql.push(' ');
                self.sql.push_str(op.as_sql());
                self.sql.push(' ');
                self.render_value(right)
            }
            BoolExpr::And(exprs) => self.render_bool_list("AND", exprs),
            BoolExpr::Or(exprs) => self.render_bool_list("OR", exprs),
            BoolExpr::Not(expr) => {
                self.sql.push_str("NOT (");
                self.render_bool(expr)?;
                self.sql.push(')');
                Ok(())
            }
            BoolExpr::Exists(stmt) => {
                self.sql.push_str("EXISTS (");
                self.render_stmt(stmt)?;
                self.sql.push(')');
                Ok(())
            }
            BoolExpr::Raw { sql, params } => self.render_raw(sql, params),
        }
    }

    fn render_bool_list(&mut self, op: &str, exprs: &[BoolExpr]) -> Result<()> {
        self.sql.push('(');
        for (idx, expr) in exprs.iter().enumerate() {
            if idx > 0 {
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
            }
            self.render_bool(expr)?;
        }
        self.sql.push(')');
        Ok(())
    }

    fn render_value(&mut self, expr: &ValueExpr) -> Result<()> {
        match expr {
            ValueExpr::Field(field) => {
                write_quoted_ident(&mut self.sql, field.db);
                Ok(())
            }
            ValueExpr::Param(param) => {
                self.push_param(param.clone());
                Ok(())
            }
            ValueExpr::Function { name, args } => self.render_call(name, args),
            ValueExpr::Aggregate { name, args, filter } => {
                self.render_call(name, args)?;
                if let Some(filter) = filter {
                    self.sql.push_str(" FILTER (WHERE ");
                    self.render_bool(filter)?;
                    self.sql.push(')');
                }
                Ok(())
            }
            ValueExpr::Case { branches, else_ } => {
                self.sql.push_str("CASE");
                for (when, then) in branches {
                    self.sql.push_str(" WHEN ");
                    self.render_bool(when)?;
                    self.sql.push_str(" THEN ");
                    self.render_value(then)?;
                }
                if let Some(else_) = else_ {
                    self.sql.push_str(" ELSE ");
                    self.render_value(else_)?;
                }
                self.sql.push_str(" END");
                Ok(())
            }
            ValueExpr::Cast { expr, pg } => {
                self.sql.push_str("CAST(");
                self.render_value(expr)?;
                self.sql.push_str(" AS ");
                self.sql.push_str(pg);
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Binary { left, op, right } => {
                self.sql.push('(');
                self.render_value(left)?;
                self.sql.push(' ');
                self.sql.push_str(value_op_sql(*op));
                self.sql.push(' ');
                self.render_value(right)?;
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Raw { sql, params } => self.render_raw(sql, params),
            ValueExpr::Subquery(stmt) => {
                self.sql.push('(');
                self.render_stmt(stmt)?;
                self.sql.push(')');
                Ok(())
            }
        }
    }

    fn render_call(&mut self, name: &str, args: &[ValueExpr]) -> Result<()> {
        self.sql.push_str(name);
        self.sql.push('(');
        for (idx, arg) in args.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(arg)?;
        }
        self.sql.push(')');
        Ok(())
    }

    fn render_raw(&mut self, sql: &str, params: &[Param]) -> Result<()> {
        crate::typed::raw::validate_bind_count(sql, params.len())?;
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

    fn push_param(&mut self, param: Param) {
        self.params.push(param);
        self.sql.push('$');
        self.sql.push_str(&self.params.len().to_string());
    }
}

impl Stmt {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        let mut renderer = Renderer::new();
        renderer.render_stmt(self)?;
        Ok(renderer.finish())
    }
}

impl Select {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Select(self.clone()).build()
    }
}

impl Insert {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Insert(self.clone()).build()
    }
}

impl crate::typed::Update {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Update(self.clone()).build()
    }
}

impl Delete {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Delete(self.clone()).build()
    }
}

impl RawStmt {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Raw(self.clone()).build()
    }
}

fn value_op_sql(op: ValueOp) -> &'static str {
    match op {
        ValueOp::Add => "+",
        ValueOp::Sub => "-",
        ValueOp::Mul => "*",
        ValueOp::Div => "/",
        ValueOp::Custom(op) => op,
    }
}

#[cfg(test)]
mod tests {
    use crate::typed::{
        Assignment, BoolExpr, Field, Insert, Meta, OpSet, Param, RawStmt, Select, SelectItem,
        Source, Stmt, ValueExpr, insert, select, table, update,
    };

    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static EMAIL_META: Meta = Meta::new("email", "email_address", "text").ops(OpSet::ordered());
    static UUID_META: Meta = Meta::new("id", "id", "uuid").ops(OpSet::equality());
    static USERS_FIELDS: [&Meta; 2] = [&ID_META, &EMAIL_META];
    static UUID_FIELDS: [&Meta; 1] = [&UUID_META];
    const ID: Field<i32> = Field::new(&ID_META);
    const EMAIL: Field<String> = Field::new(&EMAIL_META);
    const UUID_ID: Field<uuid::Uuid> = Field::new(&UUID_META);

    fn users() -> Source {
        Source::Table {
            name: "public.app_users",
            fields: &USERS_FIELDS,
        }
    }

    #[test]
    fn select_renders_typed_predicate_and_default_projection() {
        let stmt = Stmt::Select(Select {
            source: users(),
            projection: Vec::new(),
            filter: Some(ID.eq(42)),
            order: Vec::new(),
            limit: None,
            offset: None,
        });

        let built = stmt.build().unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE \"id\" = $1"
        );
        assert_eq!(built.params.len(), 1);
        assert!(built.cacheable);
    }

    #[test]
    fn raw_fragments_are_numbered_in_render_order() {
        let stmt = Stmt::Select(Select {
            source: Source::Raw {
                sql: "select ?::int4 as id".to_owned(),
                alias: "generated".to_owned(),
                params: vec![Param::typed(1_i32)],
                fields: vec![ID_META],
            },
            projection: vec![SelectItem {
                expr: ValueExpr::Raw {
                    sql: "?::text".to_owned(),
                    params: vec![Param::typed("first".to_owned())],
                },
                alias: Some("label".to_owned()),
            }],
            filter: Some(BoolExpr::Raw {
                sql: "id > ?".to_owned(),
                params: vec![Param::typed(2_i32)],
            }),
            order: Vec::new(),
            limit: None,
            offset: None,
        });

        let built = stmt.build().unwrap();

        assert_eq!(
            built.sql,
            "SELECT $1::text AS \"label\" FROM (select $2::int4 as id) AS \"generated\" WHERE id > $3"
        );
        assert_eq!(built.params.len(), 3);
        assert!(!built.cacheable);
    }

    #[test]
    fn insert_renders_columns_values_and_returning() {
        let insert = Insert {
            target: users(),
            assignments: vec![Assignment {
                field: EMAIL_META,
                value: ValueExpr::Param(Param::typed("egor@example.com".to_owned())),
            }],
            returning: vec![SelectItem {
                expr: ID.expr(),
                alias: None,
            }],
        };

        let built = insert.build().unwrap();

        assert_eq!(
            built.sql,
            "INSERT INTO \"public\".\"app_users\" (\"email_address\") VALUES ($1) RETURNING \"id\""
        );
        assert_eq!(built.params.len(), 1);
    }

    #[test]
    fn raw_stmt_rejects_bind_mismatch_before_rendering() {
        let err = RawStmt {
            sql: "select ?".to_owned(),
            params: Vec::new(),
        }
        .build()
        .unwrap_err();

        assert!(matches!(
            err,
            crate::Error::RawBindMismatch {
                placeholders: 1,
                binds: 0
            }
        ));
    }

    #[test]
    fn typed_field_can_bind_any_sqlx_supported_type() {
        let stmt = Stmt::Select(Select {
            source: Source::Table {
                name: "app_users",
                fields: &UUID_FIELDS,
            },
            projection: vec![SelectItem {
                expr: UUID_ID.expr(),
                alias: None,
            }],
            filter: Some(UUID_ID.eq(uuid::Uuid::nil())),
            order: Vec::new(),
            limit: None,
            offset: None,
        });

        let built = stmt.build().unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"id\" FROM \"app_users\" WHERE \"id\" = $1"
        );
        assert_eq!(built.params.len(), 1);
    }

    #[test]
    fn ergonomic_constructors_build_the_same_typed_ast() {
        let built = select(table("public.app_users", &USERS_FIELDS))
            .column(ID)
            .item(EMAIL.alias("email"))
            .filter(BoolExpr::and([ID.gt(10), ID.lt(20)]))
            .order_desc(ID)
            .limit(50)
            .offset(100)
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"id\", \"email_address\" AS \"email\" FROM \"public\".\"app_users\" WHERE (\"id\" > $1 AND \"id\" < $2) ORDER BY \"id\" DESC LIMIT $3 OFFSET $4"
        );
        assert_eq!(built.params.len(), 4);
    }

    #[test]
    fn write_constructors_use_field_t_assignments() {
        let insert_sql = insert(users())
            .set(EMAIL.set("new@example.com".to_owned()))
            .returning(ID)
            .build()
            .unwrap();
        let update_sql = update(users())
            .set(EMAIL.set("updated@example.com".to_owned()))
            .filter(ID.eq(1))
            .returning(ID)
            .build()
            .unwrap();

        assert_eq!(
            insert_sql.sql,
            "INSERT INTO \"public\".\"app_users\" (\"email_address\") VALUES ($1) RETURNING \"id\""
        );
        assert_eq!(
            update_sql.sql,
            "UPDATE \"public\".\"app_users\" SET \"email_address\" = $1 WHERE \"id\" = $2 RETURNING \"id\""
        );
    }
}
