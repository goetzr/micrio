/*

micrio.exe --from-file crates.txt
    crates.txt
    ----------
    tokio
    rayon
micrio.exe --most-downloaded 50
 */

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    pub dst_registry_path: String,
    #[arg(long)]
    pub from_file: Option<PathBuf>,
    #[arg(long)]
    pub most_downloaded: Option<u64>,
}