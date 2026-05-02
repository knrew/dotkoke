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
/// 3. `$HOME/.config/dotkoke_config.toml`
/// 4. `$HOME/.config/dotkoke/dotkoke_config.toml`
fn find_config_file_path(cli: &Cli) -> Result<PathBuf> {
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

    if let Some(config) = &cli.config_file {
        ensure_config_file(config)?;
        return Ok(config.clone());
    }

    if let Some(config) = env::var("DOTKOKE_CONFIG").ok().map(PathBuf::from) {
        ensure_config_file(&config)?;
        return Ok(config);
    }

    // $HOME/.config/dotkoke_config.toml
    if let Some(home) = env::var("HOME").ok().map(PathBuf::from) {
        for config in [
            home.join(".config/dotkoke_config.toml"),
            home.join(".config/dotkoke/dotkoke_config.toml"),
        ] {
            if let Some(path) = resolve_optional_config(config)? {
                return Ok(path);
            }
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
        Command::Install { dry_run } => {
            let context = CommandContext::new(config);
            install(&context, execution_mode(dry_run))?;
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
