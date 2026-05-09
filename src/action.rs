use std::path::PathBuf;

use anyhow::Result;

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
    for action in actions {
        match mode {
            ExecutionMode::Real => execute_real(action, output)?,
            ExecutionMode::DryRun => execute_dry_run(action, output),
        }
    }

    Ok(())
}

fn execute_real(action: &Action, output: ActionOutput) -> Result<()> {
    match action {
        Action::Warn { message } => {
            eprintln!("[warning] {message}");
        }
        Action::SkipAlreadyLinked { .. } => {
            if let Some(message) = stdout_message(action, ExecutionMode::Real, output) {
                println!("{message}");
            }
        }
        Action::CreateSymlink { from, to } => {
            create_symlink(from, to)?;
            println!("created link: {} -> {}", from.display(), to.display());
        }
        Action::BackupPath { from, to } => {
            rename_without_overwrite(from, to)?;
            println!("backed up: {} -> {}", from.display(), to.display());
        }
        Action::RemoveSymlink { path } => {
            remove_symlink(path)?;
            println!("removed symlink: {}", path.display());
        }
        Action::CopyFile { from, to } => {
            copy(from, to)?;
            println!("copied: {} -> {}", from.display(), to.display());
        }
        Action::RemoveManagedFile { path } => {
            remove_file(path)?;
            println!("removed managed file: {}", path.display());
        }
    }

    Ok(())
}

fn execute_dry_run(action: &Action, output: ActionOutput) {
    match action {
        Action::Warn { message } => {
            eprintln!("[warning] {message}");
        }
        Action::SkipAlreadyLinked { .. } => {
            if let Some(message) = stdout_message(action, ExecutionMode::DryRun, output) {
                println!("{message}");
            }
        }
        Action::CreateSymlink { from, to } => {
            println!("[dry-run] ln -s {} -> {}", from.display(), to.display());
        }
        Action::BackupPath { from, to } => {
            println!("[dry-run] mv {} -> {}", from.display(), to.display());
        }
        Action::RemoveSymlink { path } => {
            println!("[dry-run] rm (symlink) {}", path.display());
        }
        Action::CopyFile { from, to } => {
            println!("[dry-run] cp {} -> {}", from.display(), to.display());
        }
        Action::RemoveManagedFile { path } => {
            println!("[dry-run] rm {}", path.display());
        }
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
}
