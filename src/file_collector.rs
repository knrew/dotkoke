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
/// - `warnings`: 個別 entry の読み取り失敗など，走査を継続できる警告の一覧
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
                            Err(e) => warnings.push(format!(
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
                warnings.push(format!("cannot determine file kind of {}", path.display()));
            }
            FileKind::NotFound => {
                warnings.push(format!("not found: {}", path.display()));
            }
        }
    }

    files.sort_unstable();
    files.dedup();
    links.sort_unstable();
    links.dedup();
    warnings.sort_unstable();
    warnings.dedup();

    Ok(CollectedFiles {
        files,
        links,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::symlink};

    use tempfile::TempDir;

    use super::*;

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
    }

    #[test]
    fn collects_not_found_as_warning() {
        let root = TempDir::new().unwrap();
        let missing = root.path().join("missing");

        let collected = collect_files_and_links(&missing).unwrap();

        assert!(collected.files.is_empty());
        assert!(collected.links.is_empty());
        assert_eq!(
            collected.warnings,
            vec![format!("not found: {}", missing.display())]
        );
    }
}
