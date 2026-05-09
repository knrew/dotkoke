use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::{
    action::{Action, ExecutionMode, execute_actions},
    file_kind::{FileKind, exists, file_kind},
    install::CommandContext,
    paths::PathResolver,
};

pub fn add(context: &CommandContext, path: impl AsRef<Path>, mode: ExecutionMode) -> Result<()> {
    let actions = plan_add(context, path)?;
    execute_actions(&actions, mode)
}

pub fn plan_add(context: &CommandContext, path: impl AsRef<Path>) -> Result<Vec<Action>> {
    let path = path.as_ref();

    match file_kind(path) {
        FileKind::Symlink => {
            return Ok(vec![Action::Warn {
                message: format!("{} is a symlink. skipped.", path.display()),
            }]);
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

    if !path.starts_with(context.config().home_dir()) {
        return Err(anyhow!(
            "{} is not in {}.",
            path.display(),
            context.config().home_dir().display()
        ));
    }

    // dotfiles管理下のファイルは，取り込み対象にしない．
    if path.starts_with(context.config().dotfiles_home_dir()) {
        return Err(anyhow!(
            "{} is in {}.",
            path.display(),
            context.config().dotfiles_home_dir().display()
        ));
    }

    let to = PathResolver::new(context.config()).managed_path(&path)?;

    if exists(&to) {
        return Ok(vec![Action::Warn {
            message: format!("{} already exists. skipped.", to.display()),
        }]);
    }

    Ok(vec![Action::CopyFile { from: path, to }])
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
            Config::from_parts(dotfiles_dir, home_dir, backup_root_dir, dotfiles_home_dir).unwrap(),
            backup_dir,
        )
    }

    #[test]
    fn plans_home_file_copy_to_managed_path() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().home_dir().join(".zshrc");
        let target = context.config().dotfiles_home_dir().join(".zshrc");

        fs::write(&source, "local").unwrap();

        assert_eq!(
            plan_add(&context, &source).unwrap(),
            vec![Action::CopyFile {
                from: source,
                to: target,
            }]
        );
    }

    #[test]
    fn skips_symlink() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().home_dir().join(".zshrc");
        let link = context.config().home_dir().join(".zshenv");

        fs::write(&source, "local").unwrap();
        symlink(&source, &link).unwrap();

        assert!(matches!(
            plan_add(&context, &link).unwrap().as_slice(),
            [Action::Warn { .. }]
        ));
    }

    #[test]
    fn rejects_directory() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().home_dir().join(".config");

        fs::create_dir_all(&source).unwrap();

        assert!(plan_add(&context, &source).is_err());
    }

    #[test]
    fn rejects_not_found_path() {
        let root = TempDir::new().unwrap();
        let context = context(&root);
        let source = context.config().home_dir().join(".missing");

        let err = plan_add(&context, &source).unwrap_err().to_string();

        assert!(err.contains("does not exist"));
    }
}
