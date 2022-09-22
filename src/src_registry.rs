use crate::common::{CrateId, MicrioError, Result};
use crates_index::DependencyKind;
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

pub struct SrcIndex {
    index: crates_index::Index,
}

struct StoredIndexVec<T> {
    items: Vec<T>,
    index: usize,
}

impl<T: Clone> StoredIndexVec<T> {
    fn new() -> Self {
        StoredIndexVec {
            items: Vec::new(),
            index: 0,
        }
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
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items = iter.into_iter().collect::<Vec<_>>();
        StoredIndexVec { items, index: 0 }
    }
}

struct EnabledDependency {
    crate_version: crates_index::Version,
    enabled_features: Vec<String>,
}

impl EnabledDependency {
    fn new(crate_version: crates_index::Version, enabled_features: Vec<String>) -> Self {
        EnabledDependency {
            crate_version,
            enabled_features,
        }
    }
}

impl SrcIndex {
    pub fn new() -> Result<Self> {
        let index = crates_index::Index::new_cargo_default()?;
        Ok(SrcIndex { index })
    }

    pub fn get_required_dependencies(&self, crate_ids: &Vec<CrateId>) -> Result<HashSet<CrateId>> {
        let mut required_dependencies = HashSet::new();
        for crate_id in crate_ids {
            let crat = self.get_crate(&crate_id.name)?;
            let crate_version = self.get_crate_version(&crat, &crate_id.version)?;
            let features_table = parse_features_table(crate_version)?;
            // Enable all features for top-level crates.
            let enabled_crate_features = features_table
                .iter()
                .map(|(feature, _)| feature.clone())
                .collect::<Vec<_>>();
            let mut enabled_dependencies = Vec::new();
            for dependency in crate_version
                .dependencies()
                .iter()
                .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
            {
                if dependency.is_optional() {
                    let enabled_features = get_enabled_features_for_optional_dependency(
                        crate_version,
                        &features_table,
                        &enabled_crate_features,
                        dependency,
                    );
                    if let Some(enabled_features) = enabled_features {
                        let dep_crate_version = self.add_dependency(
                            crate_version,
                            dependency,
                            &mut required_dependencies,
                        )?;
                        enabled_dependencies
                            .push(EnabledDependency::new(dep_crate_version, enabled_features));
                    }
                } else {
                    let enabled_features = get_enabled_features_for_dependency(
                        crate_version,
                        &features_table,
                        &enabled_crate_features,
                        dependency,
                    );
                    let dep_crate_version =
                        self.add_dependency(crate_version, dependency, &mut required_dependencies)?;
                    enabled_dependencies
                        .push(EnabledDependency::new(dep_crate_version, enabled_features));
                }
            }

            for enabled_dependency in enabled_dependencies {
                self.process_enabled_dependency(enabled_dependency, &mut required_dependencies)?;
            }
        }
        Ok(required_dependencies)
    }

    fn process_enabled_dependency(
        &self,
        enabled_dependency: EnabledDependency,
        required_dependencies: &mut HashSet<CrateId>,
    ) -> Result<()> {
        let crate_version = enabled_dependency.crate_version;
        let enabled_crate_features = enabled_dependency.enabled_features;
        let features_table = parse_features_table(&crate_version)?;
        let mut enabled_dependencies = Vec::new();
        for dependency in crate_version
            .dependencies()
            .iter()
            .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
        {
            if dependency.is_optional() {
                let enabled_features = get_enabled_features_for_optional_dependency(
                    &crate_version,
                    &features_table,
                    &enabled_crate_features,
                    dependency,
                );
                if let Some(enabled_features) = enabled_features {
                    let dep_crate_version =
                        self.add_dependency(&crate_version, dependency, required_dependencies)?;
                    enabled_dependencies
                        .push(EnabledDependency::new(dep_crate_version, enabled_features));
                }
            } else {
                let enabled_features = get_enabled_features_for_dependency(
                    &crate_version,
                    &features_table,
                    &enabled_crate_features,
                    dependency,
                );
                let dep_crate_version =
                    self.add_dependency(&crate_version, dependency, required_dependencies)?;
                enabled_dependencies
                    .push(EnabledDependency::new(dep_crate_version, enabled_features));
            }
        }

        for enabled_dependency in enabled_dependencies {
            self.process_enabled_dependency(enabled_dependency, required_dependencies)?;
        }
        Ok(())
    }

    fn get_crate(&self, name: &str) -> Result<crates_index::Crate> {
        self.index.crate_(name).ok_or(MicrioError::CrateNotFound {
            crate_name: name.to_string(),
        })
    }

    fn get_crate_version<'a>(
        &self,
        crat: &'a crates_index::Crate,
        version: &str,
    ) -> Result<&'a crates_index::Version> {
        crat.versions()
            .iter()
            .rev()
            .find(|v| v.version() == version)
            .ok_or(MicrioError::CrateVersionNotFound {
                crate_name: crat.name().to_string(),
                crate_version: version.to_string(),
            })
    }

    fn get_dependency_crate(
        &self,
        dependency: &crates_index::Dependency,
    ) -> Result<crates_index::Crate> {
        let dep_crate_name = get_dependency_crate_name(dependency);
        self.get_crate(dep_crate_name)
    }

    fn get_dependency_crate_version<'a>(
        &self,
        crate_version: &crates_index::Version,
        dependency: &crates_index::Dependency,
        dep_crate: &'a crates_index::Crate,
    ) -> Result<&'a crates_index::Version> {
        let version_req = semver::VersionReq::parse(dependency.requirement()).map_err(|e| {
            MicrioError::SemVerRequirement {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                dependency_name: dep_crate.name().to_string(),
                error: e,
            }
        })?;
        for dep_crate_version in dep_crate.versions().iter().rev() {
            let version_str = dep_crate_version.version();
            let version =
                semver::Version::parse(version_str).map_err(|e| MicrioError::SemVerVersion {
                    crate_name: dep_crate.name().to_string(),
                    crate_version: version_str.to_string(),
                    error: e,
                })?;
            if version_req.matches(&version) {
                return Ok(dep_crate_version);
            }
        }
        Err(MicrioError::CompatibleCrateNotFound {
            crate_name: crate_version.name().to_string(),
            crate_version: crate_version.version().to_string(),
            dependency_name: dep_crate.name().to_string(),
        })
    }

    fn add_dependency(
        &self,
        crate_version: &crates_index::Version,
        dependency: &crates_index::Dependency,
        required_dependencies: &mut HashSet<CrateId>,
    ) -> Result<crates_index::Version> {
        let dep_crate = self.get_dependency_crate(dependency)?;
        let dep_crate_version =
            self.get_dependency_crate_version(crate_version, dependency, &dep_crate)?;
        let dep_crate_id = CrateId::new(dep_crate.name(), dep_crate_version.version());
        required_dependencies.insert(dep_crate_id);
        Ok(dep_crate_version.clone())
    }
}

enum FeatureTableEntry {
    Feature(String),
    Dependency(String),
    WeakDependencyFeature { dep_name: String, feature: String },
    StrongDependencyFeature { dep_name: String, feature: String },
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
        parsed_features_table.insert(feature.clone(), parsed_entries);
    }
    Ok(parsed_features_table)
}

fn parse_feature_table_entry(
    crate_version: &crates_index::Version,
    feature: &String,
    entry: &String,
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
        }
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
                        });
                    }
                }
            };

            if is_weak {
                if !is_optional_dependency_of(dep_name, crate_version) {
                    return Err(MicrioError::FeatureTable {
                        crate_name: crate_version.name().to_string(),
                        crate_version: crate_version.version().to_string(),
                        error_msg: format!("entry '{entry}' in feature '{feature}': name before '/' not an optional dependency")
                    });
                }
                Ok(FeatureTableEntry::WeakDependencyFeature {
                    dep_name: dep_name.to_string(),
                    feature: feat_name.to_string(),
                })
            } else {
                if !is_dependency_of(dep_name, crate_version) {
                    return Err(MicrioError::FeatureTable {
                        crate_name: crate_version.name().to_string(),
                        crate_version: crate_version.version().to_string(),
                        error_msg: format!("entry '{entry}' in feature '{feature}': name before '/' not a dependency")
                    });
                }
                Ok(FeatureTableEntry::StrongDependencyFeature {
                    dep_name: dep_name.to_string(),
                    feature: feat_name.to_string(),
                })
            }
        }
        _ => Err(MicrioError::FeatureTable {
            crate_name: crate_version.name().to_string(),
            crate_version: crate_version.version().to_string(),
            error_msg: format!("entry '{entry}' in feature '{feature}': multiple '/' separators"),
        }),
    }
}

fn is_feature_of(name: &str, crate_version: &crates_index::Version) -> bool {
    crate_version.features().contains_key(name)
}

fn is_optional_dependency_of(name: &str, crate_version: &crates_index::Version) -> bool {
    crate_version
        .dependencies()
        .iter()
        .filter(|dep| dep.is_optional())
        .position(|dep| dep.name() == name)
        .is_some()
}

fn is_dependency_of(name: &str, crate_version: &crates_index::Version) -> bool {
    crate_version
        .dependencies()
        .iter()
        .position(|dep| dep.name() == name)
        .is_some()
}

fn get_dependency_crate_name(dependency: &crates_index::Dependency) -> &str {
    if let Some(name) = dependency.package() {
        name
    } else {
        dependency.name()
    }
}

fn get_enabled_features_for_optional_dependency(
    crate_version: &crates_index::Version,
    features_table: &HashMap<String, Vec<FeatureTableEntry>>,
    enabled_crate_features: &Vec<String>,
    dependency: &crates_index::Dependency,
) -> Option<Vec<String>> {
    unimplemented!()
}

fn get_enabled_features_for_dependency(
    crate_version: &crates_index::Version,
    features_table: &HashMap<String, Vec<FeatureTableEntry>>,
    enabled_crate_features: &Vec<String>,
    dependency: &crates_index::Dependency,
) -> Vec<String> {
    let enabled_features = Vec::new();
    let features_to_examine = StoredIndexVec::from_iter(enabled_crate_features.iter().cloned());
    Ok(enabled_features)
}
