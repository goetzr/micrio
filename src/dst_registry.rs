use crate::common::Version;
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tokio::task;

#[derive(Debug)]
pub enum Error {
    Create(io::Error),
    CreateRuntime(io::Error),
    DownloadCrate {
        crate_name: String,
        crate_version: String,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
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
            Error::DownloadCrate { error, .. } => Some(error.as_ref()),
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
        let crates = crates.iter().cloned().collect::<Vec<_>>();
        let rt = tokio::runtime::Runtime::new().map_err(|e| Error::CreateRuntime(e))?;
        let results = rt.block_on(download_crates(crates.clone()));
        
        for (i, result) in results.into_iter().enumerate() {
            let name = crates[i].name();
            let version = crates[i].version();
            match result {
                Ok(fut_res) => {
                    let crate_file_contents = fut_res?;
                    // TODO: Write response to appropriate file in registry.
                },
                Err(e) => {
                    return Err(Error::DownloadCrate { crate_name: name.to_string(), crate_version: version.to_string(), error: Box::new(e) });
                }
            }
        }

        Ok(())
    }

    
}

async fn download_crates(crates: Vec<Version>) -> Vec<std::result::Result<Result<bytes::Bytes>, task::JoinError>> {
    let mut handles = Vec::new();
    for crat in crates {
        let name = crat.name().to_string();
        let version = crat.version().to_string();
        handles.push(tokio::spawn(async move {
            download_crate(&name, &version).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }
    results
}

async fn download_crate(name: &str, version: &str) -> Result<bytes::Bytes> {
    const DL_URL: &'static str = "https://static.crates.io/crates";
    let crate_url = format!(
        "{DL_URL}/{}/{}-{}.crate",
        name,
        name,
        version
    );

    let response = reqwest::get(crate_url)
        .await
        .map_err(|e| Error::DownloadCrate {
            crate_name: name.to_string(),
            crate_version: version.to_string(),
            error: Box::new(e),
        })?;

    let bytes = response.bytes()
        .await
        .map_err(|e| Error::DownloadCrate {
            crate_name: name.to_string(),
            crate_version: version.to_string(),
            error: Box::new(e),
        });
    bytes
}