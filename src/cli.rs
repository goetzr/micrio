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
#[command(about = "Mirrors a subset of crates from crates.io to a local registry.")]
pub struct Cli {
    /// Path to the directory where the crates should be mirrored.
    #[arg(value_name = "MIRROR-DIR-PATH")]
    pub mirror_dir_path: String,
    /// Mirror the crates listed in the specified file.
    /// Each line in the file must contain a crate name.
    #[arg(long, value_name = "FILE-PATH", verbatim_doc_comment)]
    pub from_file: Option<PathBuf>,
    /// Mirror the top N most downloaded crates on crates.io.
    #[arg(long, value_name = "N")]
    pub most_downloaded: Option<u64>,
}