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

    pub(super) fn from_datasets(root: &Dataset, datasets: &[Dataset]) -> Result<Self> {
        let mut scoped = Vec::with_capacity(datasets.len() + 1);
        scoped.push(ScopedDataset {
            dataset: root.clone(),
        });
        scoped.extend(
            datasets
                .iter()
                .cloned()
                .map(|dataset| ScopedDataset { dataset }),
        );
        Self::from_scoped(scoped)
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

        Self::from_scoped(datasets)
    }

    fn from_scoped(datasets: Vec<ScopedDataset>) -> Result<Self> {
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
        let mut found: Option<usize> = None;
        let mut ambiguous: Option<Vec<String>> = None;
        for (idx, scoped) in self.datasets.iter().enumerate() {
            if !scoped.dataset.matches_qualifier(qualifier) {
                continue;
            }

            if let Some(matches) = &mut ambiguous {
                matches.push(Self::label(&scoped.dataset));
            } else if let Some(first_idx) = found {
                ambiguous = Some(vec![
                    Self::label(&self.datasets[first_idx].dataset),
                    Self::label(&scoped.dataset),
                ]);
            } else {
                found = Some(idx);
            }
        }

        if let Some(matches) = ambiguous {
            return Err(Error::AmbiguousDatasetQualifier {
                qualifier: qualifier.to_owned(),
                matches: matches.join(", "),
            });
        }

        match found {
            Some(idx) => Ok(&self.datasets[idx]),
            None => Err(Error::UnknownDatasetQualifier {
                qualifier: qualifier.to_owned(),
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
