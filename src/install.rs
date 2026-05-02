use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;

use crate::{
    action::{Action, ActionOutput, ExecutionMode, execute_actions_with_output},
    config::Config,
    file_collector::collect_files_and_links,
    file_kind::{FileKind, file_kind, is_symlink_pointing_to},
    paths::PathResolver,
};

#[derive(Debug, Clone)]
pub struct CommandContext {
    config: Config,
    backup_dir: PathBuf,
}

impl CommandContext {
    pub fn new(config: Config) -> Self {
        let timestamp = Local::now().format("%Y%m%d_%H%M").to_string();
        let backup_dir = config.backup_dir_for_timestamp(&timestamp);

        if backup_dir.is_dir() {
            eprintln!("[warning] {} already exists.", backup_dir.display());
        }

        Self { config, backup_dir }
    }

    pub fn with_backup_dir(config: Config, backup_dir: PathBuf) -> Self {
        Self { config, backup_dir }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}

pub fn install(context: &CommandContext, mode: ExecutionMode) -> Result<()> {
    install_with_output(context, mode, ActionOutput::default())
}

pub fn install_with_output(
    context: &CommandContext,
    mode: ExecutionMode,
    output: ActionOutput,
) -> Result<()> {
    let actions = plan_install(context)?;
    execute_actions_with_output(&actions, mode, output)
}

pub fn plan_install(context: &CommandContext) -> Result<Vec<Action>> {
    let resolver = PathResolver::new(context.config());
    let (files, links) = collect_files_and_links(context.config().dotfiles_home_dir())?;
    let mut actions = Vec::new();

    if !links.is_empty() {
        actions.push(Action::Warn {
            message: format!(
                "symlink(s) exist in {} (they will be ignored).",
                context.config().dotfiles_home_dir().display()
            ),
        });
    }

    for from in files {
        let to = resolver.install_path(&from)?;

        if is_symlink_pointing_to(&to, &from) {
            actions.push(Action::SkipAlreadyLinked { from, to });
            continue;
        }

        match file_kind(&to) {
            FileKind::Symlink => {
                actions.push(Action::RemoveSymlink { path: to.clone() });
            }
            FileKind::File | FileKind::Dir | FileKind::Unknown => {
                actions.push(Action::BackupPath {
                    from: to.clone(),
                    to: resolver.backup_path(&to, context.backup_dir())?,
                });
            }
            FileKind::NotFound => {}
            FileKind::Error => {
                actions.push(Action::Warn {
                    message: format!("cannot determine file kind of {} (skipped)", to.display()),
                });
                continue;
            }
        }

        actions.push(Action::CreateSymlink { from, to });
    }

    Ok(actions)
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::symlink};

    use tempfile::TempDir;

    use super::*;

    fn context(root: &TempDir) -> CommandContext {
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let home_dir = root.path().join("home");
        let backup_root_dir = root.path().join("backup");
        let backup_dir = backup_root_dir.join("fixed");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&backup_root_dir).unwrap();

        CommandContext::with_backup_dir(
            Config::from_parts(dotfiles_dir, home_dir, backup_root_dir, dotfiles_home_dir),
            backup_dir,
        )
    }

    #[test]
    fn plans_new_symlink_creation() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "source").unwrap();

        assert_eq!(
            plan_install(&context).unwrap(),
            vec![Action::CreateSymlink {
                from: source,
                to: target,
            }]
        );
    }

    #[test]
    fn plans_existing_file_backup_before_symlink_creation() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();
        fs::write(&target, "local").unwrap();

        assert_eq!(
            plan_install(&context).unwrap(),
            vec![
                Action::BackupPath {
                    from: target.clone(),
                    to: context.backup_dir().join(".zshrc"),
                },
                Action::CreateSymlink {
                    from: source,
                    to: target,
                },
            ]
        );
    }

    #[test]
    fn dry_run_does_not_change_filesystem() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();

        install(&context, ExecutionMode::DryRun).unwrap();

        assert!(source.is_file());
        assert!(!target.exists());
    }

    #[test]
    fn real_install_backs_up_directory() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context
            .config()
            .dotfiles_home_dir()
            .join(".config/app/config.toml");
        let target_dir = context.config().home_dir().join(".config/app/config.toml");

        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "managed").unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        install(&context, ExecutionMode::Real).unwrap();

        assert!(target_dir.is_symlink());
        assert!(
            context
                .backup_dir()
                .join(".config/app/config.toml")
                .is_dir()
        );
    }

    #[test]
    fn plans_existing_correct_symlink_as_skip() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();
        symlink(&source, &target).unwrap();

        assert_eq!(
            plan_install(&context).unwrap(),
            vec![Action::SkipAlreadyLinked {
                from: source,
                to: target,
            }]
        );
    }
}
