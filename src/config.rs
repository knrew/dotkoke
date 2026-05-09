//! 設定ファイル(toml)から設定を読み込む．

use std::{
    fs, io,
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

        let dotfiles_dir =
            canonicalize_existing_dir(dotfiles_dir, "invalid dotfiles directory in config")?;

        let home_dir = home_dir
            .canonicalize()
            .with_context(|| format!("invalid home directory in config: {}", home_dir.display()))?;

        if !home_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", home_dir.display()));
        }

        let backup_root_dir =
            resolve_optional_dir(backup_dir, "invalid backup directory in config")?;

        let dotfiles_home_dir = dotfiles_dir
            .join("home")
            .canonicalize()
            .with_context(|| format!("invalid path: {}/home", dotfiles_dir.display()))?;

        if !dotfiles_home_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", dotfiles_home_dir.display()));
        }

        validate_home_and_dotfiles_home(&home_dir, &dotfiles_home_dir)?;

        let config = Config {
            dotfiles_dir,
            home_dir,
            backup_root_dir,
            dotfiles_home_dir,
        };

        Ok(config)
    }

    pub fn fallback(home_dir: impl AsRef<Path>) -> Result<Self> {
        let home_dir = home_dir.as_ref();
        let home_dir = home_dir
            .canonicalize()
            .with_context(|| format!("invalid fallback home directory: {}", home_dir.display()))?;

        if !home_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", home_dir.display()));
        }

        let dotfiles_dir = canonicalize_existing_dir(
            home_dir.join(".dotfiles"),
            "invalid fallback dotfiles directory",
        )?;

        let dotfiles_home_dir = dotfiles_dir
            .join("home")
            .canonicalize()
            .with_context(|| format!("invalid path: {}/home", dotfiles_dir.display()))?;

        if !dotfiles_home_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", dotfiles_home_dir.display()));
        }

        validate_home_and_dotfiles_home(&home_dir, &dotfiles_home_dir)?;

        let backup_root_dir = home_dir.join(".backup_dotfiles");
        if backup_root_dir.exists() && !backup_root_dir.is_dir() {
            return Err(anyhow!("{} is not directory.", backup_root_dir.display()));
        }

        Ok(Config {
            dotfiles_dir,
            home_dir,
            backup_root_dir,
            dotfiles_home_dir,
        })
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

fn canonicalize_existing_dir(path: PathBuf, context: &str) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .with_context(|| format!("{context}: {}", path.display()))?;

    if !path.is_dir() {
        return Err(anyhow!("{} is not directory.", path.display()));
    }

    Ok(path)
}

fn resolve_optional_dir(path: PathBuf, context: &str) -> Result<PathBuf> {
    match fs::symlink_metadata(&path) {
        Ok(_) => canonicalize_existing_dir(path, context),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(path),
        Err(e) => Err(e).with_context(|| format!("{context}: {}", path.display())),
    }
}

fn validate_home_and_dotfiles_home(home_dir: &Path, dotfiles_home_dir: &Path) -> Result<()> {
    if home_dir == dotfiles_home_dir {
        return Err(anyhow!(
            "home directory must not be the same as dotfiles home directory: {}",
            home_dir.display()
        ));
    }

    Ok(())
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
    fn reads_config_with_missing_backup_dir() {
        let root = TempDir::new().unwrap();
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let home_dir = root.path().join("home");
        let backup_dir = root.path().join("backup");
        let config_path = root.path().join("dotkoke.toml");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();
        fs::write(
            &config_path,
            format!(
                "[general]\ndotfiles = {:?}\nhome = {:?}\nbackup_dir = {:?}\n",
                dotfiles_dir, home_dir, backup_dir
            ),
        )
        .unwrap();

        let config = Config::read(config_path).unwrap();

        assert!(!backup_dir.exists());
        assert_eq!(config.backup_root_dir(), backup_dir);
    }

    #[test]
    fn rejects_file_backup_dir() {
        let root = TempDir::new().unwrap();
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let home_dir = root.path().join("home");
        let backup_dir = root.path().join("backup");
        let config_path = root.path().join("dotkoke.toml");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();
        fs::write(&backup_dir, "backup").unwrap();
        fs::write(
            &config_path,
            format!(
                "[general]\ndotfiles = {:?}\nhome = {:?}\nbackup_dir = {:?}\n",
                dotfiles_dir, home_dir, backup_dir
            ),
        )
        .unwrap();

        let err = Config::read(config_path).unwrap_err().to_string();

        assert!(err.contains("is not directory."));
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

    #[test]
    fn builds_fallback_config_from_home() {
        let root = TempDir::new().unwrap();
        let home_dir = root.path().join("home");
        let dotfiles_dir = home_dir.join(".dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();

        let config = Config::fallback(&home_dir).unwrap();

        assert_eq!(config.home_dir(), home_dir.canonicalize().unwrap());
        assert_eq!(config.dotfiles_dir(), dotfiles_dir.canonicalize().unwrap());
        assert_eq!(
            config.dotfiles_home_dir(),
            dotfiles_home_dir.canonicalize().unwrap()
        );
        assert_eq!(
            config.backup_root_dir(),
            home_dir.canonicalize().unwrap().join(".backup_dotfiles")
        );
    }

    #[test]
    fn fallback_requires_dotfiles_home_dir() {
        let root = TempDir::new().unwrap();
        let home_dir = root.path().join("home");

        fs::create_dir_all(&home_dir).unwrap();

        assert!(Config::fallback(&home_dir).is_err());
    }

    #[test]
    fn fallback_allows_missing_backup_dir() {
        let root = TempDir::new().unwrap();
        let home_dir = root.path().join("home");
        let dotfiles_home_dir = home_dir.join(".dotfiles/home");
        let backup_dir = home_dir.join(".backup_dotfiles");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();

        let config = Config::fallback(&home_dir).unwrap();

        assert!(!backup_dir.exists());
        assert_eq!(config.backup_root_dir(), backup_dir);
    }

    #[test]
    fn fallback_rejects_file_backup_dir() {
        let root = TempDir::new().unwrap();
        let home_dir = root.path().join("home");
        let dotfiles_home_dir = home_dir.join(".dotfiles/home");
        let backup_dir = home_dir.join(".backup_dotfiles");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::write(&backup_dir, "backup").unwrap();

        let err = Config::fallback(&home_dir).unwrap_err().to_string();

        assert!(err.contains("is not directory."));
    }

    #[test]
    fn rejects_home_equal_to_dotfiles_home() {
        let root = TempDir::new().unwrap();
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let backup_dir = root.path().join("backup");
        let config_path = root.path().join("dotkoke.toml");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(
            &config_path,
            format!(
                "[general]\ndotfiles = {:?}\nhome = {:?}\nbackup_dir = {:?}\n",
                dotfiles_dir, dotfiles_home_dir, backup_dir
            ),
        )
        .unwrap();

        let err = Config::read(config_path).unwrap_err().to_string();

        assert!(err.contains("must not be the same"));
    }
}
