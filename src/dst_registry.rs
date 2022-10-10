use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::fmt::{self, Display};
use std::collections::HashSet;
use crate::common::Version;

#[derive(Debug)]
pub enum Error {
    Create(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create(e) => {
                write!(f, "failed to create destination registry: {e}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create(e) => Some(e),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct DstRegistry {
    path: PathBuf,
}

impl DstRegistry {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Remove the directory then re-create it so we can start with a clean directory.
        let path = path.as_ref();
        fs::remove_dir_all(path).map_err(|e| Error::Create(e))?;
        fs::create_dir(path).map_err(|e| Error::Create(e))?;
        Ok(DstRegistry{ path: path.to_path_buf() })
    }

    pub fn populate(&self, crates: HashSet<Version>) -> Result<()> {
        Ok(())
    }
}
