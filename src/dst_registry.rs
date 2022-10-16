use crate::common::Version;
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tokio::task;

#[derive(Debug)]
pub enum Error {
    Create {
        msg: String,
        error: io::Error,
    },
    CreateIndexDir(io::Error),
    CreateRegistryDir(io::Error),
    CreateRuntime(io::Error),
    DownloadCrate {
        crate_name: String,
        crate_version: String,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    WriteRegistryFile {
        crate_name: String,
        crate_version: String,
        error: io::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create { msg, error } => {
                write!(f, "failed to create fresh destination registry directory: {msg}: {error}")
            }
            Error::CreateIndexDir(e) => {
                write!(f, "error populating index: failed to create the index directory: {e}")
            }
            Error::CreateRegistryDir(e) => {
                write!(f, "error populating registry: failed to create the registry directory: {e}")
            }
            Error::CreateRuntime(e) => {
                write!(f, "error populating registry: failed to create tokio runtime to download crates: {e}")
            }
            Error::DownloadCrate {
                crate_name,
                crate_version,
                error,
            } => {
                write!(f, "error populating registry: failed to download {crate_name} version {crate_version}: {error}")
            }
            Error::WriteRegistryFile {
                crate_name,
                crate_version,
                error,
            } => {
                write!(f, "error populating registry: failed to write {crate_name} version {crate_version} to its file on disk: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create { error, .. } => Some(error),
            Error::CreateIndexDir(e) => Some(e),
            Error::CreateRegistryDir(e) => Some(e),
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
        fs::remove_dir_all(path).map_err(|e| Error::Create { msg: "failed to remove existing directory".to_string(), error: e })?;
        fs::create_dir(path).map_err(|e| Error::Create { msg: "failed to create new directory".to_string(), error: e })?;
        Ok(DstRegistry {
            path: path.to_path_buf(),
        })
    }

    pub fn populate(&self, crates: &HashSet<Version>) -> Result<()> {
        Ok(())
    }

    fn populate_index(&self, crates: &HashSet<Version>) -> Result<()> {
        const INDEX_DIR: &'static str = "index";
        fs::create_dir(INDEX_DIR).map_err(|e| Error::CreateIndexDir(e));

        // TODO: Write config.json file.
        // TODO: Serialize each crate version to JSON at the appropriate location in the index.

        Ok(())
    }

    fn populate_registry(&self, crates: &HashSet<Version>) -> Result<()> {
        const REGISTRY_DIR: &'static str = "registry";
        fs::create_dir(REGISTRY_DIR).map_err(|e| Error::CreateRegistryDir(e));

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

fn write_crate_file(name: &str, version: &str, file_contents: bytes::Bytes) -> Result<()> {
    // TODO: Need constant for "registry" folder.
    File::create()
    unimplemented!()
}