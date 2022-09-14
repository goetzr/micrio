use crate::common::{CrateId, MicrioError, Result};
use crates_index::DependencyKind;
use log::warn;
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

pub struct SrcIndex {
    index: crates_index::Index,
    crates_map: HashMap<CrateId, FeaturesList>,
}

struct StoredIndexVec<T> {
    items: Vec<T>,
    index: usize,
}

impl<T: Clone> StoredIndexVec<T> {
    fn new() -> Self {
        StoredIndexVec { items: Vec::new(), index: 0 }
    }

    fn next_item(&mut self) -> Option<T> {
        if self.index < self.items.len() {
            let next_item = &self.items[self.index];
            self.index += 1;
            Some(next_item.clone())
        } else {
            None
        }
    }
}

impl<T> Deref for StoredIndexVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for StoredIndexVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl<T> FromIterator<T> for StoredIndexVec<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let items = iter.into_iter().collect::<Vec<_>>();
        StoredIndexVec { items, index: 0 }
    }
}

impl SrcIndex {
    pub fn new() -> Result<Self> {
        let index = crates_index::Index::new_cargo_default()?;
        Ok(SrcIndex {
            index,
            crates_map: HashMap::new(),
        })
    }

    pub fn get_with_dependencies(
        &mut self,
        crate_ids: &Vec<CrateId>,
    ) -> Result<Vec<CrateId>> {
        for crate_id in crate_ids {
            let crate_version = self.get_crate_version(crate_id)?;
            let parsed_features_table = parse_features_table(crate_version)?;
            // Enable all features for top-level crates.
            let enabled_features = parsed_features_table.iter().map(|(feature, _)| feature.clone()).collect::<Vec<_>>();
            let mut enabled_dependencies = Vec::new();
            for dependency in crate_version
                .dependencies()
                .iter()
                .filter(|d|
                {
                    d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build
                })
                {
                    
                }
        }
        unimplemented!()
    }

    fn get_crate_version(
        &self,
        crate_id: &CrateId,
    ) -> Result<&crates_index::Version> {
        self.index.crate_(&crate_id.name).ok_or(
            MicrioError::CrateNotFound { crate_name: crate_id.name.clone(), crate_version: crate_id.version.clone() }
        )
    }
}

enum FeatureTableEntry {
    Feature(String),
    Dependency(String),
    WeakDependencyFeature {
        dep_name: String,
        feature: String,
    },
    StrongDependencyFeature {
        dep_name: String,
        feature: String,
    }
}

fn parse_features_table(
    crate_version: &crates_index::Version,
) -> Result<HashMap<String, Vec<FeatureTableEntry>>> {
    let mut parsed_features_table = HashMap::new();
    for (feature, entries) in crate_version.features() {
        let mut parsed_entries = Vec::new();
        for entry in entries {
            let parsed_entry = parse_feature_table_entry(crate_version, feature, entry)?;
            parsed_entries.push(parsed_entry);
        }
        parsed_features_tables.insert(feature.clone(), parsed_entries);
    }
    Ok(parsed_features_table)
}

fn parse_feature_table_entry(
    crate_version: &crates_index::Version,
    feature: &String,
    entry: &String
) -> Result<FeatureTableEntry> {
    let parts = entry.split("/").collect::<Vec<_>>();
    match parts.len() {
        1 => {
            let name = parts[0];
            if is_feature_of(name, crate_version) {
                Ok(FeatureTableEntry::Feature(name.to_string()))
            } else if is_optional_dependency_of(name, crate_version) {
                Ok(FeatureTableEntry::Dependency(name.to_string()))
            } else {
                Err(MicrioError::FeatureTable {
                    crate_name: crate_version.name().to_string(),
                    crate_version: crate_version.version().to_string(),
                    error_msg: format!("entry '{entry}' in feature '{feature}': '{entry}' not a feature or an optional dependency")
                })
            }
        },
        2 => {
            // This should be a feature of the dependency, not the current crate.
            // Delay checking this until later.
            let feat_name = parts[1];

            let (dep_name, is_weak) = match parts[0].find("?") {
                None => (parts[0], false),
                Some(idx) => {
                    if idx == parts[0].len() - 1 {
                        // Trim off the trailing '?'.
                        (&parts[0][..parts[0].len() - 1], true)
                    } else {
                        return Err(MicrioError::FeatureTable {
                            crate_name: crate_version.name().to_string(),
                            crate_version: crate_version.version().to_string(),
                            error_msg: format!("entry '{entry}' in feature '{feature}': '?' not at end of dependency name")
                        })
                    }
                },
            };

            if is_weak {
                if !is_optional_dependency_of(dep_name, crate_version) {
                    return Err(MicrioError::FeatureTable {
                        crate_name: crate_version.name().to_string(),
                        crate_version: crate_version.version().to_string(),
                        error_msg: format!("entry '{entry}' in feature '{feature}': name before '/' not an optional dependency")
                    })
                }
                Ok(FeatureTableEntry::WeakDependencyFeature { dep_name: dep_name.to_string(), feature: feat_name.to_string() })
            } else {
                if !is_dependency_of(dep_name, crate_version) {
                    return Err(MicrioError::FeatureTable {
                        crate_name: crate_version.name().to_string(),
                        crate_version: crate_version.version().to_string(),
                        error_msg: format!("entry '{entry}' in feature '{feature}': name before '/' not a dependency")
                    })
                }
                Ok(FeatureTableEntry::StrongDependencyFeature { dep_name: dep_name.to_string(), feature: feat_name.to_string() })
            }
        },
        _ => Err(MicrioError::FeatureTable {
            crate_name: crate_version.name().to_string(),
            crate_version: crate_version.version().to_string(),
            error_msg: format!("entry '{entry}' in feature '{feature}': multiple '/' separators")
        })
    }
}

fn is_feature_of(name: &str, crate_version: &crates_index::Version) -> bool {
    crate_version
        .features()
        .iter()
        .position(|(feat, _)| feat == name)
        .is_some()
}

fn is_optional_dependency_of(name: &str, crate_version: &crates_index::Version) -> bool {
    crate_version
        .dependencies()
        .iter()
        .filter(|dep| {
            dep.is_optional() &&
            (dep.kind() == DependencyKind::Normal || dep.kind() == DependencyKind::Build)
        })
        .position(|dep| dep.name() == name)
        .is_some()
}

fn is_dependency_of(name: &str, crate_version: &crates_index::Version) -> bool {
    crate_version
        .dependencies()
        .iter()
        .filter(|dep| {
            dep.kind() == DependencyKind::Normal || dep.kind() == DependencyKind::Build
        })
        .position(|dep| dep.name() == name)
        .is_some()
}

fn is_dependency_enabled(
    dependency: &crates_index::Dependency,
    parsed_features_table: &HashMap<String, Vec<FeatureTableEntry>>,
    enabled_features: &Vec<String>,
) -> bool {
    // Walk the features table, building up the list of enabled features, until it's determined
    // that the depdendency is enabled or we hit the end of the features table.
    let mut enabled_features = enabled_features.iter().cloned().collect::<StoredIndexVec<String>>();
    let mut feature = enabled_features.next_item();
    while let Some(mut feature) = feature {
        for entry in 
    }
    false
}

fn feature_enables_dependency(
    feature: &String,
    dependency: &crates_index::Dependency,
) -> bool {

}

fn feature_table_entry_enables_dependency(
    entry: &FeatureTableEntry,
    dependency: &crates_index::Dependency,
    enabled_features: &Vec<String>,
) -> bool {
    match entry {
        FeatureTableEntry::Dependency(dep_name) => dep_name == dependency.name(),
        FeatureTableEntry::StrongDependencyFeature { dep_name, feature } => dep_name == dependency.name(),
        FeatureTableEntry::Feature(feature) => feature_enables_dependency(feature, feature_entries, dependency),
        _ => false,
    }
}

fn feature_enables_dependency(
    feature: &String,
    feature_entries: &Vec<FeatureTableEntry>,
    dependency: &crates_index::Dependency
) -> bool {
    for entry in feature_entries {
        if feature_table_entry_enables_dependency(entry, dependency) {
            return true;
        }
    }
    false
}