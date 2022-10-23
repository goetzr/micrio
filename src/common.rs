use crates_index;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

pub const TARGET_TRIPLE: &'static str = "x86_64-pc-windows-msvc";
pub const DEFAULT_FEATURE: &'static str = "default";

#[derive(Debug)]
pub enum Error {
    CrateNotFound {
        crate_name: String,
    },
    SerializeVersion(serde_json::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CrateNotFound { crate_name } => {
                write!(f, "{} not found in the source registry", crate_name)
            }
            Error::SerializeVersion(e) => {
                write!(f, "failed to serialize to JSON: {e}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self {
            Error::CrateNotFound { .. } => None,
            Error::SerializeVersion(e) => Some(e),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Version {
    pub version: crates_index::Version,
    pub download: bool,
}

impl Version {
    pub fn new(version: crates_index::Version) -> Self {
        Version { version, download: false }
    }

    pub fn download(mut self, flag: bool) -> Self {
        self.download = flag;
        self
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.version).map_err(|e| Error::SerializeVersion(e))
    }

    pub fn name(&self) -> &str {
        self.version.name()
    }

    pub fn version(&self) -> &str {
        self.version.version()
    }

    pub fn dependencies(&self) -> &[crates_index::Dependency] {
        self.version.dependencies()
    }

    pub fn features(&self) -> &HashMap<String, Vec<String>> {
        self.version.features()
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
        self.version.name().hash(state);
        self.version.version().hash(state);
    }
}

pub fn get_crate(index: &crates_index::Index, name: &str) -> Result<crates_index::Crate> {
    index.crate_(name).ok_or(Error::CrateNotFound {
        crate_name: name.to_string(),
    })
}
