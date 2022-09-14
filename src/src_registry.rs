use crate::common::{CrateId, MicrioError, Result};
use crates_index::DependencyKind;
use log::warn;
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

struct FeaturesList {
    list: Vec<String>,
}

impl FeaturesList {
    fn new() -> Self {
        FeaturesList { list: Vec::new() }
    }

    fn add_feature(&mut self, feature: &str) {
        if self.list.iter().position(|feat| feat == feature).is_none() {
            self.list.push(feature.to_string());
        }
    }
}

impl Deref for FeaturesList {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl DerefMut for FeaturesList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

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

    // TODO: Rename this function.
    pub fn get_with_dependencies(
        &mut self,
        crate_ids: &Vec<CrateId>,
    ) -> Result<Vec<CrateId>> {
        for crate_id in crate_ids {
            let crate_version = self.get_crate_version(crate_id)?;
            let parsed_features_table = parse_features_table(crate_version)?;
            // Enable all features for top-level crates.
            let enabled_features = parsed_features_table.iter().map(|feature, _| feature.clone()).collect::<Vec<_>>();
            let mut enabled_dependencies = Vec::new();
            for dependency in crate_version.dependencies().iter().filter(|d| {
                d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build
            }) {

                // Determine the features that are enabled for the crate.
                let enabled_features = self.get_enabled_dependency_features(index_version, index_dep);
            }

            self.update_dependencies(index_version);
        }
        unimplemented!()
    }

    fn update_dependencies(&mut self, version: &crates_index::Version) {}

    fn get_crate_version(
        &self,
        crate_id: &CrateId,
    ) -> Result<&crates_index::Version> {
        self.index.crate_(&crate_id.name).ok_or(
            MicrioError::CrateNotFound { crate_name: crate_id.name.clone(), crate_version: crate_id.version.clone() }
        )
    }

    // TODO: Merge this functionality into get_enabled_dependency_features.
    fn is_dependency_enabled(
        &self,
        crate_version: &crates_index::Version,
        enabled_crate_version: &Vec<String>,
        dependency: &crates_index::Dependency,
    ) -> bool {
        unimplemented!()
    }
    
    // While determining the dependency's enabled features, we also find out if an
    // optional dependency is enabled.
    // The difficulty here is we can have:
    //     A required dependency, which is always enabled.
    //         Return the list of enabled features for the dependency
    //     An optional dependency, which may or may not be enabled.
    //         Return the list of enabled features for the dependency if it's enabled,
    //         as well as whether the dependency is enabled or not.
    /*
    for dependency in crate_version.dependencies() {
        // PROBLEM: is_dependency_enabled() would do a lot of the same work.
        if !dependency.is_optional() || this.is_dependency_enabled(...) {
            let enabled_features = self.get_enabled_dependency_features(...);
            add_crate(dependency, enabled_features);
        }
        if dependency.is_optional() {

        } else {
            let enabled_features = self.get_enabled_dependency_features(
                crate_version,
                enabled_crate_features,
                dependency
            );
        }
    }
    */

    fn is_optional_dependency_enabled (
        &self,
        crate_version: &crates_index::Version,
        enabled_crate_features: &Vec<String>,
        optional_dependency: &crates_index::Dependency,
    ) -> bool {
        // Using the features enabled for the crate, recursively look through the
        // features table to determine if the optional dependency is enabled.
        let mut enabled_crate_features = enabled_crate_features
            .iter()
            .cloned()
            .collect::<StoredIndexVec<String>>();
        while let Some(enabled_crate_feature) = enabled_crate_features.next_item() {
            crate_version.features().iter().map(|(feat, feat_or_dep_arr)| {
                if feat == &enabled_crate_feature {
                    for feat_or_dep in feat_or_dep_arr {
                        if enables_optional_dependency(feat_or_dep, optional_dependency) {
                            return true;
                        }
                        // TODO: Need to add feat_or_dep to enabled_crate_features,
                        //       but only if it's a feature.
                        //       Could be:
                        //           dep       (some other dependency)
                        //           dep/feat  (some other dependency)
                        //           dep?feat  (doesn't enable dependency)
                        //           feat      (this is where we want to add)
                    }
                }
            });
        }

        return false;
    }

    fn get_enabled_dependency_features(
        &self,
        crate_version: &crates_index::Version,
        enabled_crate_features: &Vec<String>,
        dependency: &crates_index::Dependency,
    ) -> Vec<String> {
        // TODO: We could easily add these features in later, after we've determined
        //       that the dependency is actually enabled.
        //
        /*
        // Start with the features explicitly enabled for the dependency where it's specified in
        // the crate's Cargo.toml file.
        let mut enabled_features = dependency.features().iter().cloned().collect::<Vec<_>>();

        // Add the implicit default feature if it's not explicity disabled or already in the list.
        const DEFAULT_FEATURE: &'static str = "default";
        if dependency.has_default_features()
            && enabled_features.iter().position(|f| f == DEFAULT_FEATURE) == None
        {
            enabled_features.push(DEFAULT_FEATURE.to_string());
        }
        */
        
        // Using the features enabled for crate, recursively look through the
        // features table to determine if any additional features are enabled for
        // the dependency.
        let mut enabled_crate_features = enabled_crate_features
            .iter()
            .cloned()
            .collect::<StoredIndexVec<String>>();
        while let Some(crate_feature) = enabled_crate_features.next_item() {
            crate_version.features().iter().map(|(feat, feat_or_dep_arr)| {
                if feat == &crate_feature {
                    /*for feat_or_dep in feat_or_dep_arr {
                        if feat == 
                    }*/
                }
            });
        }
        unimplemented!()
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
        parsed_feature_tables.insert(feature.clone(), parsed_entries);
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

fn get_enabled_dependencies<'a>(
    crate_version: &'a crates_index::Version,
    enabled_features: &HashMap<String, Vec<FeatureTableEntry>>
) -> Vec<&'a crates_index::Dependency> {
    let mut enabled_dependencies = Vec::new();
    for dependency in crate_version.dependencies() {
        if !dependency.is_optional() || is_dependency_enabled(crate_version, enabled_features, dependency) {
            enabled_dependencies.push(dependency);
        }
    }
    enabled_dependencies
}

fn is_dependency_enabled (
    crate_version: &crates_index::Version,
    enabled_features: &HashMap<String, Vec<FeatureTableEntry>>,
    dependency: &crates_index::Dependency
) -> bool {
    for (_, entries) in enabled_features {
        for entry in entries {
            if feature_table_entry_enables_dependency(entry, dependency) {
                return true;
            }

        }
    }
    false
}

fn feature_table_entry_enables_dependency(
    entry: &FeatureTableEntry,
    dependency: &crates_index::Dependency
) -> bool {
    match entry {
        FeatureTableEntry::Dependency(dep_name) => dep_name == dependency.name(),
        FeatureTableEntry::StrongDependencyFeature { dep_name, feature } => dep_name == dependency.name(),
        // TODO: Pass in enabled_features
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

fn enables_optional_dependency(
    feat_or_dep: &String,
    optional_dependency: &crates_index::Dependency
) -> bool {
    // There are two ways an entry in the array portion of a crate's feature table can enable
    // an optional dependency:
    //     1. dep
    //     2. dep/feat
    let parts = feat_or_dep.split("/").collect::<Vec<_>>();
    parts[0] == optional_dependency.name()
}

struct DependencyFeature {
    name: String,
    is_weak: bool,
}

fn parse_dependency_feature(feat_or_dep: &String) -> std::result::Result<Option<DependencyFeature>> {
    // There are two ways an entry in the array portion of a crate's feature table can enable
    // a feature of a dependency:
    //     1. dep/feat   (strong feature)
    //            Enables the dependency, then enables the dependency's feature.
    //     2. dep?/feat  (weak feature)
    //            Does not enable the dependency.
    //            Only enables the dependency's feature if the dependency is enabled by another feature.
    let parts = feat_or_dep.split("/").collect::<Vec<_>>();
    if parts.len() == 1 {
        return None
    }
    let dep_name = parts[0];
    let dep_feat = parts[1];

    None
}