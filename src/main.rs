mod common;
mod src_registry;
mod local_registry;

use anyhow::Context;
use common::CrateId;
use src_registry::SrcIndex;
use log::error;

fn try_main() -> anyhow::Result<()> {
    let mut src_index = SrcIndex::new()?;
    let crate_ids = Vec::<CrateId>::new();
    let dep_crate_ids = src_index.get_required_dependencies(&crate_ids)?;
    for dep_crate_id in &dep_crate_ids {
        println!("Dependent crate: {} version {}", dep_crate_id.name, dep_crate_id.version);
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
