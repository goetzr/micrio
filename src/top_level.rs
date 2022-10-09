use crate::common::{self, MicrioError, Result, Version};

pub struct TopLevel<'i> {
    index: &'i crates_index::Index,
}

impl<'i> TopLevel<'i> {
    pub fn new(index: &'i crates_index::Index) -> Self {
        TopLevel { index }
    }

    pub fn get_n_most_downloaded(&self, _n: u32) -> Result<Vec<Version>> {
        let crate_version = self.get_crate_version("tokio", "1.21.2")?;
        Ok(vec![crate_version])
    }

    fn get_crate_version(&self, name: &str, version: &str) -> Result<Version> {
        let crat = self.get_crate(name)?;
        let crate_version = crat
            .versions()
            .iter()
            .rev()
            .find(|v| v.version() == version)
            .ok_or(MicrioError::CrateVersionNotFound {
                crate_name: name.to_string(),
                crate_version: version.to_string(),
            })?;
        Ok(Version(crate_version.clone()))
    }

    fn get_crate(&self, name: &str) -> Result<crates_index::Crate> {
        common::get_crate(self.index, name)
    }

    pub fn get_handpicked(&self) -> Result<Vec<Version>> {
        let name = "bytes";
        let version = "1.2.1";
        let crat = self.get_crate(name)?;
        let crate_version = crat
            .versions()
            .iter()
            .rev()
            .find(|v| v.version() == version)
            .ok_or(MicrioError::CrateVersionNotFound {
                crate_name: name.to_string(),
                crate_version: version.to_string(),
            })?;
        let crate_version = Version(crate_version.clone());
        Ok(vec![crate_version])
    }
}
