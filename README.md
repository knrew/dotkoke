# dotkoke

## 概要
dotfilesを管理するCLIツール．
`dotfiles/home/`以下にあるファイル群を$HOMEにシンボリックリンクを貼る．
Unix系OSのみ対象．

## インストール

```sh
cargo install --git https://github.com/knrew/dotkoke
```

## 設定ファイル

`dotkoke`は以下の優先順位で設定ファイル(`*.toml`)を探索する．

1. コマンドラインオプション `--config <PATH>`
1. 環境変数 `DOTKOKE_CONFIG`
1. `$HOME/.config/dotkoke_config.toml`
1. `$HOME/.config/dotkoke/dotkoke_config.toml`

設定ファイルは以下のような構造である．

```toml
[general]
dotfiles = "/path/to/dotfiles_dir"
home = "/home/username"
backup_dir = "/path/to/backup_dir"
```


| キー        | 役割                                                                 |
|-------------|----------------------------------------------------------------------|
| `dotfiles`  | dotfiles レポジトリのルート．`dotfiles/home`配下が$HOMEのミラーとして扱われる |
| `home`      | 実際にリンクを貼りたい$HOMEルート                                 |
| `backup_dir`| リンク作成時に上書き対象ファイルを退避するディレクトリ．`YYYYmmdd_HHMM` サブディレクトリが自動生成される |


### ディレクトリ構成例

```
dotfiles/
├─ home/
│  ├─ .zshrc
│  └─ .config/nvim/init.lua
└─ README.md
```

上記の場合，`install`コマンドを実行すると`$HOME/.zshrc`と`$HOME/.config/nvim/init.lua`にシンボリックリンクが作成される．

## コマンド

`dotkoke <COMMAND> [OPTIONS]`

### install

`dotfiles/home/`以下を走査して，対応する$HOME側にシンボリックリンクを作成する．
既存ファイルが存在する場合，`backup_dir/YYYYmmdd_HHMM/...`へ移動してからリンクを作成する．

- `dotkoke install`: 実際にリンクを作成する．
- `dotkoke install --dry-run`: 実際の操作は行わず，処理予定内容を表示する．

### add <PATH>

$HOMEからdotfiles管理下へファイルを取り込む．
シンボリックリンクは取り込み対象外．
実際には<PATH>を`dotfiles/home/`以下の対応する場所にファイルをコピーする．
既に同名ファイルがdofiles管理対象に存在する場合はスキップする．

例
```sh
dotkoke add [--dry-run] /home/username/.bashrc
```

### remove <PATH>

`dotfiles/home/`からファイルを削除し，必要に応じて$HOME側の対応するシンボリックリンクも削除する．

例
```sh
dotkoke remove [--dry-run] dotfiles/home/.bashrc
```

### list

現在管理しているファイル一覧を表示する．

```sh
dotkoke list
```

### 未実装コマンド

- `init`: 設定ファイル(`*.toml`)を生成する．
- `clean`: 壊れたリンクや不要なリンクを削除する．
- `status`: リンク状態，未管理ファイル等を表示する．
