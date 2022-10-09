use std::path::{Path, PathBuf};
use crate::common::{MicrioError, Result};

pub struct DstRegistry {
    path: PathBuf,
}

impl DstRegistry {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.is_dir() {
            Err(MicrioError::DstRegistryPath { path: path.to_string_lossy().into_owned() })
        } else {
            Ok(DstRegistry { path })
        }
    }
}
