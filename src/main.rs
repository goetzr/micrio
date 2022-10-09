mod common;
mod top_level;
mod src_registry;
mod dst_registry;

use top_level::TopLevel;
use src_registry::SrcRegistry;
use dst_registry::DstRegistry;
use std::collections::HashSet;
use log::error;

fn try_main() -> anyhow::Result<()> {
    env_logger::init();
    let index = crates_index::Index::new_cargo_default()?;
    let top_level = TopLevel::new(&index);
    let src_registry = SrcRegistry::new(&index)?;
    let dst_registry = DstRegistry::new("./offline_registry");

    let most_downloaded = top_level.get_n_most_downloaded(50)?;
    let handpicked = top_level.get_handpicked()?;
    let mut crates = HashSet::from_iter(most_downloaded.into_iter().chain(handpicked.into_iter()));

    let dependencies = src_registry.get_required_dependencies(&crates)?;
    crates.extend(dependencies);

    for crat in &crates {
        println!("{} version {}", crat.name(), crat.version());
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
