use crates_index;
use semver;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

pub const TARGET_TRIPLE: &'static str = "x86_64-pc-windows-msvc";
pub const DEFAULT_FEATURE: &'static str = "default";

#[derive(Clone)]
pub struct Version(pub crates_index::Version);

impl Version {
    pub fn name(&self) -> &str {
        self.0.name()
    }

    pub fn version(&self) -> &str {
        self.0.version()
    }

    pub fn dependencies(&self) -> &[crates_index::Dependency] {
        self.0.dependencies()
    }

    pub fn features(&self) -> &HashMap<String, Vec<String>> {
        self.0.features()
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.version() == other.version()
    }
}
impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name().hash(state);
        self.0.version().hash(state);
    }
}

#[derive(Debug)]
pub enum MicrioError {
    SrcRegistry(crates_index::Error),
    TargetNotFound,
    ConfigExpression {
        crate_name: String,
        crate_version: String,
        dependency_name: String,
        error: cfg_expr::ParseError,
    },
    CrateNotFound {
        crate_name: String,
    },
    CrateVersionNotFound {
        crate_name: String,
        crate_version: String,
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
    CompatibleCrateNotFound {
        crate_name: String,
        crate_version: String,
        dependency_name: String,
    },
    FeatureTable {
        crate_name: String,
        crate_version: String,
        error_msg: String,
    },
    FeatureNotFound {
        crate_name: String,
        crate_version: String,
        feature_name: String,
    },
}

impl Display for MicrioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MicrioError::SrcRegistry(e) => write!(f, "source registry error: {}", e),
            MicrioError::TargetNotFound => write!(f, "target triple {} not found", TARGET_TRIPLE),
            MicrioError::ConfigExpression {
                crate_name,
                crate_version,
                dependency_name,
                error,
            } => {
                write!(f, "error parsing target config expression for the {} dependency of {} version {}: {}", dependency_name, crate_name, crate_version, error)
            }
            MicrioError::CrateNotFound { crate_name } => {
                write!(f, "{} not found in the source registry", crate_name)
            }
            MicrioError::CrateVersionNotFound { crate_name, crate_version } => {
                write!(f, "{} version {} not found in the source registry", crate_name, crate_version)
            }
            MicrioError::SemVerRequirement {
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
            MicrioError::SemVerVersion {
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
            MicrioError::CompatibleCrateNotFound {
                crate_name,
                crate_version,
                dependency_name,
            } => {
                write!(f, "compatible crate not found in the source registry for the {} dependency of {} version {}", dependency_name, crate_name, crate_version)
            }
            MicrioError::FeatureTable {
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
            MicrioError::FeatureNotFound {
                crate_name,
                crate_version,
                feature_name,
            } => {
                write!(
                    f,
                    "feature {} not found in version {} of {}",
                    feature_name, crate_name, crate_version
                )
            }
        }
    }
}

impl std::error::Error for MicrioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MicrioError::SrcRegistry(e) => Some(e),
            MicrioError::TargetNotFound => None,
            MicrioError::ConfigExpression { error, .. } => Some(error),
            MicrioError::CrateNotFound { .. } => None,
            MicrioError::CrateVersionNotFound { .. } => None,
            MicrioError::SemVerRequirement { error, .. } => Some(error),
            MicrioError::SemVerVersion { error, .. } => Some(error),
            MicrioError::CompatibleCrateNotFound { .. } => None,
            MicrioError::FeatureTable { .. } => None,
            MicrioError::FeatureNotFound { .. } => None,
        }
    }
}

impl From<crates_index::Error> for MicrioError {
    fn from(error: crates_index::Error) -> Self {
        MicrioError::SrcRegistry(error)
    }
}

pub type Result<T> = std::result::Result<T, MicrioError>;

pub fn get_crate(index: &crates_index::Index, name: &str) -> Result<crates_index::Crate> {
    index.crate_(name).ok_or(MicrioError::CrateNotFound {
        crate_name: name.to_string(),
    })
}