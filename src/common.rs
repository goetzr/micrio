use crates_index;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

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
pub struct Version(pub crates_index::Version);

impl Version {
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.0).map_err(|e| Error::SerializeVersion(e))
    }

    pub fn name(&self) -> &str {
        self.0.name()
    }

    pub fn version(&self) -> &str {
        self.0.version()
    }

    pub fn dependencies(&self) -> &[crates_index::Dependency] {
        self.0.dependencies()
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

pub fn get_crate(index: &crates_index::Index, name: &str) -> Result<crates_index::Crate> {
    index.crate_(name).ok_or(Error::CrateNotFound {
        crate_name: name.to_string(),
    })
}
