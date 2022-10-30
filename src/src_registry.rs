use crate::common::{self, Version};
use crates_index::DependencyKind;
use log::warn;
use semver::VersionReq;
use std::collections::HashSet;
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    CrateNotFound(common::Error),
    SemVerRequirement {
        crate_name: String,
        dependency_name: String,
        error: semver::Error,
    },
    SemVerVersion {
        crate_name: String,
        crate_version: String,
        error: semver::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CrateNotFound(e) => {
                write!(f, "failed to get crate: {e}")
            }
            Error::SemVerRequirement {
                crate_name,
                dependency_name,
                error,
            } => {
                write!(
                    f,
                    "error parsing version requirement for the {} dependency of {}: {}",
                    dependency_name, crate_name, error
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
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::CrateNotFound(e) => Some(e),
            Error::SemVerRequirement { error, .. } => Some(error),
            Error::SemVerVersion { error, .. } => Some(error),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct SrcRegistry<'i> {
    index: &'i crates_index::Index,
    dependencies: HashSet<Version>,
    cur_crate_name: String,
}

impl<'i> SrcRegistry<'i> {
    pub fn new(index: &'i crates_index::Index) -> Self {
        SrcRegistry {
            index,
            dependencies: HashSet::new(),
            cur_crate_name: String::from(""),
        }
    }

    pub fn get_dependencies(&mut self, crate_versions: &HashSet<Version>) -> Result<HashSet<Version>> {
        for (i, crate_version) in crate_versions.iter().enumerate() {
            println!(
                "Analyzing {:>4} of {}: {} version {}",
                i + 1,
                crate_versions.len(),
                crate_version.name(),
                crate_version.version()
            );
            // Cache the name of the current crate for use in error messages.
            self.cur_crate_name = crate_version.name().to_string();
            let mut deps_to_analyze = Vec::new();
            for dependency in crate_version
                .dependencies()
                .iter()
                .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
            {
                let dep_version = match self.get_compatible_version(dependency)? {
                    Some(version) => version,
                    None => {
                        warn!(
                            "{} version {}: compatible version for {} dependency not found",
                            crate_version.name(),
                            crate_version.version(),
                            dependency.name()
                        );
                        continue;
                    }
                };
                if self.dependencies.insert(dep_version.clone()) {
                    deps_to_analyze.push(dep_version);
                }
            }

            for dep_version in deps_to_analyze {
                println!(
                    "\tAnalyzing dependency {} version {}",
                    dep_version.name(),
                    dep_version.version()
                );
                self.process_dependency(dep_version)?;
            }
        }
        Ok(self.dependencies.clone())
    }

    fn process_dependency(&mut self, dep_version: common::Version) -> Result<()> {
        let crate_version = dep_version;
        // Cache the name of the current crate for use in error messages.
        self.cur_crate_name = crate_version.name().to_string();
        let mut deps_to_analyze = Vec::new();
        for dependency in crate_version
            .dependencies()
            .iter()
            .filter(|d| d.kind() == DependencyKind::Normal || d.kind() == DependencyKind::Build)
        {
            let dep_version = match self.get_compatible_version(dependency)? {
                Some(version) => version,
                None => {
                    warn!(
                        "{} version {}: compatible version for {} dependency not found",
                        crate_version.name(),
                        crate_version.version(),
                        dependency.name()
                    );
                    continue;
                }
            };
            if self.dependencies.insert(dep_version.clone()) {
                deps_to_analyze.push(dep_version);
            }
        }

        for dep_version in deps_to_analyze {
            println!(
                "\tAnalyzing dependency {} version {}",
                dep_version.name(),
                dep_version.version()
            );
            self.process_dependency(dep_version)?;
        }

        Ok(())
    }

    fn get_compatible_version(
        &self,
        dependency: &crates_index::Dependency,
    ) -> Result<Option<common::Version>> {
        let version_req =
            VersionReq::parse(dependency.requirement()).map_err(|e| Error::SemVerRequirement {
                crate_name: self.cur_crate_name.clone(),
                dependency_name: dependency.name().to_string(),
                error: e,
            })?;
        let crat = common::get_crate(self.index, dependency.crate_name())
            .map_err(|e| Error::CrateNotFound(e))?;
        for crate_version in crat.versions().iter().rev().filter(|c| !c.is_yanked()) {
            let version = semver::Version::parse(crate_version.version()).map_err(|e| {
                Error::SemVerVersion {
                    crate_name: crat.name().to_string(),
                    crate_version: crate_version.version().to_string(),
                    error: e,
                }
            })?;
            if version_req.matches(&version) {
                return Ok(Some(common::Version(crate_version.clone())));
            }
        }
        Ok(None)
    }
}
