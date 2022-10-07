mod common;
mod local_registry;
mod src_registry;

use common::Version;
use log::error;
use src_registry::SrcIndex;

fn try_main() -> anyhow::Result<()> {
    env_logger::init();
    let index = crates_index::Index::new_cargo_default()?;
    let src_index = SrcIndex::new(&index)?;
    let indexmap_crate = index.crate_("indexmap").expect("failed to get top level crate");
    let indexmap_crate_version = indexmap_crate.highest_normal_version().expect("failed to get top level crate version");
    let top_level_crates = vec![Version(indexmap_crate_version.clone())];
    let required_dependencies = src_index.get_required_dependencies(&top_level_crates)?;
    for dep_crate in &required_dependencies {
        println!(
            "Dependent crate: {} version {}",
            dep_crate.name(), dep_crate.version()
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
