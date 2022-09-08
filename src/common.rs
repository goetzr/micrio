use crates_index;
use std::fmt;
use std::fmt::Display;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CrateVersion {
    pub name: String,
    pub version: String,
}

impl CrateVersion {
    pub fn new(name: &str, version: &str) -> Self {
        CrateVersion {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum MicrioError {
    SrcRegistryError(crates_index::Error),
}

impl Display for MicrioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MicrioError::SrcRegistryError(e) => write!(f, "source registry error: {}", e),
        }
    }
}

impl std::error::Error for MicrioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MicrioError::SrcRegistryError(e) => e,
        }
    }
}

impl From<crates_index::Error> for MicrioError {
    fn from(error: crates_index::Error) -> Self {
        MicrioError::SrcRegistryError(error)
    }
}

pub type Result<T> = std::result::Result<T, MicrioError>;
