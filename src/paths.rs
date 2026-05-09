use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::Config;

pub struct PathResolver<'a> {
    config: &'a Config,
}

impl<'a> PathResolver<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn install_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        Ok(self.config.home_dir().join(
            path.as_ref()
                .strip_prefix(self.config.dotfiles_home_dir())?,
        ))
    }

    pub fn managed_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        Ok(self
            .config
            .dotfiles_home_dir()
            .join(path.as_ref().strip_prefix(self.config.home_dir())?))
    }

    pub fn backup_path(&self, path: impl AsRef<Path>, backup_dir: &Path) -> Result<PathBuf> {
        Ok(backup_dir.join(path.as_ref().strip_prefix(self.config.home_dir())?))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn config(root: &TempDir) -> Config {
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let home_dir = root.path().join("home");
        let backup_root_dir = root.path().join("backup");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&backup_root_dir).unwrap();

        Config::from_parts(dotfiles_dir, home_dir, backup_root_dir, dotfiles_home_dir).unwrap()
    }

    #[test]
    fn resolves_install_path() {
        let root = TempDir::new().unwrap();
        let config = config(&root);
        let resolver = PathResolver::new(&config);

        let path = config.dotfiles_home_dir().join(".config/app/config.toml");

        assert_eq!(
            resolver.install_path(path).unwrap(),
            config.home_dir().join(".config/app/config.toml")
        );
    }

    #[test]
    fn resolves_managed_path() {
        let root = TempDir::new().unwrap();
        let config = config(&root);
        let resolver = PathResolver::new(&config);

        let path = config.home_dir().join(".zshrc");

        assert_eq!(
            resolver.managed_path(path).unwrap(),
            config.dotfiles_home_dir().join(".zshrc")
        );
    }
}
