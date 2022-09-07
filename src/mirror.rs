use crates_index::Index;
use std::{collections::HashMap, ops::{Deref, DerefMut}};

#[derive(PartialEq, Eq, Hash)]
struct CrateVersion<'i> {
    name: &'i str,
    version: &'i str,
}

impl<'i> CrateVersion<'i> {
    pub fn new(name: &'i str, version: &'i str) -> Self {
        CrateVersion { name, version }
    }
}

struct FeaturesList<'i> {
    list: Vec<&'i str>,
}

impl<'i> FeaturesList<'i> {
    fn new() -> Self {
        FeaturesList { list: Vec::new() }
    }

    fn add_feature(&mut self, feature: &'i str) {
        if self.list.iter().position(|feat| feat == feature).is_none() {
            self.list.push(feature);
        }
    }
}

impl<'i> Deref for FeaturesList<'i> {
    type Target = Vec<&'i str>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl DerefMut for FeaturesList<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

pub struct CratesIoIndex<'i> {
    // NOTE: Lifetime of strings is tied to lifetime of Version, which is tied to lifetime of Crate, NOT Index!
    index: &'i Index,
    crates: HashMap<CrateVersion<'i>, FeaturesList<'i>>,
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