use std::collections::BTreeSet;

use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::request::SelectQuery;

#[derive(Clone)]
pub(super) struct ScopedDataset {
    pub(super) dataset: Dataset,
}

pub(super) struct QueryScope {
    pub(super) datasets: Vec<ScopedDataset>,
    pub(super) has_joins: bool,
}

impl QueryScope {
    pub(super) fn from_dataset(dataset: &Dataset) -> Self {
        Self {
            datasets: vec![ScopedDataset {
                dataset: dataset.clone(),
            }],
            has_joins: false,
        }
    }

    pub(super) fn new_with_outer(query: &SelectQuery, outer_datasets: &[Dataset]) -> Result<Self> {
        let mut datasets = Vec::with_capacity(query.joins.len() + outer_datasets.len() + 1);
        datasets.push(ScopedDataset {
            dataset: query.dataset.clone(),
        });
        datasets.extend(query.joins.iter().map(|join| ScopedDataset {
            dataset: join.dataset.clone(),
        }));
        datasets.extend(
            outer_datasets
                .iter()
                .cloned()
                .map(|dataset| ScopedDataset { dataset }),
        );

        let has_joins = datasets.len() > 1;
        if has_joins {
            let mut qualifiers = BTreeSet::new();
            for scoped in &datasets {
                let qualifier = scoped.dataset.sql_qualifier();
                if !qualifiers.insert(qualifier.to_owned()) {
                    return Err(Error::DuplicateDatasetQualifier {
                        qualifier: qualifier.to_owned(),
                    });
                }
            }
        }

        Ok(Self {
            datasets,
            has_joins,
        })
    }

    pub(super) fn root(&self) -> &Dataset {
        debug_assert!(!self.datasets.is_empty());
        &self.datasets[0].dataset
    }

    pub(super) fn label(dataset: &Dataset) -> String {
        dataset
            .source_alias()
            .unwrap_or(dataset.sql_qualifier())
            .to_owned()
    }

    pub(super) fn find_qualified(&self, qualifier: &str) -> Result<&ScopedDataset> {
        let matches = self
            .datasets
            .iter()
            .filter(|scoped| scoped.dataset.matches_qualifier(qualifier))
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [] => Err(Error::UnknownDatasetQualifier {
                qualifier: qualifier.to_owned(),
            }),
            [scoped] => Ok(scoped),
            many => Err(Error::AmbiguousDatasetQualifier {
                qualifier: qualifier.to_owned(),
                matches: many
                    .iter()
                    .map(|scoped| Self::label(&scoped.dataset))
                    .collect::<Vec<_>>()
                    .join(", "),
            }),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum ExprContext {
    Filter,
    JoinOn,
    Having,
}
