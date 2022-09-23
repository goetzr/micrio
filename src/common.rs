use crates_index;
use std::fmt;
use std::fmt::Display;
use semver;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CrateId {
    pub name: String,
    pub version: String,
}

impl CrateId {
    pub fn new(name: &str, version: &str) -> Self {
        CrateId {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum MicrioError {
    SrcRegistry(crates_index::Error),
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
    }
}

impl Display for MicrioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MicrioError::SrcRegistry(e) => write!(f, "source registry error: {}", e),
            MicrioError::CrateNotFound { crate_name } => {
                write!(f, "{} not found in the source registry", crate_name)
            },
            MicrioError::CrateVersionNotFound { crate_name, crate_version } => {
                write!(f, "version {} of {} not found in the source registry", crate_name, crate_version)
            },
            MicrioError::SemVerRequirement { crate_name, crate_version, dependency_name, error } => {
                write!(f, "error parsing version requirement for the {} dependency of {} version {}: {}", dependency_name, crate_name, crate_version, error)
            },
            MicrioError::SemVerVersion { crate_name, crate_version, error } => {
                write!(f, "error parsing version string for {} version {}: {}", crate_name, crate_version, error)
            },
            MicrioError::CompatibleCrateNotFound { crate_name, crate_version, dependency_name } => {
                write!(f, "compatible crate not found in the source registry for the {} dependency of {} version {}", dependency_name, crate_name, crate_version)
            }
            MicrioError::FeatureTable { crate_name, crate_version, error_msg } => {
                write!(f, "feature table error with {} version {}: {}", crate_name, crate_version, error_msg)
            },
            MicrioError::FeatureNotFound { crate_name, crate_version, feature_name } => {
                write!(f, "feature {} not found in version {} of {}", feature_name, crate_name, crate_version)
            },
        }
    }
}

impl std::error::Error for MicrioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MicrioError::SrcRegistry(e) => Some(e),
            MicrioError::CrateNotFound { crate_name } => None,
            MicrioError::CrateVersionNotFound { crate_name, crate_version } => None,
            MicrioError::SemVerRequirement { crate_name, crate_version, dependency_name, error } => Some(error),
            MicrioError::SemVerVersion { crate_name, crate_version, error } => Some(error),
            MicrioError::CompatibleCrateNotFound { crate_name, crate_version, dependency_name } => None,
            MicrioError::FeatureTable { crate_name, crate_version, error_msg } => None,
            MicrioError::FeatureNotFound { crate_name, crate_version, feature_name } => None,
        }
    }
}

impl From<crates_index::Error> for MicrioError {
    fn from(error: crates_index::Error) -> Self {
        MicrioError::SrcRegistry(error)
    }
}

pub type Result<T> = std::result::Result<T, MicrioError>;
