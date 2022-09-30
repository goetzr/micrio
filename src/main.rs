mod common;
mod local_registry;
mod src_registry;

use common::CrateId;
use log::error;
use src_registry::{SourceIndex, Index, Crate, CrateVersion, Dependency, DependencyKind};
use std::collections::HashMap;

impl Dependency for crates_index::Dependency {
    fn name(&self) -> &str {
        self.name()
    }

    fn requirement(&self) -> &str {
        self.requirement()
    }

    fn features(&self) -> &[String] {
        self.features()
    }

    fn is_optional(&self) -> bool {
        self.is_optional()
    }

    fn has_default_features(&self) -> bool {
        self.has_default_features()
    }

    fn target(&self) -> Option<&str> {
        self.target()
    }

    fn kind(&self) -> DependencyKind {
        match self.kind() {
            crates_index::DependencyKind::Normal => DependencyKind::Normal,
            crates_index::DependencyKind::Build => DependencyKind::Build,
            crates_index::DependencyKind::Dev => DependencyKind::Dev,
        }
    }

    fn crate_name(&self) -> &str {
        self.crate_name()
    }
}

impl CrateVersion for crates_index::Version {
    fn name(&self) -> &str {
        self.name()
    }

    fn version(&self) -> &str {
        self.version()
    }

    fn dependencies(&self) -> Vec<&dyn Dependency> {
        self.dependencies().iter().map(|d| d as &dyn Dependency).collect::<Vec<_>>()
    }

    fn features(&self) -> &HashMap<String, Vec<String>> {
        self.features()
    }

    fn is_yanked(&self) -> bool {
        self.is_yanked()
    }
    
    fn clone(&self) -> Box<dyn CrateVersion> {
        Box::new(Clone::clone(self))
    }
}

impl Crate for crates_index::Crate {
    fn name(&self) -> &str {
        self.name()
    }

    fn versions(&self) -> Vec<&dyn CrateVersion> {
        self.versions().iter().map(|v| v as &dyn CrateVersion).collect::<Vec<_>>()
    }
}

impl Index for crates_index::Index {
    fn get_crate(&self, name: &str) -> Option<Box<dyn Crate>> {
        match self.crate_(name) {
            Some(crat) => Some(Box::new(crat)),
            None => None,
        }
    }
}

fn try_main() -> anyhow::Result<()> {
    env_logger::init();
    let index = crates_index::Index::new_cargo_default()?;
    let src_index = SourceIndex::new(&index)?;
    let crate_ids = Vec::<CrateId>::new();
    let dep_crate_ids = src_index.get_required_dependencies(&crate_ids)?;
    for dep_crate_id in &dep_crate_ids {
        println!(
            "Dependent crate: {} version {}",
            dep_crate_id.name, dep_crate_id.version
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = try_main() {
        let mut msg = format!("{}", error);
        for cause in error.chain() {
            msg += &format!("\n\tCaused by: {}", cause);
        }
        error!("{}", msg);
    }
}
