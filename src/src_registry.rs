use crate::common::{CrateVersion, Result};
use crates_index::DependencyKind;
use log::warn;
use std::{
    collections::HashMap,
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
    crates_map: HashMap<CrateVersion, FeaturesList>,
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
        crate_versions: &Vec<CrateVersion>,
    ) -> Result<Vec<CrateVersion>> {
        for crate_version in crate_versions {
            if let Some(index_version) = self.get_crate_version_from_index(crate_version) {
                // Assume all dependencies are enabled for top-level crates.
                for index_dep in index_version.dependencies().iter().filter(|d| {
                    d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build
                }) {
                    // Determine the features that are enabled for the crate.
                    let enabled_features = self.get_enabled_features(index_version, index_dep);
                }

                self.update_dependencies(index_version);
            }
        }
        unimplemented!()
    }

    fn update_dependencies(&mut self, version: &crates_index::Version) {}

    fn get_crate_version_from_index(
        &self,
        crate_version: &CrateVersion,
    ) -> Option<&crates_index::Version> {
        if let Some(index_crate) = self.index.crate_(&crate_version.name) {
            for index_version in index_crate.versions().iter().rev() {
                if index_version.version() == crate_version.version {
                    return Some(index_version);
                }
            }
            warn!(
                "failed to find version {} of the '{}' crate in the source index",
                crate_version.version, crate_version.name
            );
        } else {
            warn!(
                "failed to find the '{}' crate in the source index",
                crate_version.name
            );
        }
        None
    }

    fn get_enabled_features(
        &self,
        index_version: &crates_index::Version,
        index_dep: &crates_index::Dependency,
    ) -> Vec<String> {
        const DEFAULT_FEATURE: &str = "default";

        // The features listed when the dependency is specified in the crate's Cargo.toml file
        // are always enabled.
        let enabled_features = index_dep.features().iter().cloned().collect::<Vec<_>>();
        // Add the implicit default feature if it's not explicity disabled or
        // already in the list.
        if index_dep.has_default_features()
            && enabled_features.iter().position(|f| f == DEFAULT_FEATURE) == None
        {
            enabled_features.push(DEFAULT_FEATURE.to_string());
        }

        // Each feature in the features table specifies an array of additional features
        // or optional dependencies to enable.
        // Only entries in these arrays can enable features of a dependency.
        for (_, feat_list) in index_version.features() {
            for feat in feat_list {
                /*if is_feature_of_dependency(feat, index_dep) {
                    enabled_features.push(feat);
                }*/
            }
        }
        unimplemented!()
    }

    // name
    // dep:name
    // dep/feat
    // dep?/feat

    fn is_feature_of_dependency(feat: &String, index_dep: &crates_index::Dependency) -> bool {
        // There are two ways a feature in a crate's feature table can enable
        // a feature of a dependency:
        //     1. dep/feat
        //     2. dep?/feat
        let parts = feat.split("/").collect::<Vec<_>>();
        if parts.len() == 1 {
            return false;
        }
        let dep_name = parts[0].trim_end_matches("?");
        let dep_feat = parts[1];
    }
}
