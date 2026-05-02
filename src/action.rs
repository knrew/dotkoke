use std::path::PathBuf;

use anyhow::Result;

use crate::file_operations::{copy, create_symlink, remove_file, remove_symlink, rename};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Real,
    DryRun,
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
    for action in actions {
        match mode {
            ExecutionMode::Real => execute_real(action)?,
            ExecutionMode::DryRun => execute_dry_run(action),
        }
    }

    Ok(())
}

fn execute_real(action: &Action) -> Result<()> {
    match action {
        Action::Warn { message } => {
            eprintln!("[warning] {message}");
        }
        Action::SkipAlreadyLinked { from, to } => {
            println!(
                "skipped (already linked): {} -> {}",
                from.display(),
                to.display()
            );
        }
        Action::CreateSymlink { from, to } => {
            create_symlink(from, to)?;
            println!("created link: {} -> {}", from.display(), to.display());
        }
        Action::BackupPath { from, to } => {
            rename(from, to)?;
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

fn execute_dry_run(action: &Action) {
    match action {
        Action::Warn { message } => {
            eprintln!("[warning] {message}");
        }
        Action::SkipAlreadyLinked { from, to } => {
            println!(
                "skipped (already linked): {} -> {}",
                from.display(),
                to.display()
            );
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
