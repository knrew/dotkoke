use std::path::PathBuf;

use anyhow::Result;

use crate::{config::Config, file_collector::collect_files_and_links};

pub fn list(config: &Config) -> Result<()> {
    let collected = collect_files_and_links(config.dotfiles_home_dir())?;

    for warning in collected
        .warnings
        .into_iter()
        .chain(collected.collection_errors)
    {
        eprintln!("[warning] {warning}");
    }

    print!("{}", list_stdout(&collected.files));

    Ok(())
}

fn list_stdout(files: &[PathBuf]) -> String {
    let mut output = String::from("managed file(s):\n");

    for file in files {
        output.push_str(&format!("  {}\n", file.display()));
    }

    output
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn list_stdout_uses_absolute_paths() {
        let files = vec![
            PathBuf::from("/dotfiles/home/.config/app/config.toml"),
            PathBuf::from("/dotfiles/home/.zshrc"),
        ];

        assert_eq!(
            list_stdout(&files),
            "managed file(s):\n  /dotfiles/home/.config/app/config.toml\n  /dotfiles/home/.zshrc\n"
        );
    }
}
