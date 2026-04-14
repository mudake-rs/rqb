use crate::aggregate::SelectColumn;
use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::expr::Sort;
use crate::query::{QueryExpr, SetQuery};
use crate::types::FieldType;

use super::sql_expr::compatible_type;
use super::{ValidatedQueryExpr, ValidatedSelect, ValidatedSetQuery, ValidatedSetSort};

impl ValidatedQueryExpr {
    pub fn new(query: QueryExpr) -> Result<Self> {
        Self::new_with_outer_datasets(query, &[])
    }

    pub fn new_with_outer_datasets(query: QueryExpr, outer_datasets: &[Dataset]) -> Result<Self> {
        match query {
            QueryExpr::Select(select) => Ok(Self::Select(Box::new(
                ValidatedSelect::new_with_outer_datasets(*select, outer_datasets)?,
            ))),
            QueryExpr::Set(set) => Ok(Self::Set(Box::new(validate_set_query(
                *set,
                outer_datasets,
            )?))),
        }
    }
}

fn validate_set_query(query: SetQuery, outer_datasets: &[Dataset]) -> Result<ValidatedSetQuery> {
    let SetQuery {
        left,
        operator,
        right,
        sort,
        limit,
        offset,
        cacheable,
    } = query;

    let left = ValidatedQueryExpr::new_with_outer_datasets(left, outer_datasets)?;
    let right = ValidatedQueryExpr::new_with_outer_datasets(right, outer_datasets)?;
    let columns = validate_set_columns(left.columns(), right.columns())?;
    let sort = validate_set_sort(&sort, &columns)?;
    let cacheable = cacheable && left.cacheable() && right.cacheable();

    Ok(ValidatedSetQuery {
        left,
        operator,
        right,
        columns,
        sort,
        limit,
        offset,
        cacheable,
    })
}

fn validate_set_columns(
    left: &[SelectColumn],
    right: &[SelectColumn],
) -> Result<Vec<SelectColumn>> {
    if left.len() != right.len() {
        return Err(Error::InvalidSetOperationSelection {
            left: left.len(),
            right: right.len(),
        });
    }

    let mut columns = Vec::with_capacity(left.len());
    for (idx, (left, right)) in left.iter().zip(right).enumerate() {
        let left_type = left.ty();
        let right_type = right.ty();
        let Some(ty) = set_column_type(left_type, right_type) else {
            return Err(Error::IncompatibleSetOperationTypes {
                column: idx + 1,
                left_type: left_type.display_name().into_owned(),
                right_type: right_type.display_name().into_owned(),
            });
        };
        columns.push(SelectColumn::Expression {
            alias: left.alias(),
            ty,
        });
    }
    Ok(columns)
}

fn set_column_type(left: FieldType, right: FieldType) -> Option<FieldType> {
    compatible_type(left, right)
}

fn validate_set_sort(sort: &[Sort], columns: &[SelectColumn]) -> Result<Vec<ValidatedSetSort>> {
    sort.iter()
        .map(|sort| {
            let alias = sort.field.display_name();
            if columns.iter().any(|column| column.alias() == alias) {
                return Ok(ValidatedSetSort {
                    alias,
                    dir: sort.dir,
                    nulls: sort.nulls,
                });
            }
            Err(Error::UnknownField {
                dataset: "set operation".to_owned(),
                field: alias,
            })
        })
        .collect()
}
