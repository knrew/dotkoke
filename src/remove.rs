use std::{fs, io, path::Path};

use anyhow::{Context, Result, anyhow};

use crate::{
    action::{Action, ExecutionMode, execute_actions},
    file_kind::{FileKind, broken_link_status, file_kind, is_symlink_pointing_to},
    install::CommandContext,
    paths::PathResolver,
};

pub fn remove(context: &CommandContext, path: impl AsRef<Path>, mode: ExecutionMode) -> Result<()> {
    let actions = plan_remove(context, path)?;
    execute_actions(&actions, mode)
}

pub fn plan_remove(context: &CommandContext, path: impl AsRef<Path>) -> Result<Vec<Action>> {
    let path = path.as_ref();

    match file_kind(path) {
        FileKind::Symlink => {
            return Err(anyhow!("{} is a symlink.", path.display()));
        }
        FileKind::Dir => {
            return Err(anyhow!("{} is not a file.", path.display()));
        }
        FileKind::Unknown => {
            return Err(anyhow!("{} is an unknown file type.", path.display()));
        }
        FileKind::Error => {
            return Err(anyhow!("cannot determine file kind of {}.", path.display()));
        }
        FileKind::NotFound => {
            return Err(anyhow!("{} does not exist.", path.display()));
        }
        FileKind::File => {}
    }

    let path = path
        .canonicalize()
        .with_context(|| format!("invalid path: {}", path.display()))?;

    if !path.starts_with(context.config().dotfiles_home_dir()) {
        return Err(anyhow!(
            "{} is not in {}.",
            path.display(),
            context.config().dotfiles_home_dir().display()
        ));
    }

    let to = PathResolver::new(context.config()).install_path(&path)?;
    let mut actions = Vec::new();

    if should_remove_home_symlink(&to, &path)? {
        actions.push(Action::RemoveSymlink { path: to });
    }

    actions.push(Action::RemoveManagedFile { path });

    Ok(actions)
}

fn should_remove_home_symlink(to: &Path, managed: &Path) -> Result<bool> {
    match fs::symlink_metadata(to) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Ok(is_symlink_pointing_to(to, managed)? || broken_link_status(to)?)
        }
        Ok(_) => Ok(false),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(false)
        }
        Err(e) => Err(e).with_context(|| format!("failed to inspect symlink: {}", to.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::symlink};

    use tempfile::TempDir;

    use super::*;
    use crate::Config;

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
    fn plans_managed_file_removal_with_home_symlink() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let managed = context.config().dotfiles_home_dir().join(".zshrc");
        let home = context.config().home_dir().join(".zshrc");

        fs::write(&managed, "managed").unwrap();
        symlink(&managed, &home).unwrap();

        assert_eq!(
            plan_remove(&context, &managed).unwrap(),
            vec![
                Action::RemoveSymlink { path: home },
                Action::RemoveManagedFile { path: managed },
            ]
        );
    }

    #[test]
    fn keeps_unrelated_home_symlink() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let managed = context.config().dotfiles_home_dir().join(".zshrc");
        let other = context.config().dotfiles_home_dir().join(".zshenv");
        let home = context.config().home_dir().join(".zshrc");

        fs::write(&managed, "managed").unwrap();
        fs::write(&other, "other").unwrap();
        symlink(&other, &home).unwrap();

        assert_eq!(
            plan_remove(&context, &managed).unwrap(),
            vec![Action::RemoveManagedFile { path: managed }]
        );
    }

    #[test]
    fn removes_broken_home_symlink() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let managed = context.config().dotfiles_home_dir().join(".zshrc");
        let home = context.config().home_dir().join(".zshrc");

        fs::write(&managed, "managed").unwrap();
        symlink(context.config().home_dir().join(".missing"), &home).unwrap();

        assert_eq!(
            plan_remove(&context, &managed).unwrap(),
            vec![
                Action::RemoveSymlink { path: home },
                Action::RemoveManagedFile { path: managed },
            ]
        );
    }

    #[test]
    fn removes_home_symlink_with_not_a_directory_destination() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let managed = context.config().dotfiles_home_dir().join(".zshrc");
        let home = context.config().home_dir().join(".zshrc");
        let blocking_file = root.path().join("blocking-file");

        fs::write(&managed, "managed").unwrap();
        fs::write(&blocking_file, "blocking").unwrap();
        symlink(blocking_file.join("child"), &home).unwrap();

        assert_eq!(
            plan_remove(&context, &managed).unwrap(),
            vec![
                Action::RemoveSymlink { path: home },
                Action::RemoveManagedFile { path: managed },
            ]
        );
    }

    #[test]
    fn plans_managed_file_removal_when_home_parent_is_not_a_directory() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let managed = context
            .config()
            .dotfiles_home_dir()
            .join(".config/app/config.toml");
        let blocking_file = context.config().home_dir().join(".config");

        fs::create_dir_all(managed.parent().unwrap()).unwrap();
        fs::write(&managed, "managed").unwrap();
        fs::write(&blocking_file, "blocking").unwrap();

        assert_eq!(
            plan_remove(&context, &managed).unwrap(),
            vec![Action::RemoveManagedFile { path: managed }]
        );
    }

    #[test]
    fn rejects_not_found_path() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let managed = context.config().dotfiles_home_dir().join(".missing");

        let err = plan_remove(&context, &managed).unwrap_err().to_string();

        assert!(err.contains("does not exist"));
    }
}
