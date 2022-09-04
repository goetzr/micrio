use crates_index::Index;
use std::{collections::HashMap, ops::{Deref, DerefMut}};
use crate::common::CrateVersion;

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

pub struct CratesIoIndex<'i> {
    index: &'i Index,
    crates: HashMap<CrateVersion, FeaturesList>,
}

impl<'i> CratesIoIndex<'i> {
    pub fn new(index: &'i Index) -> Self {
        CratesIoIndex { index, crates: HashMap::new() }
    }

    fn doit(&mut self) {
        let cv = CrateVersion::new("clap", "3.5.1");
        self.crates.insert(cv, FeaturesList::new());
        let feat = "somefeature";
        let cv = CrateVersion::new("clap", "3.5.1");
        let features_list = self.crates.get_mut(&cv).unwrap();
        features_list.add_feature(feat);
    }
}

//impl<'i, FL> CratesIoIndex<'i, FL> 