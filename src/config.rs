//! 設定ファイル(toml)から設定を読み込む．

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct General {
    #[serde(rename = "dotfiles")]
    dotfiles_dir: PathBuf,

    #[serde(rename = "home")]
    home_dir: PathBuf,

    #[serde(rename = "backup_dir")]
    backup_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct Toml {
    general: General,
}

#[derive(Debug, Clone)]
pub struct Config {
    // dotfiles レポジトリのルート．
    dotfiles_dir: PathBuf,

    // $HOME．
    home_dir: PathBuf,

    // バックアップのルートディレクトリ．
    backup_root_dir: PathBuf,

    // $HOMEのミラー．
    // dotfiles/home/
    dotfiles_home_dir: PathBuf,
}

impl Config {
    pub fn read(config_toml_path: impl AsRef<Path>) -> Result<Self> {
        let config_toml_path = config_toml_path.as_ref();

        let toml_str = fs::read_to_string(config_toml_path).with_context(|| {
            format!("failed to read config file: {}", config_toml_path.display())
        })?;

        let Toml {
            general:
                General {
                    dotfiles_dir,
                    home_dir,
                    backup_dir,
                },
        } = toml::from_str(&toml_str).with_context(|| {
            format!(
                "failed to parse config file: {}",
                config_toml_path.display()
            )
        })?;

        let dotfiles_dir = dotfiles_dir.canonicalize().with_context(|| {
            format!(
                "invalid dotfiles directory in config: {}",
                dotfiles_dir.display()
            )
        })?;

        if !dotfiles_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", dotfiles_dir.display()));
        }

        let home_dir = home_dir
            .canonicalize()
            .with_context(|| format!("invalid home directory in config: {}", home_dir.display()))?;

        if !home_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", home_dir.display()));
        }

        let backup_root_dir = backup_dir.canonicalize().with_context(|| {
            format!(
                "invalid backup directory in config: {}",
                backup_dir.display()
            )
        })?;

        let dotfiles_home_dir = dotfiles_dir
            .join("home")
            .canonicalize()
            .with_context(|| format!("invalid path: {}/home", dotfiles_dir.display()))?;

        if !dotfiles_home_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", dotfiles_home_dir.display()));
        }

        let config = Config {
            dotfiles_dir,
            home_dir,
            backup_root_dir,
            dotfiles_home_dir,
        };

        Ok(config)
    }

    pub fn from_parts(
        dotfiles_dir: PathBuf,
        home_dir: PathBuf,
        backup_root_dir: PathBuf,
        dotfiles_home_dir: PathBuf,
    ) -> Self {
        Self {
            dotfiles_dir,
            home_dir,
            backup_root_dir,
            dotfiles_home_dir,
        }
    }

    pub fn dotfiles_dir(&self) -> &Path {
        &self.dotfiles_dir
    }

    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    pub fn backup_root_dir(&self) -> &Path {
        &self.backup_root_dir
    }

    pub fn dotfiles_home_dir(&self) -> &Path {
        &self.dotfiles_home_dir
    }

    pub fn backup_dir_for_timestamp(&self, timestamp: &str) -> PathBuf {
        self.backup_root_dir.join(timestamp)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn reads_config_without_timestamping_backup_dir() {
        let root = TempDir::new().unwrap();
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let home_dir = root.path().join("home");
        let backup_dir = root.path().join("backup");
        let config_path = root.path().join("dotkoke.toml");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(
            &config_path,
            format!(
                "[general]\ndotfiles = {:?}\nhome = {:?}\nbackup_dir = {:?}\n",
                dotfiles_dir, home_dir, backup_dir
            ),
        )
        .unwrap();

        let config = Config::read(config_path).unwrap();

        assert_eq!(config.dotfiles_dir(), dotfiles_dir.canonicalize().unwrap());
        assert_eq!(config.home_dir(), home_dir.canonicalize().unwrap());
        assert_eq!(config.backup_root_dir(), backup_dir.canonicalize().unwrap());
        assert_eq!(
            config.dotfiles_home_dir(),
            dotfiles_home_dir.canonicalize().unwrap()
        );
    }

    #[test]
    fn rejects_missing_dotfiles_home_dir() {
        let root = TempDir::new().unwrap();
        let dotfiles_dir = root.path().join("dotfiles");
        let home_dir = root.path().join("home");
        let backup_dir = root.path().join("backup");
        let config_path = root.path().join("dotkoke.toml");

        fs::create_dir_all(&dotfiles_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(
            &config_path,
            format!(
                "[general]\ndotfiles = {:?}\nhome = {:?}\nbackup_dir = {:?}\n",
                dotfiles_dir, home_dir, backup_dir
            ),
        )
        .unwrap();

        assert!(Config::read(config_path).is_err());
    }
}
