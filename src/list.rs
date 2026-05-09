use anyhow::Result;

use crate::{config::Config, file_collector::collect_files_and_links};

pub fn list(config: &Config) -> Result<()> {
    let collected = collect_files_and_links(config.dotfiles_home_dir())?;

    for warning in collected.warnings {
        eprintln!("[warning] {warning}");
    }

    println!("managed file(s):");
    for file in collected.files {
        println!("  {}", file.display());
    }

    Ok(())
}
