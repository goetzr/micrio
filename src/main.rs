mod common;
mod top_level;
mod src_registry;
mod dst_registry;
mod cli;

use top_level::TopLevelBuilder;
use src_registry::SrcRegistry;
use dst_registry::DstRegistry;
use std::collections::HashSet;
use log::error;
use cli::Cli;
use clap::Parser;

fn try_main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let index = crates_index::Index::new_cargo_default()?;
    let top_level_builder = TopLevelBuilder::new(&index)?;
    let src_registry = SrcRegistry::new(&index)?;
    let dst_registry = DstRegistry::new(&cli.mirror_dir_path)?;

    let mut crates = HashSet::new();
    match cli.from_file {
        Some(file_path) => crates.extend(top_level_builder.from_file(file_path)?),
        None => (),
    };
    match cli.most_downloaded {
        Some(n) => crates.extend(top_level_builder.get_n_most_downloaded(n)?),
        None => (),
    };

    if crates.is_empty() {
        println!("ERROR: no crates selected to mirror");
        cli.print_help();
        std::process::exit(1);
    }

    let dependencies = src_registry.get_required_dependencies(&crates)?;
    crates.extend(dependencies);

    dst_registry.populate(&crates)?;

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
