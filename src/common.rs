use crates_index;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use serde::Serialize;

pub const TARGET_TRIPLE: &'static str = "x86_64-pc-windows-msvc";
pub const DEFAULT_FEATURE: &'static str = "default";

#[derive(Debug)]
pub enum Error {
    CrateNotFound {
        crate_name: String,
    },
    CrateVersionNotFound {
        crate_name: String,
        crate_version: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CrateNotFound { crate_name } => {
                write!(f, "{} not found in the source registry", crate_name)
            }
            Error::CrateVersionNotFound {
                crate_name,
                crate_version,
            } => {
                write!(
                    f,
                    "{} version {} not found in the source registry",
                    crate_name, crate_version
                )
            }
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Serialize)]
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

pub fn get_crate_version(
    index: &crates_index::Index,
    name: &str,
    version: &str,
) -> Result<Version> {
    let crat = get_crate(index, name)?;
    let crate_version = crat
        .versions()
        .iter()
        .rev()
        .find(|v| v.version() == version)
        .ok_or(Error::CrateVersionNotFound {
            crate_name: name.to_string(),
            crate_version: version.to_string(),
        })?;
    Ok(Version::new(crate_version.clone()))
}
