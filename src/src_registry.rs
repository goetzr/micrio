use crate::common::{self, Version};
use cfg_expr::{targets::get_builtin_target_by_triple, targets::TargetInfo, Expression, Predicate};
use crates_index::DependencyKind;
use log::{trace, warn};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    Create,
    CrateNotFound(common::Error),
    ConfigExpression {
        crate_name: String,
        crate_version: String,
        dependency_name: String,
        error: cfg_expr::ParseError,
    },
    SemVerRequirement {
        crate_name: String,
        crate_version: String,
        dependency_name: String,
        error: semver::Error,
    },
    SemVerVersion {
        crate_name: String,
        crate_version: String,
        error: semver::Error,
    },
    FeatureTable {
        crate_name: String,
        crate_version: String,
        error_msg: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create => {
                write!(
                    f,
                    "failed to create source registry: target triple {} not found",
                    common::TARGET_TRIPLE
                )
            }
            Error::CrateNotFound(e) => {
                write!(f, "failed to get crate: {e}")
            }
            Error::ConfigExpression {
                crate_name,
                crate_version,
                dependency_name,
                error,
            } => {
                write!(f, "error parsing target config expression for the {} dependency of {} version {}: {}", dependency_name, crate_name, crate_version, error)
            }
            Error::SemVerRequirement {
                crate_name,
                crate_version,
                dependency_name,
                error,
            } => {
                write!(
                    f,
                    "error parsing version requirement for the {} dependency of {} version {}: {}",
                    dependency_name, crate_name, crate_version, error
                )
            }
            Error::SemVerVersion {
                crate_name,
                crate_version,
                error,
            } => {
                write!(
                    f,
                    "error parsing version string for {} version {}: {}",
                    crate_name, crate_version, error
                )
            }
            Error::FeatureTable {
                crate_name,
                crate_version,
                error_msg,
            } => {
                write!(
                    f,
                    "feature table error with {} version {}: {}",
                    crate_name, crate_version, error_msg
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create => None,
            Error::CrateNotFound(e) => Some(e),
            Error::ConfigExpression { error, .. } => Some(error),
            Error::SemVerRequirement { error, .. } => Some(error),
            Error::SemVerVersion { error, .. } => Some(error),
            Error::FeatureTable { .. } => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

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

pub struct SrcRegistry<'i> {
    index: &'i crates_index::Index,
    target: &'static TargetInfo,
}

impl<'i> SrcRegistry<'i> {
    pub fn new(index: &'i crates_index::Index) -> Result<Self> {
        let target = get_builtin_target_by_triple(common::TARGET_TRIPLE).ok_or(Error::Create)?;
        Ok(SrcRegistry { index, target })
    }

    pub fn get_required_dependencies(
        &self,
        crate_versions: &HashSet<Version>,
    ) -> Result<HashSet<Version>> {
        let mut required_dependencies = HashSet::new();
        for (i, crate_version) in crate_versions.iter().enumerate() {
            println!("Analyzing {:>4} of {:>4}: {} version {}", i+1, crate_versions.len(), crate_version.name(), crate_version.version());
            trace!(
                "{} version {}: (START) getting required dependencies",
                crate_version.name(),
                crate_version.version()
            );
            let features_table = parse_then_grow_features_table(crate_version)?;
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
                let download = self.dependency_enabled_for_target(crate_version, dependency)?;
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
                        if let Some(dep_crate_version) = self.add_dependency(
                                crate_version,
                                dependency,
                                download,
                                &mut required_dependencies)?
                        {
                            enabled_dependencies.push(EnabledDependency::new(
                                dep_crate_version,
                                enabled_features,
                                dependency.has_default_features(),
                            ));
                        }
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
                    if let Some(dep_crate_version) = self.add_dependency(
                            crate_version,
                            dependency,
                            download,
                            &mut required_dependencies)?
                    {
                        enabled_dependencies.push(EnabledDependency::new(
                            dep_crate_version,
                            enabled_features,
                            dependency.has_default_features(),
                        ));
                    }
                }
            }

            for enabled_dependency in enabled_dependencies {
                // NOTE: Can't skip processing a dependency if it's already in the set of required dependencies.
                //       It could have more features enabled from the parent crate this time,
                //       requiring additional dependencies to be downloaded.
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
        let features_table = parse_then_grow_features_table(&crate_version)?;
        let mut enabled_dependencies = Vec::new();
        for dependency in crate_version
            .dependencies()
            .iter()
            .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
        {
            // If the parent crate should not be downloaded, neither should
            // any of its dependencies.
            let download = crate_version.download
                && self.dependency_enabled_for_target(&crate_version, dependency)?;
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
                    if let Some(dep_crate_version) = self.add_dependency(
                            &crate_version,
                            dependency,
                            download,
                            required_dependencies)?
                    {
                        enabled_dependencies.push(EnabledDependency::new(
                            dep_crate_version,
                            enabled_features,
                            dependency.has_default_features(),
                        ));
                    }
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
                if let Some(dep_crate_version) = self.add_dependency(
                        &crate_version,
                        dependency,
                        download,
                        required_dependencies)?
                {
                    enabled_dependencies.push(EnabledDependency::new(
                        dep_crate_version,
                        enabled_features,
                        dependency.has_default_features(),
                    ));
            }
            }
        }

        for enabled_dependency in enabled_dependencies {
            // NOTE: Can't skip processing a dependency if it's already in the set of required dependencies.
            //       It could have more features enabled from the parent crate this time,
            //       requiring additional dependencies to be downloaded.
            self.process_enabled_dependency(enabled_dependency, required_dependencies)?;
        }

        trace!(
            "{} version {}: (END) getting required dependencies",
            crate_version.name(),
            crate_version.version()
        );
        Ok(())
    }

    fn add_dependency(
        &self,
        crate_version: &Version,
        dependency: &crates_index::Dependency,
        download: bool,
        required_dependencies: &mut HashSet<Version>,
    ) -> Result<Option<Version>> {
        let dep_crate = common::get_crate(self.index, dependency.crate_name())
            .map_err(|e| Error::CrateNotFound(e))?;
        if let Some(dep_crate_version) = get_dependency_crate_version(crate_version, dependency, &dep_crate)? {
            let dep_crate_version = Version::new(dep_crate_version.clone()).download(download);
            required_dependencies.insert(dep_crate_version.clone());
            Ok(Some(dep_crate_version))
        } else {
            warn!("{} version {}: compatible crate not found in the source registry for the {} dependency", crate_version.name(), crate_version.version(), dep_crate.name());
            Ok(None)
        }
        
    }

    fn dependency_enabled_for_target(
        &self,
        crate_version: &Version,
        dependency: &crates_index::Dependency,
    ) -> Result<bool> {
        match dependency.target() {
            Some(expr_str) => {
                trace!("{} version {} dependency {}: target expression = {expr_str}",
                    crate_version.name(),
                    crate_version.version(),
                    dependency.name()
                );
                if expr_str.starts_with("cfg") {
                    let expr =
                        Expression::parse(&expr_str).map_err(|e| Error::ConfigExpression {
                            crate_name: crate_version.name().to_string(),
                            crate_version: crate_version.version().to_string(),
                            dependency_name: dependency.name().to_string(),
                            error: e,
                        })?;
                    let result = expr.eval(|pred| match pred {
                        Predicate::Target(tp) => {
                            let passed = tp.matches(self.target);
                            trace!("{} version {} dependency {}: target predicate specified, passed = {passed}",
                                crate_version.name(),
                                crate_version.version(),
                                dependency.name()
                            );
                            Some(passed)
                        }
                        Predicate::TargetFeature(tf) => {
                            // https://doc.rust-lang.org/reference/conditional-compilation.html#target_feature
                            // https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute
                            
                            // Assume our target system has the specified platform architecture feature available.
                            trace!("{} version {} dependency {}: target feature '{tf}' specified, assuming that the target system has this platform architecture feature available",
                                crate_version.name(),
                                crate_version.version(),
                                dependency.name()
                            );
                            Some(true)
                        }
                        Predicate::Flag(flag) => {
                            // A custom flag must be passed to rustc with "--cfg".
                            // Assume we're not doing this.
                            trace!("{} version {} dependency {}: custom flag '{flag}' specified, assuming that this flag is NOT enabled via rustc --cfg",
                                crate_version.name(),
                                crate_version.version(),
                                dependency.name()
                            );
                            Some(false)
                        }
                        Predicate::KeyValue { key, val } => {
                            // A generic key = value predicate that doesn’t match one of the known options.
                            // I've seen crates use "target = <target-triple>", so check for that explicity.
                            // Otherwise, assume the key-value pair is not satisfied.
                            if *key == "target" {
                                Some(*val == self.target.triple.as_str())
                            } else {
                                warn!("{} version {} dependency {}: custom key-value pair '{key} = {val}' specified, assuming this condition is NOT satisfied",
                                    crate_version.name(),
                                    crate_version.version(),
                                    dependency.name()
                                );
                                Some(false)
                            }
                        }
                        _ => {
                            warn!(
                                "{} version {} dependency {}: ignoring unsupported predicate",
                                    crate_version.name(),
                                    crate_version.version(),
                                    dependency.name()
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
                    let enabled = expr_str == self.target.triple.as_str();
                    trace!("{} version {} dependency {}: full target triple specified, enabled = {enabled}",
                        crate_version.name(),
                        crate_version.version(),
                        dependency.name()
                    );
                    Ok(enabled)
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
) -> Result<Option<&'a crates_index::Version>> {
    let version_req = semver::VersionReq::parse(dependency.requirement()).map_err(|e| {
        Error::SemVerRequirement {
            crate_name: crate_version.name().to_string(),
            crate_version: crate_version.version().to_string(),
            dependency_name: dep_crate.name().to_string(),
            error: e,
        }
    })?;
    for dep_crate_version in dep_crate.versions().iter().rev().filter(|c| !c.is_yanked()) {
        let version_str = dep_crate_version.version();
        let version = semver::Version::parse(version_str).map_err(|e| Error::SemVerVersion {
            crate_name: dep_crate.name().to_string(),
            crate_version: version_str.to_string(),
            error: e,
        })?;
        if version_req.matches(&version) {
            return Ok(Some(dep_crate_version));
        }
    }
    Ok(None)
}

enum FeatureTableEntry {
    Feature(String),
    Dependency(String),
    WeakDependencyFeature { dep_name: String, feature: String },
    StrongDependencyFeature { dep_name: String, feature: String },
}

fn parse_then_grow_features_table(
    crate_version: &Version,
) -> Result<HashMap<String, Vec<FeatureTableEntry>>> {
    let mut parsed_features_table = HashMap::new();
    let mut implicit_features = HashSet::<&str>::from_iter(
        crate_version
            .dependencies()
            .iter()
            .filter(|e| {
                e.is_optional()
                    && (e.kind() == DependencyKind::Normal || e.kind() == DependencyKind::Build)
            })
            .map(|d| d.name()),
    );
    for (feature, entries) in crate_version.features() {
        let mut parsed_entries = Vec::new();
        for entry in entries {
            let parsed_entry =
                parse_feature_table_entry(crate_version, feature, entry, &mut implicit_features)?;
            parsed_entries.push(parsed_entry);
        }
        parsed_features_table.insert(feature.clone(), parsed_entries);
    }
    // Add any implicit features from optional dependencies to the features table.
    let mut final_features_table = parsed_features_table;
    for dep_name in implicit_features {
        final_features_table.insert(
            dep_name.to_string(),
            vec![FeatureTableEntry::Dependency(dep_name.to_string())],
        );
    }
    Ok(final_features_table)
}

fn parse_feature_table_entry(
    crate_version: &Version,
    feature: &String,
    entry: &String,
    implicit_features: &mut HashSet<&str>,
) -> Result<FeatureTableEntry> {
    // Possibilities:
    //   feat_name
    //   dep_name (optional dependency)
    //   dep_name/feat_name (optional or required dependency)
    //   dep_name?/feat_name (optional dependency)
    //   dep:dep_name (optional dependency, implicit feature disabled)
    //   dep:dep_name/feat_name (optional dependency, implicit feature disabled)
    //   dep:dep_name?/feat_name (optional dependency, implicit feature disabled)
    let parts = entry.split("/").collect::<Vec<_>>();
    match parts.len() {
        1 => parse_feature_or_dependency_entry(crate_version, feature, entry, implicit_features),
        2 => parse_dependency_feature_entry(
            crate_version,
            feature,
            entry,
            parts[0],
            parts[1],
            implicit_features,
        ),
        _ => Err(Error::FeatureTable {
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
    implicit_features: &mut HashSet<&str>,
) -> Result<FeatureTableEntry> {
    // Possibilities:
    //   feat_name
    //   dep_name (optional dependency)
    //   dep:dep_name (optional dependency, implicit feature disabled)
    if let Some(dep_name) = entry.strip_prefix("dep:") {
        // This must be an optional dependency. It's implicit feature is disabled here.
        if is_optional_dependency_of(dep_name, crate_version) {
            // It's implicit feature is disabled here, so remove it from the set of implicit features to add.
            implicit_features.remove(dep_name);
            Ok(FeatureTableEntry::Dependency(dep_name.to_string()))
        } else {
            Err(Error::FeatureTable {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                error_msg: format!("entry '{entry}' in feature '{feature}': name after 'dep:' not an optional dependency")
            })
        }
    } else {
        // Feature or optional dependency.
        if is_feature_of(entry, crate_version) {
            Ok(FeatureTableEntry::Feature(entry.to_string()))
        } else if is_optional_dependency_of(entry, crate_version) {
            // Implicit feature is NOT disabled here.
            Ok(FeatureTableEntry::Dependency(entry.to_string()))
        } else {
            Err(Error::FeatureTable {
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
    implicit_features: &mut HashSet<&str>,
) -> Result<FeatureTableEntry> {
    // Possibilities:
    //   dep_name/feat_name (optional or required dependency)
    //   dep_name?/feat_name (optional dependency)
    //   dep:dep_name/feat_name (optional dependency, implicit feature disabled)
    //       NOTE: The code below allows a required dependency with this form, even though it shouldn't.
    //             This is b/c it blindly strips off the "dep:" prefix at the beginning.
    //   dep:dep_name?/feat_name (optional dependency, implicit feature disabled)
    if let Some(mut dep_name) = dep_name.strip_prefix("dep:") {
        // This must be an optional dependency. It's implicit feature is disabled here.
        let mut is_weak_dep = false;
        if let Some(stripped) = dep_name.strip_suffix("?") {
            dep_name = stripped;
            is_weak_dep = true;
        }
        if is_optional_dependency_of(dep_name, crate_version) {
            // It's implicit feature is disabled here, so remove it from the set of implicit features to add.
            implicit_features.remove(dep_name);
            if is_weak_dep {
                Ok(FeatureTableEntry::WeakDependencyFeature {
                    dep_name: dep_name.to_string(),
                    feature: dep_feat_name.to_string(),
                })
            } else {
                Ok(FeatureTableEntry::StrongDependencyFeature {
                    dep_name: dep_name.to_string(),
                    feature: dep_feat_name.to_string(),
                })
            }
        } else {
            Err(Error::FeatureTable {
                crate_name: crate_version.name().to_string(),
                crate_version: crate_version.version().to_string(),
                error_msg: format!("entry '{entry}' in feature '{feature}': name after 'dep:' not an optional dependency")
            })
        }
    } else {
        // Required or optional dependency.
        // If optional, its implicit feature is NOT disabled here.
        if let Some(dep_name) = dep_name.strip_suffix("?") {
            // This must be an optional dependency.
            if is_optional_dependency_of(dep_name, crate_version) {
                Ok(FeatureTableEntry::WeakDependencyFeature {
                    dep_name: dep_name.to_string(),
                    feature: dep_feat_name.to_string(),
                })
            } else {
                Err(Error::FeatureTable {
                    crate_name: crate_version.name().to_string(),
                    crate_version: crate_version.version().to_string(),
                    error_msg: format!("entry '{entry}' in feature '{feature}': name before '?' not an optional dependency")
                })
            }
        } else {
            if is_dependency_of(dep_name, crate_version) {
                Ok(FeatureTableEntry::StrongDependencyFeature {
                    dep_name: dep_name.to_string(),
                    feature: dep_feat_name.to_string(),
                })
            } else {
                Err(Error::FeatureTable {
                    crate_name: crate_version.name().to_string(),
                    crate_version: crate_version.version().to_string(),
                    error_msg: format!(
                        "entry '{entry}' in feature '{feature}': name before '/' not a dependency"
                    ),
                })
            }
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
        let entries = match features_table.get(&feature_under_exam) {
            Some(entries) => entries,
            None => {
                if !feature_under_exam.trim().is_empty() {
                    warn!("Feature {feature_under_exam} was not found in {} version {}", crate_version.name(), crate_version.version());
                }
                continue;
            }
        };
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
        let entries = match features_table.get(&feature_under_exam) {
            Some(entries) => entries,
            None => {
                if !feature_under_exam.trim().is_empty() {
                    warn!("Feature {feature_under_exam} was not found in {} version {}", crate_version.name(), crate_version.version());
                }
                continue;
            }
        };
        for entry in entries {
            match entry {
                FeatureTableEntry::Feature(feature) => {
                    if !features_to_examine.contains(feature) {
                        features_to_examine.push_back(feature.clone())
                    }
                }
                FeatureTableEntry::Dependency(dep_name) if dep_name == dependency.name() => {
                    // This happens enough to warrant changing this from a warning to a trace.
                    trace!(
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
    #[test]
    fn test1() {}
}
