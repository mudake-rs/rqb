use crate::aggregate::Aggregate;
use crate::dataset::{Cte, Dataset, Join, JoinKind};
use crate::error::Error;
use crate::expr::{Expr, Sort, SortDir};
use crate::field::FieldRef;
use crate::request::{LockMode, RowLock, SearchRequest, SelectQuery};
use crate::sql_expr::SelectItem;

pub fn select(dataset: impl Into<Dataset>) -> SelectBuilder {
    SelectBuilder::new(dataset)
}

macro_rules! join_methods {
    ($($method:ident => $kind:ident),* $(,)?) => {
        $(
            pub fn $method(mut self, dataset: impl Into<Dataset>, on: impl Into<Expr>) -> Self {
                self.query
                    .joins
                    .push(Join::new(JoinKind::$kind, dataset, on));
                self
            }
        )*
    };
}

macro_rules! lateral_join_methods {
    ($($method:ident => $kind:ident),* $(,)?) => {
        $(
            pub fn $method(mut self, dataset: impl Into<Dataset>, on: impl Into<Expr>) -> Self {
                self.query
                    .joins
                    .push(Join::lateral(JoinKind::$kind, dataset, on));
                self
            }
        )*
    };
}

macro_rules! select_filter_methods {
    () => {
        pub fn filter(self, expr: impl Into<Expr>) -> Self {
            self.and_where(expr)
        }

        pub fn replace_filter(mut self, expr: impl Into<Expr>) -> Self {
            self.query.request.filter = Some(expr.into());
            self
        }

        pub fn and_where(mut self, expr: impl Into<Expr>) -> Self {
            self.query.request.filter = match self.query.request.filter.take() {
                Some(existing) => Some(existing.and(expr)),
                None => Some(expr.into()),
            };
            self
        }

        pub fn or_where(mut self, expr: impl Into<Expr>) -> Self {
            self.query.request.filter = match self.query.request.filter.take() {
                Some(existing) => Some(existing.or(expr)),
                None => Some(expr.into()),
            };
            self
        }

        pub fn filter_if(self, condition: bool, expr: impl Into<Expr>) -> Self {
            if condition {
                self.and_where(expr)
            } else {
                self
            }
        }

        pub fn filter_option<V, F>(self, value: Option<V>, f: F) -> Self
        where
            F: FnOnce(V) -> Expr,
        {
            match value {
                Some(value) => self.and_where(f(value)),
                None => self,
            }
        }
    };
}

macro_rules! apply_method {
    () => {
        pub fn apply<F>(self, f: F) -> Self
        where
            F: FnOnce(Self) -> Self,
        {
            f(self)
        }
    };
}

#[derive(Clone, Debug)]
#[must_use]
pub struct SelectBuilder {
    query: SelectQuery,
}

impl SelectBuilder {
    pub fn new(dataset: impl Into<Dataset>) -> Self {
        Self {
            query: SelectQuery::new(dataset),
        }
    }

    pub fn request(mut self, request: SearchRequest) -> Self {
        self.query.request.merge_in(request);
        self.query.cacheable = false;
        self
    }

    pub fn replace_request(mut self, request: SearchRequest) -> Self {
        self.query.request = request;
        self.query.cacheable = false;
        self
    }

    pub fn cacheable(mut self, cacheable: bool) -> Self {
        self.query.cacheable = cacheable;
        self
    }

    pub fn fields<I, F>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.query.projection = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn select<I, F>(self, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.fields(fields)
    }

    pub fn select_expr(mut self, item: SelectItem) -> Self {
        self.query.select_items.push(item);
        self
    }

    select_filter_methods!();
    apply_method!();

    pub fn order_by(mut self, sort: impl Into<Sort>) -> Self {
        self.query.request.sort.push(sort.into());
        self
    }

    join_methods!(
        join => Inner,
        left_join => Left,
        right_join => Right,
        full_join => Full,
    );

    lateral_join_methods!(
        join_lateral => Inner,
        left_join_lateral => Left,
    );

    pub fn cross_join(mut self, dataset: impl Into<Dataset>) -> Self {
        self.query.joins.push(Join::cross(dataset));
        self
    }

    pub fn cross_join_lateral(mut self, dataset: impl Into<Dataset>) -> Self {
        self.query.joins.push(Join::cross_lateral(dataset));
        self
    }

    pub fn sort_asc(self, field: impl Into<FieldRef>) -> Self {
        self.order_by(Sort::new(field, SortDir::Asc))
    }

    pub fn sort_desc(self, field: impl Into<FieldRef>) -> Self {
        self.order_by(Sort::new(field, SortDir::Desc))
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.query.request.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.query.request.offset = Some(offset);
        self
    }

    pub fn into_source(self, alias: impl Into<String>) -> Dataset {
        Dataset::subquery(self, alias)
    }

    pub fn cte(mut self, cte: Cte) -> Self {
        self.query.ctes.push(cte);
        self
    }

    pub fn distinct(mut self) -> Self {
        self.query.distinct = true;
        self
    }

    pub fn distinct_on<I, F>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.query.distinct_on = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn group_by<I, F>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.query.group_by = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn agg(mut self, aggregate: Aggregate) -> Self {
        self.query.aggregates.push(aggregate);
        self
    }

    pub fn json_agg<I, F>(self, alias: impl Into<String>, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.push_json_agg(alias, fields, true)
    }

    pub fn json_agg_nullable<I, F>(self, alias: impl Into<String>, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.push_json_agg(alias, fields, false)
    }

    fn push_json_agg<I, F>(
        mut self,
        alias: impl Into<String>,
        fields: I,
        default_empty: bool,
    ) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.query
            .aggregates
            .push(crate::aggregate::json_agg_with_default(
                alias,
                fields,
                default_empty,
            ));
        self
    }

    pub fn order_within(mut self, alias: &str, sort: impl Into<Sort>) -> Self {
        let sort = sort.into();
        match self
            .query
            .aggregates
            .iter_mut()
            .find(|aggregate| aggregate.alias() == alias)
        {
            Some(
                Aggregate::JsonAgg { order_by, .. }
                | Aggregate::ArrayAgg { order_by, .. }
                | Aggregate::StringAgg { order_by, .. },
            ) => *order_by = Some(sort),
            Some(_) => self
                .query
                .builder_errors
                .push(Error::AggregateOrderUnsupported {
                    alias: alias.to_owned(),
                }),
            None => self
                .query
                .builder_errors
                .push(Error::UnknownAggregateAlias {
                    alias: alias.to_owned(),
                }),
        }
        self
    }

    pub fn filter_agg(mut self, alias: &str, expr: impl Into<Expr>) -> Self {
        let expr = expr.into();
        if let Some(aggregate) = self
            .query
            .aggregates
            .iter_mut()
            .find(|aggregate| aggregate.alias() == alias)
        {
            aggregate.set_filter(expr);
        } else {
            self.query
                .builder_errors
                .push(Error::UnknownAggregateAlias {
                    alias: alias.to_owned(),
                });
        }
        self
    }

    pub fn having(mut self, expr: impl Into<Expr>) -> Self {
        self.query.having = Some(expr.into());
        self
    }

    pub fn lock(mut self, mode: LockMode) -> Self {
        self.query.lock = Some(RowLock::new(mode));
        self
    }

    pub fn for_update(self) -> Self {
        self.lock(LockMode::Update)
    }

    pub fn for_no_key_update(self) -> Self {
        self.lock(LockMode::NoKeyUpdate)
    }

    pub fn for_share(self) -> Self {
        self.lock(LockMode::Share)
    }

    pub fn for_key_share(self) -> Self {
        self.lock(LockMode::KeyShare)
    }

    pub fn nowait(mut self) -> Self {
        self.query.lock = Some(self.query.lock.unwrap_or_default().nowait());
        self
    }

    pub fn skip_locked(mut self) -> Self {
        self.query.lock = Some(self.query.lock.unwrap_or_default().skip_locked());
        self
    }

    pub fn build(self) -> SelectQuery {
        self.query
    }
}

impl From<SelectBuilder> for SelectQuery {
    fn from(builder: SelectBuilder) -> Self {
        builder.build()
    }
}
