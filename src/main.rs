mod cli;
mod common;
mod dst_registry;
mod src_registry;
mod top_level;

use clap::{CommandFactory, Parser};
use cli::Cli;
use dst_registry::DstRegistry;
use log::error;
use src_registry::SrcRegistry;
use std::collections::HashSet;
use top_level::TopLevelBuilder;

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
        println!("ERROR: no crates selected to mirror\n");
        Cli::command().print_help()?;
        std::process::exit(1);
    }

    println!("{} top level crates selected.", crates.len());
    println!("Getting required dependencies...");
    let dependencies = src_registry.get_required_dependencies(&crates)?;
    let tot_num_deps = dependencies.len();
    let num_deps_dl = dependencies.iter().filter(|d| d.download).count();
    crates.extend(dependencies);
    println!("Done getting required dependencies.");
    println!(
        "{} total dependencies identified, {} of these must be downloaded.",
        tot_num_deps, num_deps_dl
    );

    println!("Populating local registry...");
    dst_registry.populate(&crates)?;
    println!("Done populating local registry.");

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
