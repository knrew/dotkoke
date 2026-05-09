# dotkoke

## 概要

dotkoke は dotfiles を管理する Unix 系 OS 向け CLI ツールである．
`dotfiles/home/` 以下にある通常ファイルを `$HOME` 側の対応パスへシンボリックリンクする．

## インストール

```sh
cargo install --git https://github.com/knrew/dotkoke
```

## 設定ファイル

`dotkoke` は以下の優先順位で設定ファイル(`*.toml`)を探索する．

1. コマンドラインオプション `--config <PATH>`
2. 環境変数 `DOTKOKE_CONFIG`
3. `$XDG_CONFIG_HOME/dotkoke/config.toml`
4. `$HOME/.config/dotkoke/config.toml`

設定ファイルが未指定で，上記の自動探索先にも存在しない場合は，以下の fallback 設定を使う．

| キー | fallback 値 |
| ---- | ---- |
| `dotfiles` | `$HOME/.dotfiles` |
| `home` | `$HOME` |
| `backup_dir` | `$HOME/.backup_dotfiles` |

`--config` または `DOTKOKE_CONFIG` で明示した設定ファイルが存在しない場合は fallback せずエラーにする．

設定ファイルは以下の構造である．

```toml
[general]
dotfiles = "/path/to/dotfiles_dir"
home = "/home/username"
backup_dir = "/path/to/backup_dir"
```

| キー | 役割 |
| ---- | ---- |
| `dotfiles` | dotfiles リポジトリのルート．`dotfiles/home/` 配下が `$HOME` のミラーとして扱われる |
| `home` | 実際にリンクを貼りたい `$HOME` ルート |
| `backup_dir` | リンク作成時に上書き対象ファイルやディレクトリを退避するルート．`YYYYmmdd_HHMM` サブディレクトリが自動生成される |

設定ファイルを読み込む場合，`dotfiles`，`home`，`backup_dir` は読み込み時に canonicalize されるため，指定先は事前に存在している必要がある．
fallback を使う場合，`$HOME/.dotfiles/home` は事前に存在している必要がある．`$HOME/.backup_dotfiles` は存在しなくてもよい．

### ディレクトリ構成例

```text
dotfiles/
├─ home/
│  ├─ .zshrc
│  └─ .config/nvim/init.lua
└─ README.md
```

上記の場合，`install` コマンドを実行すると `$HOME/.zshrc` と `$HOME/.config/nvim/init.lua` にシンボリックリンクが作成される．

## コマンド

```sh
dotkoke [OPTIONS] <COMMAND>
```

共通オプション:

- `--config <PATH>`: 使用する設定ファイルを指定する．

### install

`dotfiles/home/` 以下を走査して，対応する `$HOME` 側にシンボリックリンクを作成する．
既存ファイルまたはディレクトリが存在する場合，`backup_dir/YYYYmmdd_HHMM/...` へ移動してからリンクを作成する．

```sh
dotkoke install [--dry-run] [--show-skipped]
```

- `--dry-run`: 実際の操作は行わず，処理予定内容を表示する．
- `--show-skipped`: 既に正しいリンクが存在するためスキップしたファイルも表示する．

`dotfiles/home/` 以下にシンボリックリンクがある場合，それらはリンク作成対象にせず，警告して無視する．
`$HOME` 側に既に正しいシンボリックリンクがある場合はスキップする．
`$HOME` 側に別のシンボリックリンクがある場合は削除してからリンクを作成する．

### add <PATH>

`$HOME` から `dotfiles/home/` 以下へファイルを取り込む．
対象は `$HOME` 配下の通常ファイルのみで，シンボリックリンクは取り込み対象外としてスキップする．
既に同名ファイルが dotfiles 管理対象に存在する場合は上書きせずにスキップする．

```sh
dotkoke add [--dry-run] <PATH>
```

例:

```sh
dotkoke add /home/username/.bashrc
dotkoke add --dry-run /home/username/.config/git/config
```

### remove <PATH>

`dotfiles/home/` から管理ファイルを削除し，必要に応じて `$HOME` 側の対応するシンボリックリンクも削除する．
対象は `dotfiles/home/` 配下の通常ファイルのみである．
`$HOME` 側のリンクが別の管理ファイルを指している場合は削除しない．

```sh
dotkoke remove [--dry-run] <PATH>
```

例:

```sh
dotkoke remove /path/to/dotfiles/home/.bashrc
dotkoke remove --dry-run /path/to/dotfiles/home/.config/git/config
```

### list

現在管理している通常ファイルの一覧を表示する．
`dotfiles/home/` 以下のシンボリックリンクは管理ファイル一覧に含めない．

```sh
dotkoke list
```

### 未実装コマンド

- `init`: 設定ファイル(`*.toml`)を生成する．
- `clean`: 壊れたリンクや不要なリンクを削除する．
- `status`: リンク状態，未管理ファイル等を表示する．
