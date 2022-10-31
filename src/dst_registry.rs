use crate::common::Version;
use git2::Repository;
use std::collections::HashSet;
use std::env;
use std::fmt::{self, Display};
use std::fs::{self, DirEntry, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::{task, sync};

#[derive(Debug)]
pub enum Error {
    Create {
        msg: String,
        error: io::Error,
    },
    CreateIndexDir(io::Error),
    InitGitRepo(git2::Error),
    WriteConfigJson(io::Error),
    AddCrateToIndex {
        crate_name: String,
        crate_version: String,
        msg: String,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    AddFileToGitRepo(Box<dyn std::error::Error + Send + Sync + 'static>),
    CommitGitRepo(git2::Error),
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
        msg: String,
        error: io::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create { msg, error } => {
                write!(
                    f,
                    "failed to create fresh destination registry directory: {msg}: {error}"
                )
            }
            Error::CreateIndexDir(e) => {
                write!(
                    f,
                    "error populating index: failed to create the index directory: {e}"
                )
            }
            Error::InitGitRepo(e) => {
                write!(
                    f,
                    "error populating index: failed to initialize git repo: {e}"
                )
            }
            Error::WriteConfigJson(e) => {
                write!(
                    f,
                    "error populating index: failed to write config.json file: {e}"
                )
            }
            Error::AddCrateToIndex {
                crate_name,
                crate_version,
                msg,
                error,
            } => {
                write!(
                    f,
                    "error populating index: failed to add {crate_name} version {crate_version} to the index: {msg}: {error}"
                )
            }
            Error::AddFileToGitRepo(e) => {
                write!(
                    f,
                    "error populating index: failed to add file to git repo: {e}"
                )
            }
            Error::CommitGitRepo(e) => {
                write!(f, "error populating index: failed to commit git repo: {e}")
            }
            Error::CreateRegistryDir(e) => {
                write!(
                    f,
                    "error populating registry: failed to create the registry directory: {e}"
                )
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
                msg,
                error,
            } => {
                write!(f, "error populating registry: failed to write {crate_name} version {crate_version} to its file on disk: {msg}: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create { error, .. } => Some(error),
            Error::CreateIndexDir(e) => Some(e),
            Error::InitGitRepo(e) => Some(e),
            Error::WriteConfigJson(e) => Some(e),
            Error::AddCrateToIndex { error, .. } => Some(error.as_ref()),
            Error::AddFileToGitRepo(e) => Some(e.as_ref()),
            Error::CommitGitRepo(e) => Some(e),
            Error::CreateRegistryDir(e) => Some(e),
            Error::CreateRuntime(e) => Some(e),
            Error::DownloadCrate { error, .. } => Some(error.as_ref()),
            Error::WriteRegistryFile { error, .. } => Some(error),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

const INDEX_DIR: &'static str = "index";
const REGISTRY_DIR: &'static str = "registry";

pub struct DstRegistry {
    path: PathBuf,
}

impl DstRegistry {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut path = path.as_ref().to_path_buf();

        // Ensure the path to the destination registry is an absolute path
        // with forward slashes separating the components.
        if !path.is_absolute() {
            let mut rel_path = path.as_path();
            if rel_path.starts_with("./") {
                rel_path = path.strip_prefix("./").unwrap();
            } else if rel_path.starts_with(".\\") {
                rel_path = path.strip_prefix(".\\").unwrap();
            }

            let cur_dir = env::current_dir().map_err(|e| Error::Create {
                msg: "failed to get current directory to make absolute path".to_string(),
                error: e,
            })?;
            path = cur_dir.join(&rel_path);
        }
        path = PathBuf::from_str(path.to_string_lossy().replace("\\", "/").as_str()).unwrap();

        // Remove the directory then re-create it so we can start with a clean directory.
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|e| Error::Create {
                msg: "failed to remove existing directory".to_string(),
                error: e,
            })?;
        }
        fs::create_dir(&path).map_err(|e| Error::Create {
            msg: "failed to create new directory".to_string(),
            error: e,
        })?;
        Ok(DstRegistry { path })
    }

    pub fn populate(&self, crates: &HashSet<Version>) -> Result<()> {
        let top_dir_path = self.path.to_string_lossy();
        populate_index(top_dir_path.as_ref(), crates)?;
        populate_registry(top_dir_path.as_ref(), crates)?;
        Ok(())
    }
}

fn populate_index(top_dir_path: &str, crates: &HashSet<Version>) -> Result<()> {
    let index_dir_path = format!("{top_dir_path}/{INDEX_DIR}");
    fs::create_dir(&index_dir_path).map_err(|e| Error::CreateIndexDir(e))?;

    let repo = create_git_repo(&index_dir_path)?;
    write_config_json_file(top_dir_path)?;
    add_crates_to_index(top_dir_path, &crates)?;
    add_files_to_git_repo(&index_dir_path, &repo)?;

    Ok(())
}

fn populate_registry(top_dir_path: &str, crates: &HashSet<Version>) -> Result<()> {
    let registry_dir_path = format!("{top_dir_path}/{REGISTRY_DIR}");
    fs::create_dir(&registry_dir_path).map_err(|e| Error::CreateRegistryDir(e))?;

    let crates = Vec::from_iter(crates.iter().cloned());
    let rt = tokio::runtime::Runtime::new().map_err(|e| Error::CreateRuntime(e))?;

    let sem = sync::Semaphore::new(100);
    let results = rt.block_on(download_crates(crates.clone(), &registry_dir_path, &sem));

    for (i, result) in results.into_iter().enumerate() {
        let name = crates[i].name();
        let version = crates[i].version();
        match result {
            Ok(_) => {
                //let crate_file_contents = fut_res?;
                //add_crate_to_registry(&registry_dir_path, name, version, crate_file_contents)?;
                ()
            }
            Err(e) => {
                // Task panicked.
                return Err(Error::DownloadCrate {
                    crate_name: name.to_string(),
                    crate_version: version.to_string(),
                    error: Box::new(e),
                });
            }
        }
    }

    Ok(())
}

fn create_git_repo(index_dir_path: &str) -> Result<Repository> {
    Repository::init(index_dir_path).map_err(|e| Error::InitGitRepo(e))
}

fn write_config_json_file(top_dir_path: &str) -> Result<()> {
    let config_json_path = format!("{top_dir_path}/{INDEX_DIR}/config.json");
    let config_json_contents = format!(
        r#"{{
    "dl": "file://{}/{REGISTRY_DIR}"
}}"#,
        top_dir_path
    );
    fs::write(config_json_path, config_json_contents).map_err(|e| Error::WriteConfigJson(e))?;
    Ok(())
}

fn add_crates_to_index(top_dir_path: &str, crates: &HashSet<Version>) -> Result<()> {
    for crat in crates {
        add_crate_to_index(top_dir_path, crat)?;
    }
    Ok(())
}

fn add_crate_to_index(top_dir_path: &str, crat: &Version) -> Result<()> {
    let crate_path = get_crate_index_path(top_dir_path, crat)?;

    let crate_path = format!("{crate_path}/{}", crat.name().to_lowercase());
    let mut crate_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(crate_path)
        .map_err(|e| Error::AddCrateToIndex {
            crate_name: crat.name().to_string(),
            crate_version: crat.version().to_string(),
            msg: "failed to open crate file".to_string(),
            error: Box::new(e),
        })?;

    let mut crate_version_info = crat.to_json().map_err(|e| Error::AddCrateToIndex {
        crate_name: crat.name().to_string(),
        crate_version: crat.version().to_string(),
        msg: "failed to serialize crate version information to a string".to_string(),
        error: Box::new(e),
    })?;
    crate_version_info += "\n";


    crate_file
        .write_all(crate_version_info.as_bytes())
        .map_err(|e| Error::AddCrateToIndex {
            crate_name: crat.name().to_string(),
            crate_version: crat.version().to_string(),
            msg: "failed to write crate version information to file".to_string(),
            error: Box::new(e),
        })?;

    Ok(())
}

fn get_crate_index_path(top_dir_path: &str, crat: &Version) -> Result<String> {
    let crate_name = crat.name().to_lowercase();
    match crate_name.len() {
        1 => {
            let crate_path = format!("{top_dir_path}/{INDEX_DIR}/1");
            if !Path::new(&crate_path).exists() {
                fs::create_dir(&crate_path).map_err(|e| Error::AddCrateToIndex {
                    crate_name: crat.name().to_string(),
                    crate_version: crat.version().to_string(),
                    msg: "failed to create '1' directory".to_string(),
                    error: Box::new(e),
                })?;
            }
            Ok(crate_path)
        }
        2 => {
            let crate_path = format!("{top_dir_path}/{INDEX_DIR}/2");
            if !Path::new(&crate_path).exists() {
                fs::create_dir(&crate_path).map_err(|e| Error::AddCrateToIndex {
                    crate_name: crat.name().to_string(),
                    crate_version: crat.version().to_string(),
                    msg: "failed to create '2' directory".to_string(),
                    error: Box::new(e),
                })?;
            }
            Ok(crate_path)
        }
        3 => {
            let crate_path = format!("{top_dir_path}/{INDEX_DIR}/3");
            if !Path::new(&crate_path).exists() {
                fs::create_dir(&crate_path).map_err(|e| Error::AddCrateToIndex {
                    crate_name: crat.name().to_string(),
                    crate_version: crat.version().to_string(),
                    msg: "failed to create '3' directory".to_string(),
                    error: Box::new(e),
                })?;
            }

            let crate_path = format!(
                "{crate_path}/{}",
                crate_name.chars().take(1).collect::<String>()
            );
            if !Path::new(&crate_path).exists() {
                fs::create_dir(&crate_path).map_err(|e| Error::AddCrateToIndex {
                    crate_name: crat.name().to_string(),
                    crate_version: crat.version().to_string(),
                    msg: "failed to create crate directory in '3' directory".to_string(),
                    error: Box::new(e),
                })?;
            }
            Ok(crate_path)
        }
        _ => {
            let dir1_name = crate_name.chars().take(2).collect::<String>();
            let crate_path = format!("{top_dir_path}/{INDEX_DIR}/{dir1_name}");
            if !Path::new(&crate_path).exists() {
                fs::create_dir(&crate_path).map_err(|e| Error::AddCrateToIndex {
                    crate_name: crat.name().to_string(),
                    crate_version: crat.version().to_string(),
                    msg: format!("failed to create {dir1_name} directory"),
                    error: Box::new(e),
                })?;
            }

            let dir2_name = crate_name.chars().skip(2).take(2).collect::<String>();
            let crate_path = format!("{crate_path}/{dir2_name}");
            if !Path::new(&crate_path).exists() {
                fs::create_dir(&crate_path).map_err(|e| Error::AddCrateToIndex {
                    crate_name: crat.name().to_string(),
                    crate_version: crat.version().to_string(),
                    msg: format!("failed to create {dir2_name} directory"),
                    error: Box::new(e),
                })?;
            }

            Ok(crate_path)
        }
    }
}

fn add_files_to_git_repo(index_dir_path: &str, repo: &Repository) -> Result<()> {
    let mut index = repo
        .index()
        .map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
    let entries = fs::read_dir(index_dir_path).map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
        add_file_to_git_repo(index_dir_path, &mut index, &entry)
            .map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
    }
    index
        .write()
        .map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
    commit_git_repo(repo, &mut index)?;
    Ok(())
}

fn add_file_to_git_repo(
    index_dir_path: &str,
    index: &mut git2::Index,
    entry: &DirEntry,
) -> Result<()> {
    let metadata = entry
        .metadata()
        .map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
    if metadata.is_file() {
        let path = pathdiff::diff_paths(entry.path(), index_dir_path).unwrap();
        index
            .add_path(&path)
            .map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
    } else if metadata.is_dir() {
        if entry.file_name() != ".git" {
            let entries =
                fs::read_dir(entry.path()).map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
            for entry in entries {
                let entry = entry.map_err(|e| Error::AddFileToGitRepo(Box::new(e)))?;
                add_file_to_git_repo(index_dir_path, index, &entry)?
            }
        }
    }
    Ok(())
}

fn commit_git_repo(repo: &Repository, index: &mut git2::Index) -> Result<()> {
    let oid = index.write_tree().map_err(|e| Error::CommitGitRepo(e))?;
    let signature = git2::Signature::now("Russ Goetz", "russgoetz@gmail.com")
        .map_err(|e| Error::CommitGitRepo(e))?;
    //let parent_commit = find_last_commit(&repo)?;
    let tree = repo.find_tree(oid).map_err(|e| Error::CommitGitRepo(e))?;
    repo.commit(
        Some("HEAD"),     //  point HEAD to our new commit
        &signature,       // author
        &signature,       // committer
        "Initial commit", // commit message
        &tree,            // tree
        &[],
        //&[&parent_commit],
    )
    .map_err(|e| Error::CommitGitRepo(e))?; // parents
    Ok(())
}

async fn download_crates(
    crates: Vec<Version>,
    registry_dir_path: &str,
    sem: &sync::Semaphore,
) -> Vec<std::result::Result<Result<()>, task::JoinError>> {
    let mut results = Vec::new();
    for (i, crat) in crates.iter().enumerate() {
        let _permit = sem.acquire().await.expect("acquire semaphore");
        let name = crat.name().to_string();
        let version = crat.version().to_string();
        let path = registry_dir_path.to_string();
        let result = tokio::spawn(async move {
            download_crate(&name, &version, &path).await
        }).await;
        results.push(result);
        println!("Downloaded {:>4} of {:>4}: {} version {}", i+1, crates.len(), crates[i].name(), crates[i].version());
    }
    results
}

async fn download_crate(name: &str, version: &str, registry_dir_path: &str) -> Result<()> {
    const DL_URL: &'static str = "https://static.crates.io/crates";
    let crate_url = format!("{DL_URL}/{name}/{name}-{version}.crate");

    let response = reqwest::get(crate_url)
        .await
        .map_err(|e| Error::DownloadCrate {
            crate_name: name.to_string(),
            crate_version: version.to_string(),
            error: Box::new(e),
        })?;

    let bytes = response.bytes().await.map_err(|e| Error::DownloadCrate {
        crate_name: name.to_string(),
        crate_version: version.to_string(),
        error: Box::new(e),
    })?;

    add_crate_to_registry(registry_dir_path, name, version, bytes)
}

fn add_crate_to_registry(
    registry_dir_path: &str,
    name: &str,
    version: &str,
    file_contents: bytes::Bytes,
) -> Result<()> {
    let crate_dir_path = format!("{registry_dir_path}/{name}");
    if !Path::new(&crate_dir_path).exists() {
        fs::create_dir(&crate_dir_path).map_err(|e| Error::WriteRegistryFile {
            crate_name: name.to_string(),
            crate_version: version.to_string(),
            msg: format!("failed to create {name} directory"),
            error: e,
        })?;
    }
    let crate_dir_path = format!("{crate_dir_path}/{version}");
    fs::create_dir(&crate_dir_path).map_err(|e| Error::WriteRegistryFile {
        crate_name: name.to_string(),
        crate_version: version.to_string(),
        msg: format!("failed to create {version} directory"),
        error: e,
    })?;
    let crate_file_path = format!("{crate_dir_path}/download");
    fs::write(crate_file_path, file_contents).map_err(|e| Error::WriteRegistryFile {
        crate_name: name.to_string(),
        crate_version: version.to_string(),
        msg: "failed to write contents to file".to_string(),
        error: e,
    })?;
    Ok(())
}
