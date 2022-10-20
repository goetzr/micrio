use crate::common::{self, Version};
use crates_io_api::{CratesQuery, Sort, SyncClient};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    Create(http::header::InvalidHeaderValue),
    QueryMostDownloadedCrates(crates_io_api::Error),
    MostDownloadedCrateNotFound(common::Error),
    HandPickedCrateNotFound(common::Error),
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
            Error::HandPickedCrateNotFound(e) => {
                write!(f, "failed to get hand-picked crate: {e}")
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
            Error::HandPickedCrateNotFound(e) => Some(e),
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
                let crate_version =
                    common::get_crate_version(self.index, &crat.name, &crat.max_version)
                        .map_err(|e| Error::MostDownloadedCrateNotFound(e))?;
                most_downloaded.push(crate_version);
            }
        }

        if trim_results {
            most_downloaded.truncate(n as usize);
        }
        Ok(most_downloaded)
    }

    pub fn get_handpicked(&self) -> Result<Vec<Version>> {
        let name = "tokio";
        let version = "1.21.2";
        let crate_version = common::get_crate_version(self.index, name, version)
            .map_err(|e| Error::HandPickedCrateNotFound(e))?;
        Ok(vec![crate_version])
    }
}
