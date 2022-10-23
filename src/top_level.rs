use crate::common::{self, Version};
use crates_io_api::{CratesQuery, Sort, SyncClient};
use log::warn;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    Create(http::header::InvalidHeaderValue),
    QueryMostDownloadedCrates(crates_io_api::Error),
    MostDownloadedCrateNotFound(common::Error),
    FromFile(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create(e) => {
                write!(f, "failed to create top-level crates builder: {e}")
            }
            Error::QueryMostDownloadedCrates(e) => {
                write!(f, "failed to query the most downloaded crates: {e}")
            }
            Error::MostDownloadedCrateNotFound(e) => {
                write!(f, "failed to get most downloaded crate: {e}")
            }
            Error::FromFile(e) => {
                write!(f, "failed to get crates from the file: {e}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create(e) => Some(e),
            Error::QueryMostDownloadedCrates(e) => Some(e),
            Error::MostDownloadedCrateNotFound(e) => Some(e),
            Error::FromFile(e) => Some(e.as_ref()),
        }
    }
}

impl From<http::header::InvalidHeaderValue> for Error {
    fn from(e: http::header::InvalidHeaderValue) -> Self {
        Error::Create(e)
    }
}

impl From<crates_io_api::Error> for Error {
    fn from(e: crates_io_api::Error) -> Self {
        Error::QueryMostDownloadedCrates(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct TopLevelBuilder<'i> {
    index: &'i crates_index::Index,
    client: SyncClient,
}

impl<'i> TopLevelBuilder<'i> {
    pub fn new(index: &'i crates_index::Index) -> Result<Self> {
        let client = SyncClient::new(
            "my-user-agent (my-contact@domain.com)",
            std::time::Duration::from_millis(1000),
        )?;
        Ok(TopLevelBuilder { index, client })
    }

    pub fn get_n_most_downloaded(&self, n: u64) -> Result<Vec<Version>> {
        const PAGE_SIZE: u64 = 50;

        let mut num_pages = n / PAGE_SIZE;
        let mut trim_results = false;
        if n % PAGE_SIZE != 0 {
            num_pages += 1;
            trim_results = true;
        }

        let mut most_downloaded = Vec::new();

        let mut query = CratesQuery::builder()
            .sort(Sort::Downloads)
            .page_size(PAGE_SIZE)
            .build();
        for page_index in 0..num_pages {
            query.set_page(page_index + 1);
            let page = self.client.crates(query.clone())?;
            for crat in page.crates {
                let crat = common::get_crate(self.index, &crat.name)
                    .map_err(|e| Error::MostDownloadedCrateNotFound(e))?;
                let version = crat.highest_normal_version();
                if version.is_none() {
                    // No versions available for this crate. Skip over it.
                    warn!(
                        "no versions available for the most downloaded crate {}",
                        crat.name()
                    );
                    continue;
                }
                let version = common::Version::new(version.unwrap().clone()).download(true);
                most_downloaded.push(version);
            }
        }

        if trim_results {
            most_downloaded.truncate(n as usize);
        }
        Ok(most_downloaded)
    }

    pub fn from_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<Version>> {
        let file =
            BufReader::new(File::open(&file_path).map_err(|e| Error::FromFile(Box::new(e)))?);
        let mut crates = Vec::new();
        for line in file.lines() {
            let crate_name = line.map_err(|e| Error::FromFile(Box::new(e)))?;
            let crat = common::get_crate(self.index, &crate_name)
                .map_err(|e| Error::FromFile(Box::new(e)))?;
            let version = crat.highest_normal_version();
            if version.is_none() {
                // No versions available for this crate. Skip over it.
                let file_path = file_path.as_ref();
                warn!(
                    "no versions available for the {crate_name} crate in the {} file",
                    file_path.to_string_lossy()
                );
                continue;
            }
            let version = common::Version::new(version.unwrap().clone()).download(true);
            crates.push(version);
        }
        Ok(crates)
    }
}
