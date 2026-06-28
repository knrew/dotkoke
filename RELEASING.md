# Release

## 配布方針

dotkoke は crates.io には publish しない．
主配布は GitHub Releases に添付する build 済み binary と `install.sh` である．
Rust toolchain を持つ利用者向けには，tagged release を指定した `cargo install --locked --git --tag` を正式手順として扱う．
default branch からの `cargo install --locked --git` は開発版であり，安定版としては扱わない．
正式な配布経路は GitHub Releases と tagged release を指定した `cargo install` である．

## Versioning

version は SemVer を基準にする．
release tag は `vX.Y.Z` とし，`Cargo.toml` の `[package].version` と一致させる．
`0.y.z` の間は破壊的変更を含む可能性がある．

## Release 前チェックリスト

- 関連 issue と PR の状態を確認する．
- `Cargo.toml` の version を更新する．
- README の install 例に書く version を必要に応じて更新する．
- `cargo fmt --all --check --verbose` を実行する．
- `cargo clippy --workspace --all-features -- -D warnings` を実行する．
- `cargo test --workspace --all-features --no-fail-fast --verbose` を実行する．
- release notes に含める内容を確認する．

## Release 手順

1. `Cargo.toml` の `[package].version` を release する version に更新する．
2. README の tagged release 例を同じ version に更新する．
3. 変更を commit する．
4. `vX.Y.Z` 形式の tag を作成する．
5. tag を push する．
6. GitHub Actions の Release workflow が成功することを確認する．
7. GitHub Release に assets が添付されていることを確認する．
8. `install.sh` で smoke test する．

例:

```sh
git tag v0.1.0
git push origin v0.1.0
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/knrew/dotkoke/releases/latest/download/install.sh | sh
```

## Release assets

Release workflow は以下を GitHub Release に添付する．

- `install.sh`
- `SHA256SUMS`
- `dotkoke-v0.1.0-x86_64-unknown-linux-gnu.tar.xz`
- `dotkoke-v0.1.0-x86_64-unknown-linux-musl.tar.xz`
- `dotkoke-v0.1.0-x86_64-apple-darwin.tar.xz`
- `dotkoke-v0.1.0-aarch64-apple-darwin.tar.xz`

各 archive の layout は以下である．

```text
dotkoke-v0.1.0-<target>/
  dotkoke
  README.md
  LICENSE
```

## install.sh

`install.sh` は target を自動判定し，`dotkoke-${VERSION}-${TARGET}.tar.xz` と `SHA256SUMS` を GitHub Releases から download する．
archive の sha256 checksum を検証してから binary を install directory に配置する．
default の install directory は `$HOME/.local/bin` である．
`install.sh` は shell profile や rc file を自動編集しない．

override には以下を使える．

- `--target <triple>` または `DOTKOKE_TARGET`
- `--to <dir>` または `DOTKOKE_INSTALL_DIR`
- `--version <tag>` または `DOTKOKE_VERSION`
- `DOTKOKE_DOWNLOAD_BASE_URL`

正式サポート target は以下である．

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

## Rollback / failed release

tag push 後に workflow が失敗し，GitHub Release が作成されていない場合は，修正 commit を入れて tag を作り直すか，同じ tag を扱うリスクを確認してから再実行する．
公開済み release asset の差し替えは避ける．
公開後に問題が見つかった場合は，可能な限り新しい patch version を release する．
