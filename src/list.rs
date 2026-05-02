use anyhow::Result;

use crate::{config::Config, file_collector::collect_files_and_links};

pub fn list(config: &Config) -> Result<()> {
    let (files, _) = collect_files_and_links(config.dotfiles_home_dir())?;

    println!("managed file(s):");
    for file in files {
        println!("  {}", file.display());
    }

    Ok(())
}
