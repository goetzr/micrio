use crate::common::Version;
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    Create(io::Error),
    CreateRuntime(io::Error),
    DownloadCrate {
        crate_name: String,
        crate_version: String,
        error: reqwest::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create(e) => {
                write!(f, "failed to create destination registry: {e}")
            }
            Error::CreateRuntime(e) => {
                write!(f, "failed to create tokio runtime to download crates: {e}")
            }
            Error::DownloadCrate {
                crate_name,
                crate_version,
                error,
            } => {
                write!(f, "failed to {crate_name} version {crate_version}: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create(e) => Some(e),
            Error::CreateRuntime(e) => Some(e),
            Error::DownloadCrate { error, .. } => Some(error),
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
        Ok(DstRegistry {
            path: path.to_path_buf(),
        })
    }

    pub fn populate(&self, crates: &HashSet<Version>) -> Result<()> {
        Ok(())
    }

    fn populate_index(&self, crates: &HashSet<Version>) -> Result<()> {
        // TODO: Write config.json file.
        // TODO: Serialize each crate version to JSON at the appropriate location in the index.
        const INDEX_DIR: &'static str = "index";
        Ok(())
    }

    fn populate_registry(&self, crates: &HashSet<Version>) -> Result<()> {
        // TODO: Download each crate to the appropriate location in the index.
        // https://static.crates.io/crates/{name}/{name}-{version}.crate
        const DL_URL: &'static str = "https://static.crates.io/crates";
        let mut rt = tokio::runtime::Runtime::new().map_err(|e| Error::CreateRuntime(e))?;

        async fn download_crate(crat: &Version) -> Result<reqwest::Response> {
            let response = reqwest::get(format!(
                "{DL_URL}/{}/{}-{}.crate",
                crat.name(),
                crat.name(),
                crat.version()
            ))
            .await
            .map_err(|e| Error::DownloadCrate {
                crate_name: crat.name().to_string(),
                crate_version: crat.version().to_string(),
                error: e,
            });
            response
        }

        rt.block_on(async {
            for crat in crates {
                tokio::spawn(download_crate(crat));
            }
        });
        Ok(())
    }
}
