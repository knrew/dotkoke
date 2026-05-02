use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};

use dotkoke::*;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long = "config", global = true)]
    config_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// unimplemented
    Init {},

    /// dotifiles/home以下のファイルのリンクを$HOMEに貼る．
    Install {
        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        show_skipped: bool,
    },

    /// `path`をdotfilesに加え管理対象に加える．
    Add {
        #[arg(long)]
        dry_run: bool,

        path: PathBuf,
    },

    /// `path`をdotfilesから削除し管理対象から外す．
    Remove {
        #[arg(long)]
        dry_run: bool,

        path: PathBuf,
    },

    /// unimplemented
    Clean {},

    /// 管理対象ファイル一覧を表示する．
    List {},

    /// unimplemented
    Status {},
}

/// configを探す．
///
/// 以下の優先順位でconfigを探す．
/// 1. コマンドオプション`--config`で指定されたファイル
/// 2. 環境変数`DOTKOKE_CONFIG`で指定されたファイル
/// 3. `$XDG_CONFIG_HOME/dotkoke/config.toml`
/// 4. `$HOME/.config/dotkoke/config.toml`
fn find_config_file_path(cli: &Cli) -> Result<PathBuf> {
    resolve_config_file_path(
        cli.config_file.clone(),
        env::var_os("DOTKOKE_CONFIG").map(PathBuf::from),
        env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
    )
}

fn resolve_config_file_path(
    cli_config: Option<PathBuf>,
    env_config: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf> {
    fn ensure_config_file(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("{} does not exist.", path.display()));
        }

        if !path.is_file() {
            return Err(anyhow!("{} is not a file.", path.display()));
        }

        Ok(())
    }

    fn resolve_optional_config(path: PathBuf) -> Result<Option<PathBuf>> {
        if !path.exists() {
            return Ok(None);
        }

        if !path.is_file() {
            return Err(anyhow!("{} is not a file.", path.display()));
        }

        Ok(Some(path))
    }

    if let Some(config) = cli_config {
        ensure_config_file(&config)?;
        return Ok(config);
    }

    if let Some(config) = env_config {
        ensure_config_file(&config)?;
        return Ok(config);
    }

    if let Some(xdg_config_home) = xdg_config_home {
        let config = xdg_config_home.join("dotkoke/config.toml");
        if let Some(path) = resolve_optional_config(config)? {
            return Ok(path);
        }
    }

    if let Some(home) = home {
        let config = home.join(".config/dotkoke/config.toml");
        if let Some(path) = resolve_optional_config(config)? {
            return Ok(path);
        }
    }

    Err(anyhow!("config file not found."))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_file_path = find_config_file_path(&cli)?;

    let config = Config::read(config_file_path)?;

    match cli.command {
        Command::Init {} => {
            unimplemented!();
        }
        Command::Install {
            dry_run,
            show_skipped,
        } => {
            let context = CommandContext::new(config);
            install_with_output(
                &context,
                execution_mode(dry_run),
                ActionOutput { show_skipped },
            )?;
        }
        Command::Add { path, dry_run } => {
            let context = CommandContext::new(config);
            add(&context, path, execution_mode(dry_run))?;
        }
        Command::Remove { path, dry_run } => {
            let context = CommandContext::new(config);
            remove(&context, path, execution_mode(dry_run))?;
        }
        Command::Clean {} => {
            unimplemented!();
        }
        Command::List {} => {
            list(&config)?;
        }
        Command::Status {} => {
            unimplemented!();
        }
    }

    Ok(())
}

fn execution_mode(dry_run: bool) -> ExecutionMode {
    if dry_run {
        ExecutionMode::DryRun
    } else {
        ExecutionMode::Real
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    #[test]
    fn config_option_has_highest_priority() {
        let root = TempDir::new().unwrap();
        let cli_config = root.path().join("cli.toml");
        let env_config = root.path().join("env.toml");
        let xdg_config = root.path().join("xdg/dotkoke/config.toml");
        let home_config = root.path().join("home/.config/dotkoke/config.toml");

        touch(&cli_config);
        touch(&env_config);
        touch(&xdg_config);
        touch(&home_config);

        let path = resolve_config_file_path(
            Some(cli_config.clone()),
            Some(env_config),
            Some(root.path().join("xdg")),
            Some(root.path().join("home")),
        )
        .unwrap();

        assert_eq!(path, cli_config);
    }

    #[test]
    fn env_config_has_priority_over_auto_discovery() {
        let root = TempDir::new().unwrap();
        let env_config = root.path().join("env.toml");
        let xdg_config = root.path().join("xdg/dotkoke/config.toml");
        let home_config = root.path().join("home/.config/dotkoke/config.toml");

        touch(&env_config);
        touch(&xdg_config);
        touch(&home_config);

        let path = resolve_config_file_path(
            None,
            Some(env_config.clone()),
            Some(root.path().join("xdg")),
            Some(root.path().join("home")),
        )
        .unwrap();

        assert_eq!(path, env_config);
    }

    #[test]
    fn xdg_config_home_has_priority_over_home_config() {
        let root = TempDir::new().unwrap();
        let xdg_config_home = root.path().join("xdg");
        let home = root.path().join("home");
        let xdg_config = xdg_config_home.join("dotkoke/config.toml");
        let home_config = home.join(".config/dotkoke/config.toml");

        touch(&xdg_config);
        touch(&home_config);

        let path = resolve_config_file_path(None, None, Some(xdg_config_home), Some(home)).unwrap();

        assert_eq!(path, xdg_config);
    }

    #[test]
    fn home_config_is_used_when_xdg_config_home_is_unset() {
        let root = TempDir::new().unwrap();
        let home = root.path().join("home");
        let home_config = home.join(".config/dotkoke/config.toml");

        touch(&home_config);

        let path = resolve_config_file_path(None, None, None, Some(home)).unwrap();

        assert_eq!(path, home_config);
    }

    #[test]
    fn auto_discovery_rejects_directory_config_path() {
        let root = TempDir::new().unwrap();
        let xdg_config = root.path().join("xdg/dotkoke/config.toml");
        fs::create_dir_all(&xdg_config).unwrap();

        let err = resolve_config_file_path(None, None, Some(root.path().join("xdg")), None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("is not a file."));
    }

    #[test]
    fn missing_config_returns_not_found_error() {
        let root = TempDir::new().unwrap();

        let err = resolve_config_file_path(
            None,
            None,
            Some(root.path().join("xdg")),
            Some(root.path().join("home")),
        )
        .unwrap_err()
        .to_string();

        assert_eq!(err, "config file not found.");
    }
}
