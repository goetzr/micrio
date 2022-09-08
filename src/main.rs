mod common;
mod src_registry;
mod local_registry;

use anyhow::Context;
use src_registry::CratesIoIndex;
use log::error;

fn try_main() -> anyhow::Result<()> {
    let index = crates_index::Index::new_cargo_default()?;
    let index2 = CratesIoIndex::new(&index);

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
