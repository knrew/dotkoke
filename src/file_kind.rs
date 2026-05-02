use std::{fs, os::unix::fs::MetadataExt, path::Path};

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

/// `path`が壊れたシンボリックリンクならtrue
pub fn is_broken_link(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => fs::metadata(path).is_err(),
        _ => false,
    }
}

/// `link`がsymlinkでその参照先と`target`が同じ実体を指すならtrue，
/// それ以外の場合false．
pub fn is_symlink_pointing_to(link: impl AsRef<Path>, target: impl AsRef<Path>) -> bool {
    let link = link.as_ref();
    let target = target.as_ref();

    let Ok(raw_destination) = fs::read_link(link) else {
        return false;
    };

    let destination_abs = if raw_destination.is_absolute() {
        raw_destination
    } else {
        link.parent()
            .expect("link should have parent")
            .join(raw_destination)
    };

    match (fs::metadata(&destination_abs), fs::metadata(target)) {
        (Ok(destination_meta), Ok(target_meta)) => {
            destination_meta.dev() == target_meta.dev()
                && destination_meta.ino() == target_meta.ino()
        }
        _ => false,
    }
}
