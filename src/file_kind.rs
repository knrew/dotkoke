use std::{fs, io, path::Path};

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Symlink,
    File,
    Dir,

    // 不明な(上記以外の)ファイルタイプ．
    // 存在はする．
    Unknown,

    // 存在しないパス．
    NotFound,

    // 上記のどれにも当てはまらない場合．
    Error,
}

pub fn file_kind(path: impl AsRef<Path>) -> FileKind {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            let ft = meta.file_type();
            if ft.is_symlink() {
                FileKind::Symlink
            } else if ft.is_dir() {
                FileKind::Dir
            } else if ft.is_file() {
                FileKind::File
            } else {
                FileKind::Unknown
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileKind::NotFound,
        Err(_) => FileKind::Error,
    }
}

pub fn is_symlink(path: impl AsRef<Path>) -> bool {
    matches!(file_kind(path), FileKind::Symlink)
}

pub fn is_file(path: impl AsRef<Path>) -> bool {
    matches!(file_kind(path), FileKind::File)
}

pub fn exists(path: impl AsRef<Path>) -> bool {
    !matches!(file_kind(path), FileKind::NotFound)
}

/// `path` が壊れたシンボリックリンクなら true を返す．
///
/// リンク先の `NotFound` だけを broken と扱い，権限エラーなどの判定不能状態は
/// 呼び出し側へエラーとして返す．
pub fn broken_link_status(path: impl AsRef<Path>) -> Result<bool> {
    let path = path.as_ref();
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => match fs::metadata(path) {
            Ok(_) => Ok(false),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(e) => {
                Err(e).with_context(|| format!("failed to resolve symlink: {}", path.display()))
            }
        },
        Ok(_) => Ok(false),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("failed to inspect symlink: {}", path.display())),
    }
}

/// `link` が symlink で，解決先パスが `target` の canonical path と一致するなら true を返す．
///
/// inode 同一性ではなくパス同一性で判定するため，管理元と同じ inode の hard link を
/// 指す symlink は正しいリンクとして扱わない．
pub fn is_symlink_pointing_to(link: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<bool> {
    let link = link.as_ref();
    let target = target.as_ref();

    match file_kind(link) {
        FileKind::Symlink => {}
        FileKind::File | FileKind::Dir | FileKind::Unknown | FileKind::NotFound => {
            return Ok(false);
        }
        FileKind::Error => {
            return Err(anyhow!("cannot determine file kind of {}.", link.display()));
        }
    };

    let raw_destination = fs::read_link(link)
        .with_context(|| format!("failed to read symlink: {}", link.display()))?;

    let destination_abs = if raw_destination.is_absolute() {
        raw_destination
    } else {
        link.parent()
            .expect("link should have parent")
            .join(raw_destination)
    };

    let destination = match destination_abs.canonicalize() {
        Ok(path) => path,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(e).with_context(|| {
                format!(
                    "failed to canonicalize symlink destination: {}",
                    destination_abs.display()
                )
            });
        }
    };
    let target = match target.canonicalize() {
        Ok(path) => path,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to canonicalize target: {}", target.display()));
        }
    };

    Ok(destination == target)
}
