use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::file_operations::{
    copy, create_symlink, remove_file, remove_symlink, rename_without_overwrite,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Real,
    DryRun,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionOutput {
    pub show_skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Warn { message: String },
    SkipAlreadyLinked { from: PathBuf, to: PathBuf },
    CreateSymlink { from: PathBuf, to: PathBuf },
    BackupPath { from: PathBuf, to: PathBuf },
    RemoveSymlink { path: PathBuf },
    CopyFile { from: PathBuf, to: PathBuf },
    RemoveManagedFile { path: PathBuf },
}

pub fn execute_actions(actions: &[Action], mode: ExecutionMode) -> Result<()> {
    execute_actions_with_output(actions, mode, ActionOutput::default())
}

pub fn execute_actions_with_output(
    actions: &[Action],
    mode: ExecutionMode,
    output: ActionOutput,
) -> Result<()> {
    let mut completed_backups = Vec::new();

    for action in actions {
        match mode {
            ExecutionMode::Real => {
                execute_real(action, output)
                    .with_context(|| execution_error_context(action, &completed_backups))?;

                if let Action::BackupPath { from, to } = action {
                    completed_backups.push((from.clone(), to.clone()));
                }
            }
            ExecutionMode::DryRun => execute_dry_run(action, output),
        }
    }

    Ok(())
}

fn execute_real(action: &Action, output: ActionOutput) -> Result<()> {
    match action {
        Action::Warn { message } => {
            print_warning(message);
        }
        Action::SkipAlreadyLinked { .. } => {}
        Action::CreateSymlink { from, to } => {
            create_symlink(from, to)?;
        }
        Action::BackupPath { from, to } => {
            rename_without_overwrite(from, to)?;
        }
        Action::RemoveSymlink { path } => {
            remove_symlink(path)?;
        }
        Action::CopyFile { from, to } => {
            copy(from, to)?;
        }
        Action::RemoveManagedFile { path } => {
            remove_file(path)?;
        }
    }

    if let Some(message) = stdout_message(action, ExecutionMode::Real, output) {
        println!("{message}");
    }

    Ok(())
}

fn execute_dry_run(action: &Action, output: ActionOutput) {
    if let Action::Warn { message } = action {
        print_warning(message);
    }

    if let Some(message) = stdout_message(action, ExecutionMode::DryRun, output) {
        println!("{message}");
    }
}

fn print_warning(message: &str) {
    eprintln!("[warning] {message}");
}

fn stdout_message(action: &Action, mode: ExecutionMode, output: ActionOutput) -> Option<String> {
    match action {
        Action::SkipAlreadyLinked { from, to } => output.show_skipped.then(|| {
            format!(
                "skipped (already linked): {} -> {}",
                from.display(),
                to.display()
            )
        }),
        Action::CreateSymlink { from, to } => match mode {
            ExecutionMode::Real => Some(format!(
                "created link: {} -> {}",
                from.display(),
                to.display()
            )),
            ExecutionMode::DryRun => Some(format!(
                "[dry-run] ln -s {} -> {}",
                from.display(),
                to.display()
            )),
        },
        Action::BackupPath { from, to } => match mode {
            ExecutionMode::Real => {
                Some(format!("backed up: {} -> {}", from.display(), to.display()))
            }
            ExecutionMode::DryRun => Some(format!(
                "[dry-run] mv {} -> {}",
                from.display(),
                to.display()
            )),
        },
        Action::RemoveSymlink { path } => match mode {
            ExecutionMode::Real => Some(format!("removed symlink: {}", path.display())),
            ExecutionMode::DryRun => Some(format!("[dry-run] rm (symlink) {}", path.display())),
        },
        Action::CopyFile { from, to } => match mode {
            ExecutionMode::Real => Some(format!("copied: {} -> {}", from.display(), to.display())),
            ExecutionMode::DryRun => Some(format!(
                "[dry-run] cp {} -> {}",
                from.display(),
                to.display()
            )),
        },
        Action::RemoveManagedFile { path } => match mode {
            ExecutionMode::Real => Some(format!("removed managed file: {}", path.display())),
            ExecutionMode::DryRun => Some(format!("[dry-run] rm {}", path.display())),
        },
        Action::Warn { .. } => None,
    }
}

fn execution_error_context(action: &Action, completed_backups: &[(PathBuf, PathBuf)]) -> String {
    let mut context = format!("failed to execute action: {}", action_description(action));

    if !completed_backups.is_empty() {
        let backups = completed_backups
            .iter()
            .map(|(from, to)| format!("{} -> {}", from.display(), to.display()))
            .collect::<Vec<_>>()
            .join(", ");

        context.push_str(&format!(". completed backup(s): {backups}"));
    }

    context
}

fn action_description(action: &Action) -> String {
    match action {
        Action::Warn { message } => format!("warn: {message}"),
        Action::SkipAlreadyLinked { from, to } => {
            format!(
                "skip already linked: {} -> {}",
                from.display(),
                to.display()
            )
        }
        Action::CreateSymlink { from, to } => {
            format!("create symlink: {} -> {}", from.display(), to.display())
        }
        Action::BackupPath { from, to } => {
            format!("backup path: {} -> {}", from.display(), to.display())
        }
        Action::RemoveSymlink { path } => format!("remove symlink: {}", path.display()),
        Action::CopyFile { from, to } => {
            format!("copy file: {} -> {}", from.display(), to.display())
        }
        Action::RemoveManagedFile { path } => format!("remove managed file: {}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn hides_skip_message_by_default() {
        let action = Action::SkipAlreadyLinked {
            from: PathBuf::from("/dotfiles/home/.zshrc"),
            to: PathBuf::from("/home/user/.zshrc"),
        };

        assert_eq!(
            stdout_message(&action, ExecutionMode::Real, ActionOutput::default()),
            None
        );
    }

    #[test]
    fn shows_skip_message_when_enabled() {
        let action = Action::SkipAlreadyLinked {
            from: PathBuf::from("/dotfiles/home/.zshrc"),
            to: PathBuf::from("/home/user/.zshrc"),
        };

        assert_eq!(
            stdout_message(
                &action,
                ExecutionMode::Real,
                ActionOutput { show_skipped: true }
            ),
            Some(
                "skipped (already linked): /dotfiles/home/.zshrc -> /home/user/.zshrc".to_string()
            )
        );
    }

    #[test]
    fn shows_dry_run_skip_message_when_enabled() {
        let action = Action::SkipAlreadyLinked {
            from: PathBuf::from("/dotfiles/home/.zshrc"),
            to: PathBuf::from("/home/user/.zshrc"),
        };

        assert_eq!(
            stdout_message(
                &action,
                ExecutionMode::DryRun,
                ActionOutput { show_skipped: true }
            ),
            Some(
                "skipped (already linked): /dotfiles/home/.zshrc -> /home/user/.zshrc".to_string()
            )
        );
    }

    #[test]
    fn keeps_non_skip_messages_visible_by_default() {
        let action = Action::CreateSymlink {
            from: PathBuf::from("/dotfiles/home/.zshrc"),
            to: PathBuf::from("/home/user/.zshrc"),
        };

        assert_eq!(
            stdout_message(&action, ExecutionMode::DryRun, ActionOutput::default()),
            Some("[dry-run] ln -s /dotfiles/home/.zshrc -> /home/user/.zshrc".to_string())
        );
    }

    #[test]
    fn generates_stdout_messages_in_one_place() {
        let cases = [
            (
                Action::CreateSymlink {
                    from: PathBuf::from("/dotfiles/home/.zshrc"),
                    to: PathBuf::from("/home/user/.zshrc"),
                },
                ExecutionMode::Real,
                "created link: /dotfiles/home/.zshrc -> /home/user/.zshrc",
            ),
            (
                Action::CreateSymlink {
                    from: PathBuf::from("/dotfiles/home/.zshrc"),
                    to: PathBuf::from("/home/user/.zshrc"),
                },
                ExecutionMode::DryRun,
                "[dry-run] ln -s /dotfiles/home/.zshrc -> /home/user/.zshrc",
            ),
            (
                Action::BackupPath {
                    from: PathBuf::from("/home/user/.zshrc"),
                    to: PathBuf::from("/backup/.zshrc"),
                },
                ExecutionMode::Real,
                "backed up: /home/user/.zshrc -> /backup/.zshrc",
            ),
            (
                Action::BackupPath {
                    from: PathBuf::from("/home/user/.zshrc"),
                    to: PathBuf::from("/backup/.zshrc"),
                },
                ExecutionMode::DryRun,
                "[dry-run] mv /home/user/.zshrc -> /backup/.zshrc",
            ),
            (
                Action::RemoveSymlink {
                    path: PathBuf::from("/home/user/.zshrc"),
                },
                ExecutionMode::Real,
                "removed symlink: /home/user/.zshrc",
            ),
            (
                Action::RemoveSymlink {
                    path: PathBuf::from("/home/user/.zshrc"),
                },
                ExecutionMode::DryRun,
                "[dry-run] rm (symlink) /home/user/.zshrc",
            ),
            (
                Action::CopyFile {
                    from: PathBuf::from("/home/user/.zshrc"),
                    to: PathBuf::from("/dotfiles/home/.zshrc"),
                },
                ExecutionMode::Real,
                "copied: /home/user/.zshrc -> /dotfiles/home/.zshrc",
            ),
            (
                Action::CopyFile {
                    from: PathBuf::from("/home/user/.zshrc"),
                    to: PathBuf::from("/dotfiles/home/.zshrc"),
                },
                ExecutionMode::DryRun,
                "[dry-run] cp /home/user/.zshrc -> /dotfiles/home/.zshrc",
            ),
            (
                Action::RemoveManagedFile {
                    path: PathBuf::from("/dotfiles/home/.zshrc"),
                },
                ExecutionMode::Real,
                "removed managed file: /dotfiles/home/.zshrc",
            ),
            (
                Action::RemoveManagedFile {
                    path: PathBuf::from("/dotfiles/home/.zshrc"),
                },
                ExecutionMode::DryRun,
                "[dry-run] rm /dotfiles/home/.zshrc",
            ),
        ];

        for (action, mode, expected) in cases {
            assert_eq!(
                stdout_message(&action, mode, ActionOutput::default()),
                Some(expected.to_string())
            );
        }
    }

    #[test]
    fn create_symlink_failure_context_contains_action_paths() {
        let root = TempDir::new().unwrap();
        let source = root.path().join("dotfiles/home/.zshrc");
        let blocking_parent = root.path().join("home/.config");
        let target = blocking_parent.join("app.toml");

        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(blocking_parent.parent().unwrap()).unwrap();
        fs::write(&source, "managed").unwrap();
        fs::write(&blocking_parent, "blocking").unwrap();

        let err = execute_actions(
            &[Action::CreateSymlink {
                from: source.clone(),
                to: target.clone(),
            }],
            ExecutionMode::Real,
        )
        .unwrap_err();

        let message = format!("{err:#}");
        assert!(message.contains("failed to execute action: create symlink"));
        assert!(message.contains(&source.display().to_string()));
        assert!(message.contains(&target.display().to_string()));
    }

    #[test]
    fn failure_context_contains_completed_backup_paths() {
        let root = TempDir::new().unwrap();
        let source = root.path().join("dotfiles/home/.zshrc");
        let target = root.path().join("home/.zshrc");
        let backup = root.path().join("backup/.zshrc");
        let blocking_parent = root.path().join("home/.config");
        let failing_target = blocking_parent.join("app.toml");

        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&source, "managed").unwrap();
        fs::write(&target, "local").unwrap();
        fs::write(&blocking_parent, "blocking").unwrap();

        let err = execute_actions(
            &[
                Action::BackupPath {
                    from: target.clone(),
                    to: backup.clone(),
                },
                Action::CreateSymlink {
                    from: source,
                    to: failing_target,
                },
            ],
            ExecutionMode::Real,
        )
        .unwrap_err();

        let message = format!("{err:#}");
        assert!(message.contains("completed backup(s):"));
        assert!(message.contains(&target.display().to_string()));
        assert!(message.contains(&backup.display().to_string()));
    }
}
