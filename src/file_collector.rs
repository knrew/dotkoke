use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::file_kind::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedFiles {
    pub files: Vec<PathBuf>,
    pub links: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub collection_errors: Vec<String>,
}

/// 指定したパス以下を再帰的に探索し，通常ファイルとシンボリックリンク(壊れたリンクを含む)を収集する．
///
/// # 引数
///
/// - `path`: 探索を開始するディレクトリまたはファイルのパス
///
/// # 返り値
///
/// `CollectedFiles`:
/// - `files`: 通常ファイルのパス一覧
/// - `links`: シンボリックリンクのパス一覧
/// - `warnings`: 無視可能な entry の警告一覧
/// - `collection_errors`: 不完全な収集につながる警告一覧
///
/// # NOTE
/// - 引数で指定したパスが `files` または `links` に入る可能性がある．
/// - シンボリックリンク，通常ファイル，ディレクトリのどれでもないパスは
///   警告として収集して無視する．
/// - ディレクトリへのシンボリックリンクは辿らない．
pub fn collect_files_and_links(path: impl AsRef<Path>) -> Result<CollectedFiles> {
    let mut files = vec![];
    let mut links = vec![];
    let mut warnings = vec![];
    let mut collection_errors = vec![];

    let mut stack = vec![path.as_ref().to_path_buf()];

    while let Some(path) = stack.pop() {
        match file_kind(&path) {
            FileKind::Symlink => {
                // 壊れたリンクも収集．
                links.push(path);
            }
            FileKind::File => {
                files.push(path);
            }
            FileKind::Dir => match fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(e) => stack.push(e.path()),
                            Err(e) => collection_errors.push(format!(
                                "failed to read entry in {}: {}",
                                path.display(),
                                e
                            )),
                        }
                    }
                }
                Err(e) => {
                    return Err(e)
                        .with_context(|| format!("failed to read directory: {}", path.display()));
                }
            },
            FileKind::Unknown => {
                warnings.push(format!("unknown file type: {}", path.display()));
            }
            FileKind::Error => {
                collection_errors.push(format!("cannot determine file kind of {}", path.display()));
            }
            FileKind::NotFound => {
                collection_errors.push(format!("not found: {}", path.display()));
            }
        }
    }

    files.sort_unstable();
    files.dedup();
    links.sort_unstable();
    links.dedup();
    warnings.sort_unstable();
    warnings.dedup();
    collection_errors.sort_unstable();
    collection_errors.dedup();

    Ok(CollectedFiles {
        files,
        links,
        warnings,
        collection_errors,
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::symlink, path::Path, process::Command};

    use tempfile::TempDir;

    use super::*;

    fn make_fifo(path: &Path) {
        let status = Command::new("mkfifo").arg(path).status().unwrap();
        assert!(status.success());
    }

    #[test]
    fn collects_regular_files_and_symlinks_without_following_symlinks() {
        let root = TempDir::new().unwrap();
        let base = root.path().join("home");
        let nested = base.join(".config/app");
        let file = base.join(".zshrc");
        let nested_file = nested.join("config.toml");
        let symlink_path = base.join("linked");

        fs::create_dir_all(&nested).unwrap();
        fs::write(&file, "zsh").unwrap();
        fs::write(&nested_file, "config").unwrap();
        symlink(&nested_file, &symlink_path).unwrap();

        let collected = collect_files_and_links(&base).unwrap();

        assert_eq!(collected.files, vec![nested_file, file]);
        assert_eq!(collected.links, vec![symlink_path]);
        assert!(collected.warnings.is_empty());
        assert!(collected.collection_errors.is_empty());
    }

    #[test]
    fn collects_broken_symlink_as_link() {
        let root = TempDir::new().unwrap();
        let base = root.path().join("home");
        let symlink_path = base.join("broken");

        fs::create_dir_all(&base).unwrap();
        symlink(base.join("missing"), &symlink_path).unwrap();

        let collected = collect_files_and_links(&base).unwrap();

        assert!(collected.files.is_empty());
        assert_eq!(collected.links, vec![symlink_path]);
        assert!(collected.warnings.is_empty());
        assert!(collected.collection_errors.is_empty());
    }

    #[test]
    fn collects_unknown_as_warning() {
        let root = TempDir::new().unwrap();
        let base = root.path().join("home");
        let fifo = base.join("app.fifo");

        fs::create_dir_all(&base).unwrap();
        make_fifo(&fifo);

        let collected = collect_files_and_links(&base).unwrap();

        assert!(collected.files.is_empty());
        assert!(collected.links.is_empty());
        assert_eq!(
            collected.warnings,
            vec![format!("unknown file type: {}", fifo.display())]
        );
        assert!(collected.collection_errors.is_empty());
    }

    #[test]
    fn collects_not_found_as_collection_error() {
        let root = TempDir::new().unwrap();
        let missing = root.path().join("missing");

        let collected = collect_files_and_links(&missing).unwrap();

        assert!(collected.files.is_empty());
        assert!(collected.links.is_empty());
        assert!(collected.warnings.is_empty());
        assert_eq!(
            collected.collection_errors,
            vec![format!("not found: {}", missing.display())]
        );
    }
}
