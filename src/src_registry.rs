use std::{collections::HashMap, ops::{Deref, DerefMut}};
use crate::common::{CrateVersion, MicrioError, Result};
use log::warn;

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
        Ok(SrcIndex { index, crates_map: HashMap::new() })
    }

    pub fn get_dependencies(&mut self, crate_versions: &Vec<CrateVersion>) -> Result<Vec<CrateVersion>> {
        for crate_version in crate_versions {
            if let Some(index_version) = self.get_crate_version_from_index(crate_version) {
                // TODO: Get all features.
                if let Some(_) = self.crates_map.insert(crate_version.clone(), FeaturesList::new()) {
                    warn!("duplicate crates in the list of crate versions");
                }
                self.update_dependencies(index_version);
            }
        }
        unimplemented!()
    }

    fn update_dependencies(&mut self, version: &crates_index::Version) {

    }

    fn get_crate_version_from_index(&self, crate_version: &CrateVersion) -> Option<&crates_index::Version> {
        if let Some(index_crate) = self.index.crate_(&crate_version.name) {
            for index_version in index_crate.versions().iter().rev() {
                if index_version.version() == crate_version.version {
                    return Some(index_version);
                }
            }
            warn!("failed to find version {} of the '{}' crate in the source index", crate_version.version, crate_version.name);
        } else {
            warn!("failed to find the '{}' crate in the source index",  crate_version.name);
        }
        None
    }

    fn get_all_features(&self, index_version: &crates_index::Version) -> FeaturesList {
        // TODO: Look at optional deps
        let all_features = FeaturesList::new();
        for (feat, feat_list) in index_version.features() {
            all_features.add_feature(feat);

        }
        unimplemented!()
    }

    fn is_feature_optional_dependency()
}