use crate::common::{CrateId, Result};
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

    pub fn get_dependencies(
        &mut self,
        crate_ids: &Vec<CrateId>,
    ) -> Result<Vec<CrateId>> {
        for crate_id in crate_ids {
            if let Some(index_version) = self.get_crate_version_from_index(crate_id) {
                // Assume all dependencies are enabled for top-level crates.
                for index_dep in index_version.dependencies().iter().filter(|d| {
                    d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build
                }) {
                    // Determine the features that are enabled for the crate.
                    let enabled_features = self.get_enabled_dependency_features(index_version, index_dep);
                }

                self.update_dependencies(index_version);
            }
        }
        unimplemented!()
    }

    fn update_dependencies(&mut self, version: &crates_index::Version) {}

    fn get_crate_version_from_index(
        &self,
        crate_id: &CrateId,
    ) -> Option<&crates_index::Version> {
        if let Some(index_crate) = self.index.crate_(&crate_id.name) {
            for index_version in index_crate.versions().iter().rev() {
                if index_version.version() == crate_id.version {
                    return Some(index_version);
                }
            }
            warn!(
                "failed to find version {} of the '{}' crate in the source index",
                crate_id.version, crate_id.name
            );
        } else {
            warn!(
                "failed to find the '{}' crate in the source index",
                crate_id.name
            );
        }
        None
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
    
    fn get_enabled_dependency_features(
        &self,
        crate_version: &crates_index::Version,
        enabled_crate_features: &Vec<String>,
        dependency: &crates_index::Dependency,
    ) -> Vec<String> {
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
        
        // Now, using the features enabled for current crate, recursively look through
        // the features table to determine if any additional features are enabled for
        // the dependency.
        let mut enabled_crate_features = enabled_crate_features.iter().cloned().collect::<StoredIndexVec<String>>();
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

struct DependencyFeature {
    name: String,
    is_weak: bool,
}

fn parse_dependency_feature(feat_or_dep: &String) -> Option<DependencyFeature> {
    // There are two ways an entry in the array portion of a crate's feature table can enable
    // a feature of a dependency:
    //     1. dep/feat    (strong: Implicitly enables the dependency, then enables the dependency's feature.)
    //     2. dep?/feat   (weak:   Does not enable the dependency.
    //                             Only enables the dependency's feature if the dependency is enabled
    //                             by another feature.)
    let parts = feat_or_dep.split("/").collect::<Vec<_>>();
    if parts.len() == 1 {
        return None
    }
    let dep_name = parts[0];
    let dep_feat = parts[1];

    None
}