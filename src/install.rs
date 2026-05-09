use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
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
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_dir = unique_backup_dir(&config, &timestamp);

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
    let collected = collect_files_and_links(context.config().dotfiles_home_dir())?;
    let mut actions = Vec::new();

    if !collected.warnings.is_empty() {
        return Err(anyhow!(
            "failed to collect all files: {}",
            collected.warnings.join("; ")
        ));
    }

    if !collected.links.is_empty() {
        actions.push(Action::Warn {
            message: format!(
                "symlink(s) exist in {} (they will be ignored).",
                context.config().dotfiles_home_dir().display()
            ),
        });
    }

    for from in collected.files {
        let to = resolver.install_path(&from)?;
        ensure_install_parent_is_safe(context.config(), &to)?;

        if is_symlink_pointing_to(&to, &from)? {
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

fn unique_backup_dir(config: &Config, timestamp: &str) -> PathBuf {
    let backup_dir = config.backup_dir_for_timestamp(timestamp);
    if !backup_dir.exists() {
        return backup_dir;
    }

    for index in 1.. {
        let backup_dir = config.backup_dir_for_timestamp(&format!("{timestamp}-{index}"));
        if !backup_dir.exists() {
            return backup_dir;
        }
    }

    unreachable!("unbounded suffix search should always return a backup directory")
}

fn ensure_install_parent_is_safe(config: &Config, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory.", path.display()))?;

    if !parent.starts_with(config.home_dir()) {
        return Err(anyhow!(
            "{} is not in {}.",
            parent.display(),
            config.home_dir().display()
        ));
    }

    let relative_parent = parent.strip_prefix(config.home_dir())?;
    let mut current = config.home_dir().to_path_buf();

    for component in relative_parent.components() {
        current.push(component.as_os_str());

        match file_kind(&current) {
            FileKind::Dir => {}
            FileKind::NotFound => break,
            FileKind::File => {
                return Err(anyhow!(
                    "{} is a file. cannot create install parent directory.",
                    current.display()
                ));
            }
            FileKind::Symlink => {
                return Err(anyhow!(
                    "{} is a symlink. cannot create install parent directory.",
                    current.display()
                ));
            }
            FileKind::Unknown => {
                return Err(anyhow!(
                    "{} is an unknown file type. cannot create install parent directory.",
                    current.display()
                ));
            }
            FileKind::Error => {
                return Err(anyhow!(
                    "cannot determine file kind of {}.",
                    current.display()
                ));
            }
        }
    }

    Ok(())
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

    fn config(root: &TempDir) -> Config {
        let dotfiles_dir = root.path().join("dotfiles");
        let dotfiles_home_dir = dotfiles_dir.join("home");
        let home_dir = root.path().join("home");
        let backup_root_dir = root.path().join("backup");

        fs::create_dir_all(&dotfiles_home_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();

        Config::from_parts(dotfiles_dir, home_dir, backup_root_dir, dotfiles_home_dir)
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
    fn real_install_creates_missing_backup_root_dir() {
        let root = TempDir::new().unwrap();
        let config = config(&root);
        let backup_dir = config.backup_root_dir().join("fixed");
        let context = CommandContext::with_backup_dir(config, backup_dir.clone());
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();
        fs::write(&target, "local").unwrap();

        install(&context, ExecutionMode::Real).unwrap();

        assert_eq!(
            fs::read_to_string(backup_dir.join(".zshrc")).unwrap(),
            "local"
        );
    }

    #[test]
    fn real_install_does_not_overwrite_existing_backup() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");
        let backup = context.backup_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();
        fs::write(&target, "local1").unwrap();
        install(&context, ExecutionMode::Real).unwrap();
        fs::remove_file(&target).unwrap();
        fs::write(&target, "local2").unwrap();

        let err = install(&context, ExecutionMode::Real)
            .unwrap_err()
            .to_string();

        assert!(err.contains("backup destination already exists"));
        assert_eq!(fs::read_to_string(backup).unwrap(), "local1");
    }

    #[test]
    fn selects_suffixed_backup_dir_when_timestamp_dir_exists() {
        let root = TempDir::new().unwrap();
        let config = config(&root);
        let existing = config.backup_dir_for_timestamp("20260509_230000");

        fs::create_dir_all(&existing).unwrap();

        assert_eq!(
            unique_backup_dir(&config, "20260509_230000"),
            config.backup_dir_for_timestamp("20260509_230000-1")
        );
    }

    #[test]
    fn rejects_file_in_install_parent_path() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context
            .config()
            .dotfiles_home_dir()
            .join(".config/app/config.toml");
        let blocking_parent = context.config().home_dir().join(".config");

        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "managed").unwrap();
        fs::write(&blocking_parent, "local").unwrap();

        let err = plan_install(&context).unwrap_err().to_string();

        assert!(err.contains("cannot create install parent directory"));
    }

    #[test]
    fn rejects_symlink_in_install_parent_path() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context
            .config()
            .dotfiles_home_dir()
            .join(".config/app/config.toml");
        let external = root.path().join("external");
        let linked_parent = context.config().home_dir().join(".config");

        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "managed").unwrap();
        fs::create_dir_all(&external).unwrap();
        symlink(&external, &linked_parent).unwrap();

        let err = plan_install(&context).unwrap_err().to_string();

        assert!(err.contains("cannot create install parent directory"));
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

    #[test]
    fn plans_existing_relative_correct_symlink_as_skip() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();
        symlink("../dotfiles/home/.zshrc", &target).unwrap();

        assert_eq!(
            plan_install(&context).unwrap(),
            vec![Action::SkipAlreadyLinked {
                from: source,
                to: target,
            }]
        );
    }

    #[test]
    fn does_not_skip_symlink_to_hard_link_of_managed_file() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let hard_link = root.path().join("same-inode");
        let target = context.config().home_dir().join(".zshrc");

        fs::write(&source, "managed").unwrap();
        fs::hard_link(&source, &hard_link).unwrap();
        symlink(&hard_link, &target).unwrap();

        assert_eq!(
            plan_install(&context).unwrap(),
            vec![
                Action::RemoveSymlink {
                    path: target.clone()
                },
                Action::CreateSymlink {
                    from: source,
                    to: target,
                },
            ]
        );
    }

    #[test]
    fn plans_unresolvable_home_symlink_destination_for_replacement() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().dotfiles_home_dir().join(".zshrc");
        let target = context.config().home_dir().join(".zshrc");
        let blocking_file = root.path().join("blocking-file");

        fs::write(&source, "managed").unwrap();
        fs::write(&blocking_file, "blocking").unwrap();
        symlink(blocking_file.join("child"), &target).unwrap();

        assert_eq!(
            plan_install(&context).unwrap(),
            vec![
                Action::RemoveSymlink {
                    path: target.clone()
                },
                Action::CreateSymlink {
                    from: source,
                    to: target,
                },
            ]
        );
    }

    #[test]
    fn rejects_collection_warnings() {
        let root = TempDir::new().unwrap();
        let context = context(&root);

        fs::remove_dir(context.config().dotfiles_home_dir()).unwrap();

        let err = plan_install(&context).unwrap_err().to_string();

        assert!(err.contains("failed to collect all files"));
    }
}
