use crate::common::{self, MicrioError, Result, Version};
use cfg_expr::{targets::get_builtin_target_by_triple, targets::TargetInfo, Expression, Predicate};
use crates_index::DependencyKind;
use log::{trace, warn};
use std::{collections::{HashMap, HashSet, VecDeque},
};

struct EnabledDependency {
    crate_version: Version,
    enabled_features: Vec<String>,
    has_default_features: bool,
}

impl EnabledDependency {
    fn new(
        crate_version: Version,
        enabled_features: Vec<String>,
        has_default_features: bool,
    ) -> Self {
        EnabledDependency {
            crate_version,
            enabled_features,
            has_default_features,
        }
    }
}

pub struct SrcIndex<'i> {
    index: &'i crates_index::Index,
    target: &'static TargetInfo,
}

impl<'i> SrcIndex<'i> {
    pub fn new(index: &'i crates_index::Index) -> Result<Self> {
        let target = get_builtin_target_by_triple(common::TARGET_TRIPLE)
            .ok_or(MicrioError::TargetNotFound)?;
        Ok(SrcIndex { index, target })
    }

    pub fn get_required_dependencies(
        &self,
        crate_versions: &HashSet<Version>,
    ) -> Result<HashSet<Version>> {
        let mut required_dependencies = HashSet::new();
        for crate_version in crate_versions {
            trace!(
                "{} version {}: (START) getting required dependencies",
                crate_version.name(),
                crate_version.version()
            );
            let features_table = parse_features_table(crate_version)?;
            // Enable all features for each top-level crate to ensure all potential features
            // for each of its dependencies are enabled.
            let enabled_crate_features = features_table
                .iter()
                .map(|(feature, _)| feature.clone())
                .collect::<Vec<_>>();
            trace!(
                "{} version {}: enabled features: {}",
                crate_version.name(),
                crate_version.version(),
                enabled_crate_features.join(",")
            );
            let mut enabled_dependencies = Vec::new();
            for dependency in crate_version
                .dependencies()
                .iter()
                .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
            {
                if !self.dependency_enabled_for_target(crate_version, dependency)? {
                    trace!(
                        "{} version {}: {} dependency not enabled for this target",
                        crate_version.name(),
                        crate_version.version(),
                        dependency.name()
                    );
                    continue;
                }
                // NOTE: All optional dependencies of top-level crates are force-enabled.
                if dependency.is_optional() {
                    let enabled_features = get_enabled_features_for_optional_dependency(
                        crate_version,
                        &features_table,
                        &enabled_crate_features,
                        dependency,
                        true, // Force-enable all optional dependencies for top-level crates.
                    )?;
                    if let Some(enabled_features) = enabled_features {
                        // Optional dependency is enabled.
                        // This should always be true for top-level crates.
                        trace!(
                            "{} version {}: optional dependency {} features: {}",
                            crate_version.name(),
                            crate_version.version(),
                            dependency.name(),
                            &enabled_features.join(",")
                        );
                        let dep_crate_version = self.add_dependency(
                            crate_version,
                            dependency,
                            &mut required_dependencies,
                        )?;
                        enabled_dependencies.push(EnabledDependency::new(
                            dep_crate_version,
                            enabled_features,
                            dependency.has_default_features(),
                        ));
                    } else {
                        // Should never gete here b/c we force-enable all optional dependencies of top-level crates.
                        warn!(
                            "{} version {}: {} optional dependency not enabled for top-level crate",
                            crate_version.name(),
                            crate_version.version(),
                            dependency.name()
                        );
                    }
                } else {
                    let enabled_features = get_enabled_features_for_dependency(
                        crate_version,
                        &features_table,
                        &enabled_crate_features,
                        dependency,
                    )?;
                    trace!(
                        "{} version {}: required dependency {} features: {}",
                        crate_version.name(),
                        crate_version.version(),
                        dependency.name(),
                        &enabled_features.join(",")
                    );
                    let dep_crate_version =
                        self.add_dependency(crate_version, dependency, &mut required_dependencies)?;
                    enabled_dependencies.push(EnabledDependency::new(
                        dep_crate_version,
                        enabled_features,
                        dependency.has_default_features(),
                    ));
                }
            }

            for enabled_dependency in enabled_dependencies {
                self.process_enabled_dependency(enabled_dependency, &mut required_dependencies)?;
            }

            trace!(
                "{} version {}: (END) getting required dependencies",
                crate_version.name(),
                crate_version.version()
            );
        }
        Ok(required_dependencies)
    }

    fn process_enabled_dependency(
        &self,
        enabled_dependency: EnabledDependency,
        required_dependencies: &mut HashSet<Version>,
    ) -> Result<()> {
        let crate_version = enabled_dependency.crate_version;
        trace!(
            "{} version {}: (START) getting required dependencies",
            crate_version.name(),
            crate_version.version()
        );
        let mut enabled_crate_features = enabled_dependency.enabled_features;
        if enabled_dependency.has_default_features {
            if crate_version
                .features()
                .contains_key(common::DEFAULT_FEATURE)
                && !enabled_crate_features
                    .iter()
                    .any(|f| f == common::DEFAULT_FEATURE)
            {
                enabled_crate_features.push(common::DEFAULT_FEATURE.to_string());
            }
        } else {
            let idx_default = enabled_crate_features
                .iter()
                .position(|f| f == common::DEFAULT_FEATURE);
            if let Some(idx) = idx_default {
                enabled_crate_features.swap_remove(idx);
            }
        }
        trace!(
            "{} version {}: enabled features: {}",
            crate_version.name(),
            crate_version.version(),
            &enabled_crate_features.join(",")
        );
        let features_table = parse_features_table(&crate_version)?;
        let mut enabled_dependencies = Vec::new();
        for dependency in crate_version
            .dependencies()
            .iter()
            .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
        {
            if !self.dependency_enabled_for_target(&crate_version, dependency)? {
                trace!(
                    "{} version {}: {} dependency not enabled for this target",
                    crate_version.name(),
                    crate_version.version(),
                    dependency.name()
                );
                continue;
            }
            if dependency.is_optional() {
                let enabled_features = get_enabled_features_for_optional_dependency(
                    &crate_version,
                    &features_table,
                    &enabled_crate_features,
                    dependency,
                    false, // Don't force-enable the dependency.
                )?;
                if let Some(enabled_features) = enabled_features {
                    // Optional dependency is enabled.
                    trace!(
                        "{} version {}: optional dependency {} features: {}",
                        crate_version.name(),
                        crate_version.version(),
                        dependency.name(),
                        &enabled_features.join(",")
                    );
                    let dep_crate_version =
                        self.add_dependency(&crate_version, dependency, required_dependencies)?;
                    enabled_dependencies.push(EnabledDependency::new(
                        dep_crate_version,
                        enabled_features,
                        dependency.has_default_features(),
                    ));
                }
            } else {
                let enabled_features = get_enabled_features_for_dependency(
                    &crate_version,
                    &features_table,
                    &enabled_crate_features,
                    dependency,
                )?;
                trace!(
                    "{} version {}: required dependency {} features: {}",
                    crate_version.name(),
                    crate_version.version(),
                    dependency.name(),
                    &enabled_features.join(",")
                );
                let dep_crate_version =
                    self.add_dependency(&crate_version, dependency, required_dependencies)?;
                enabled_dependencies.push(EnabledDependency::new(
                    dep_crate_version,
                    enabled_features,
                    dependency.has_default_features(),
                ));
            }
        }

        for enabled_dependency in enabled_dependencies {
            self.process_enabled_dependency(enabled_dependency, required_dependencies)?;
        }

        trace!(
            "{} version {}: (END) getting required dependencies",
            crate_version.name(),
            crate_version.version()
        );
        Ok(())
    }

    fn get_crate(&self, name: &str) -> Result<crates_index::Crate> {
        common::get_crate(self.index, name)
    }

    fn add_dependency(
        &self,
        crate_version: &Version,
        dependency: &crates_index::Dependency,
        required_dependencies: &mut HashSet<Version>,
    ) -> Result<Version> {
        let dep_crate = self.get_crate(dependency.crate_name())?;
        let dep_crate_version =
            get_dependency_crate_version(crate_version, dependency, &dep_crate)?;
        let dep_crate_version = Version(dep_crate_version.clone());
        required_dependencies.insert(dep_crate_version.clone());
        Ok(dep_crate_version)
    }

    fn dependency_enabled_for_target(
        &self,
        crate_version: &Version,
        dependency: &crates_index::Dependency,
    ) -> Result<bool> {
        match dependency.target() {
            Some(expr_str) => {
                trace!(
                    "{} version {} dependency {} target expression = {}",
                    crate_version.name(),
                    crate_version.version(),
                    dependency.name(),
                    expr_str
                );
                if expr_str.starts_with("cfg") {
                    trace!("config expression");
                    let expr =
                        Expression::parse(&expr_str).map_err(|e| MicrioError::ConfigExpression {
                            crate_name: crate_version.name().to_string(),
                            crate_version: crate_version.version().to_string(),
                            dependency_name: dependency.name().to_string(),
                            error: e,
                        })?;
                    let result = expr.eval(|pred| match pred {
                        Predicate::Target(tp) => Some(tp.matches(self.target)),
                        _ => {
                            warn!(
                                "{} version {}: target expression {} does not contain \
                                a target predicate",
                                crate_version.name(),
                                crate_version.version(),
                                expr_str
                            );
                            None
                        }
                    });
                    match result {
                        Some(result) => Ok(result),
                        None => Ok(true),
                    }
                } else {
                    // Full target triple specified.
                    trace!("full target triple");
                    Ok(expr_str == self.target.triple.as_str())
                }
            }
            None => Ok(true),
        }
    }
}

fn get_dependency_crate_version<'a>(
    crate_version: &Version,
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
    for dep_crate_version in dep_crate.versions().iter().rev().filter(|c| !c.is_yanked()) {
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

enum FeatureTableEntry {
    Feature(String),
    Dependency(String),
    WeakDependencyFeature { dep_name: String, feature: String },
    StrongDependencyFeature { dep_name: String, feature: String },
}

fn parse_features_table(
    crate_version: &Version,
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
    crate_version: &Version,
    feature: &String,
    entry: &String,
) -> Result<FeatureTableEntry> {
    // Possibilities:
    //   feat_name
    //   dep_name (optional dependency)
    //   dep_name/feat_name (optional or required dependency)
    //   dep_name?/feat_name (optional dependency)
    //   dep:dep_name (optional dependency)
    //   dep:dep_name/feat_name (optional dependency)
    //   dep:dep_name?/feat_name (optional dependency)
    let parts = entry.split("/").collect::<Vec<_>>();
    match parts.len() {
        1 => parse_feature_or_dependency_entry(crate_version, feature, entry),
        2 => parse_dependency_feature_entry(crate_version, feature, entry, parts[0], parts[1]),
        _ => Err(MicrioError::FeatureTable {
            crate_name: crate_version.name().to_string(),
            crate_version: crate_version.version().to_string(),
            error_msg: format!("entry '{entry}' in feature '{feature}': invalid format"),
        }),
    }
}

fn parse_feature_or_dependency_entry(
    crate_version: &Version,
    feature: &String,
    entry: &String,
) -> Result<FeatureTableEntry> {
    // Possibilities:
    //   feat_name
    //   dep_name (optional dependency)
    //   dep:dep_name (optional dependency)
    if let Some(dep_name) = entry.strip_prefix("dep:") {
        if is_optional_dependency_of(dep_name, crate_version) {
            Ok(FeatureTableEntry::Dependency(dep_name.to_string()))
        } else {
            Err(MicrioError::FeatureTable {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                error_msg: format!("entry '{entry}' in feature '{feature}': name after 'dep:' not an optional dependency")
            })
        }
    } else {
        if is_feature_of(entry, crate_version) {
            Ok(FeatureTableEntry::Feature(entry.to_string()))
        } else if is_optional_dependency_of(entry, crate_version) {
            Ok(FeatureTableEntry::Dependency(entry.to_string()))
        } else {
            Err(MicrioError::FeatureTable {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                error_msg: format!("entry '{entry}' in feature '{feature}': '{entry}' not a feature or an optional dependency")
            })
        }
    }
}

fn parse_dependency_feature_entry(
    crate_version: &Version,
    feature: &String,
    entry: &String,
    dep_name: &str,
    dep_feat_name: &str,
) -> Result<FeatureTableEntry> {
    // Possibilities:
    //   dep_name/feat_name (optional or required dependency)
    //   dep_name?/feat_name (optional dependency)
    //   dep:dep_name/feat_name (optional dependency)
    //       NOTE: The code below allows a required dependency with this form, even though it shouldn't.
    //             This is b/c it blindly strips off the "dep:" prefix at the beginning.
    //   dep:dep_name?/feat_name (optional dependency)
    let mut dep_name = dep_name;
    if let Some(stripped) = dep_name.strip_prefix("dep:") {
        dep_name = stripped;
    }

    if let Some(dep_name) = dep_name.strip_suffix("?") {
        if is_optional_dependency_of(dep_name, crate_version) {
            Ok(FeatureTableEntry::WeakDependencyFeature {
                dep_name: dep_name.to_string(),
                feature: dep_feat_name.to_string(),
            })
        } else {
            Err(MicrioError::FeatureTable {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                error_msg: format!("entry '{entry}' in feature '{feature}': name before '/' not an optional dependency")
            })
        }
    } else {
        if is_dependency_of(dep_name, crate_version) {
            Ok(FeatureTableEntry::StrongDependencyFeature {
                dep_name: dep_name.to_string(),
                feature: dep_feat_name.to_string(),
            })
        } else {
            Err(MicrioError::FeatureTable {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                error_msg: format!(
                    "entry '{entry}' in feature '{feature}': name before '/' not a dependency"
                ),
            })
        }
    }
}

fn is_feature_of(name: &str, crate_version: &Version) -> bool {
    crate_version.features().contains_key(name)
}

fn is_optional_dependency_of(name: &str, crate_version: &Version) -> bool {
    crate_version
        .dependencies()
        .iter()
        .filter(|dep| dep.is_optional())
        .any(|dep| dep.name() == name)
}

fn is_dependency_of(name: &str, crate_version: &Version) -> bool {
    crate_version
        .dependencies()
        .iter()
        .any(|dep| dep.name() == name)
}

fn get_enabled_features_for_optional_dependency(
    crate_version: &Version,
    features_table: &HashMap<String, Vec<FeatureTableEntry>>,
    enabled_crate_features: &Vec<String>,
    dependency: &crates_index::Dependency,
    force_enable: bool,
) -> Result<Option<Vec<String>>> {
    let mut enabled_features = dependency.features().iter().cloned().collect::<Vec<_>>();
    let mut weakly_enabled_features = Vec::new();
    let mut features_to_examine = VecDeque::from_iter(enabled_crate_features.iter().cloned());
    let mut dependency_enabled = force_enable;

    while let Some(feature_under_exam) = features_to_examine.pop_front() {
        let entries =
            features_table
                .get(&feature_under_exam)
                .ok_or(MicrioError::FeatureNotFound {
                    crate_name: crate_version.name().to_string(),
                    crate_version: crate_version.version().to_string(),
                    feature_name: feature_under_exam,
                })?;
        for entry in entries {
            match entry {
                FeatureTableEntry::Feature(feature) => {
                    if !features_to_examine.contains(feature) {
                        features_to_examine.push_back(feature.clone())
                    }
                }
                FeatureTableEntry::Dependency(dep_name) if dep_name == dependency.name() => {
                    dependency_enabled = true
                }
                FeatureTableEntry::StrongDependencyFeature { dep_name, feature }
                    if dep_name == dependency.name() =>
                {
                    dependency_enabled = true;
                    if !enabled_features.contains(feature) {
                        enabled_features.push(feature.clone())
                    }
                }
                FeatureTableEntry::WeakDependencyFeature { dep_name, feature }
                    if dep_name == dependency.name() =>
                {
                    if !weakly_enabled_features.contains(feature) {
                        weakly_enabled_features.push(feature.clone());
                    }
                }
                _ => (),
            }
        }
    }

    if dependency_enabled {
        for weak_feature in weakly_enabled_features {
            if !enabled_features.contains(&weak_feature) {
                enabled_features.push(weak_feature);
            }
        }
        Ok(Some(enabled_features))
    } else {
        Ok(None)
    }
}

fn get_enabled_features_for_dependency(
    crate_version: &Version,
    features_table: &HashMap<String, Vec<FeatureTableEntry>>,
    enabled_crate_features: &Vec<String>,
    dependency: &crates_index::Dependency,
) -> Result<Vec<String>> {
    let mut enabled_features = dependency.features().iter().cloned().collect::<Vec<_>>();
    let mut features_to_examine = VecDeque::from_iter(enabled_crate_features.iter().cloned());

    while let Some(feature_under_exam) = features_to_examine.pop_front() {
        let entries =
            features_table
                .get(&feature_under_exam)
                .ok_or(MicrioError::FeatureNotFound {
                    crate_name: crate_version.name().to_string(),
                    crate_version: crate_version.version().to_string(),
                    feature_name: feature_under_exam,
                })?;
        for entry in entries {
            match entry {
                FeatureTableEntry::Feature(feature) => {
                    if !features_to_examine.contains(feature) {
                        features_to_examine.push_back(feature.clone())
                    }
                }
                FeatureTableEntry::Dependency(dep_name) if dep_name == dependency.name() => {
                    warn!(
                        "{} version {}: required dependency {dep_name} found in feature table",
                        crate_version.name(),
                        crate_version.version()
                    );
                }
                FeatureTableEntry::StrongDependencyFeature { dep_name, feature }
                    if dep_name == dependency.name() =>
                {
                    if !enabled_features.contains(feature) {
                        enabled_features.push(feature.clone())
                    }
                }
                FeatureTableEntry::WeakDependencyFeature { dep_name, feature }
                    if dep_name == dependency.name() =>
                {
                    warn!("{} version {}: weak dependency feature {feature} found in feature table for required dependency {}", crate_version.name(), crate_version.version(), dependency.name());
                }
                _ => (),
            }
        }
    }

    Ok(enabled_features)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test1() {}
}
