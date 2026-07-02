# dotkoke Specification

この文書は、dotkoke の公開挙動、CLI の契約、設定スキーマ、安全性要件の正本である。
利用者から見える振る舞いを定義し、内部モジュール構成、実装計画、利用する Rust crate の選定理由は扱わない。

## Document Responsibilities

- [specification.md](specification.md): 公開挙動、CLI の契約、設定スキーマ、安全性要件。
- [architecture.md](architecture.md): 実装方針と責務分離の記録。仕様を上書きしない。
- [development.md](development.md): コントリビューター向けの開発・検証手順。
- [glossary.md](glossary.md): プロジェクト用語と表記基準。

仕様、docs、実装、test が矛盾する場合は、暗黙に実装を正とせず差分の意図を確認してから揃える。

## Scope

dotkoke は Linux と macOS を対象とする dotfiles 管理 CLI である。
source tree の通常ファイルを destination tree へ反映する。

dotkoke は以下のコマンドを提供する。

```text
dotkoke init [--config <PATH>] [--dry-run]
dotkoke init --print
dotkoke install [--dry-run]
dotkoke add [--dry-run] [--install] [--update] <PATH>...
dotkoke remove [--dry-run] <PATH>...
dotkoke status
```

`list` と `clean` はこの仕様の対象外である。

CLI の help、output、log、error message は英語で表示する。
この仕様文書の本文は日本語で記述する。

## Terms

この仕様文書で使う主要用語は `docs/glossary.md` で定義する。
本文では glossary の定義に従う。

## Configuration

設定ファイルは TOML で記述する。

```toml
[paths]
dotfiles = "/path/to/dotfiles"
destination = "/home/user"
backup = "/home/user/.backup_dotfiles"

[source]
root = "home"
ignore = [
  ".config/some-tool/local.toml",
  ".config/some-tool/cache",
]

[placement]
default_method = "symlink"

[[placement.rules]]
path = ".config/some-tool/config.toml"
method = "copy"
```

### Paths

`paths.dotfiles` は dotfiles repository の root directory を表す。
absolute path でなければならない。
存在する directory でなければならない。

`paths.destination` は destination root を表す。
absolute path でなければならない。
存在する directory でなければならない。

`paths.backup` は backup root を表す。
absolute path でなければならない。
存在しない場合は必要になった時点で作成される。
存在する場合は directory でなければならない。

`paths.destination` と `source root` が同じ directory であってはならない。

### Source Settings

`source.root` は `paths.dotfiles` から source root directory への relative path である。
空文字、`.`、absolute path、`..` を含む path は configuration error とする。
`{paths.dotfiles}/{source.root}` は存在する directory でなければならない。

`source.ignore` は managed file から除外する source-relative path を表す。
省略時の default は空配列である。

`source.ignore` の各要素は source-relative path である。
glob、正規表現、negation は使えない。
一致判定は exact match で行う。
absolute path、空文字、`.`、`..` を含む path は configuration error とする。

`source.ignore` の path が通常ファイルとして存在する場合、その file は managed file にしない。
`source.ignore` の path が directory として存在する場合、その directory と配下の subtree は managed file にしない。
`source.ignore` の path が存在しない場合は configuration error にしない。
`source.ignore` の path が symlink や unknown file type に一致する場合も configuration error にしない。
subtree の一致判定は path component の境界で行う。

`source.ignore` によって除外された path は excluded path であり、`install`、`add`、`remove`、`status` のいずれでも managed file として扱わない。

### Placement Settings

`placement.default_method` は placement rule に一致しない managed file の placement method を表す。
指定できる値は `symlink` と `copy` である。
省略時の default は `symlink` である。

`[[placement.rules]]` は managed file ごとの placement method を指定する。

`placement.rules.path` は source-relative path である。
glob は使えない。
一致判定は exact match で行う。
absolute path、空文字、`.`、`..` を含む path は configuration error とする。

`placement.rules.method` に指定できる値は `symlink` と `copy` である。

同じ `path` を持つ placement rule が複数存在する場合は configuration error とする。
placement rule が存在しない managed file は `placement.default_method` を使う。

`placement.rules.path` が `source.ignore` の path と同じ、または `source.ignore` の path を directory とみなした subtree 配下にある場合は configuration error とする。

### Config Discovery

設定ファイルは以下の優先順位で探索する。

1. command line option `--config <PATH>`
2. environment variable `DOTKOKE_CONFIG`
3. `$XDG_CONFIG_HOME/dotkoke/config.toml`
4. `$HOME/.config/dotkoke/config.toml`
5. fallback config

`--config` または `DOTKOKE_CONFIG` で指定された path が存在しない場合は error とする。
指定された path が file でない場合も error とする。

`XDG_CONFIG_HOME` が未設定、空文字、または relative path の場合は無視し、`$HOME/.config/dotkoke/config.toml` の探索へ進む。

fallback config は以下と同等である。

```toml
[paths]
dotfiles = "$HOME/.dotfiles"
destination = "$HOME"
backup = "$HOME/.backup_dotfiles"

[source]
root = "home"

[placement]
default_method = "symlink"
```

fallback config の path は実行時に展開済み absolute path として扱う。

## Managed Files

dotkoke は `source root` 配下を再帰的に走査する。
`source.ignore` に一致する path は走査対象から除外する。
excluded directory の中は走査しない。

通常ファイルだけを managed file とする。
directory は走査対象であり、managed file ではない。
symlink は辿らず、managed file にしない。
FIFO、socket、device など通常ファイル、directory、symlink 以外の file type は managed file にしない。

`source.ignore` によって除外された path は warning や `status` の `unsupported` にはしない。
source tree の symlink と unknown file type は、`source.ignore` に一致しない場合、warning または `status` の `unsupported` として扱う。
読み取れない directory、entry の読み取り失敗、file kind の判定不能など、走査が不完全になる問題は error とする。
source tree scan error がある場合、部分的な filesystem 変更を残さないため filesystem を変更しない。

managed file の処理順は source-relative path 昇順で安定させる。

## Placement Methods

### symlink

`symlink` は destination path に managed file への symbolic link を作成する。
destination path の symlink が managed file の canonical path を指している場合、desired state と一致しているとみなす。

symlink の一致判定では symlink の target を解決して managed file の canonical path と比較する。
managed file と同じ inode を持つ別 path を指す symlink は一致とみなさない。

### copy

`copy` は destination path に通常ファイルとして managed file をコピーする。
destination path が通常ファイルで、内容 bytes と permission bits が managed file と一致している場合、desired state と一致しているとみなす。

mtime は一致判定に使わない。

## Destination and Conflicts

destination path が存在せず、親 directory が作成可能な場合、`install` は desired state を作成する。

destination path が desired state と一致する場合、`install` は何もしない。

destination path が存在し、desired state と一致しない場合、その状態を destination conflict とする。
destination conflict では、その destination path を対応する backup path へ移動してから desired state を作成する。
destination conflict のある path の種類は問わない。
通常ファイル、directory、symlink、broken symlink、unknown file type はすべて backup 対象の destination path である。

symlink は target を辿らず、symlink object 自体を backup path へ移動する。
相対 symlink の raw target は保持される。

destination path の親 path の途中に通常ファイル、symlink、unknown file type、判定不能 path がある場合、その managed file は install できない。
`status` では `blocked` として表示する。
`install` では error とし、部分的な filesystem 変更を残さない。

## Backup

backup root は `install` または `add --install` が既存 destination path を置き換える場合に使う。

1 回の実行につき 1 つの backup set directory を使う。
backup set directory の名前は local time の `YYYYmmdd_HHMMSS` とする。
同名 directory が既に存在する場合は `YYYYmmdd_HHMMSS-1`, `YYYYmmdd_HHMMSS-2` のように suffix を付ける。

backup path は destination-relative path を backup set directory 配下に維持する。

例:

```text
destination path: /home/me/.config/foo/config.toml
backup root:      /home/me/.backup_dotfiles
backup set dir:   /home/me/.backup_dotfiles/20260702_213000
backup path:      /home/me/.backup_dotfiles/20260702_213000/.config/foo/config.toml
```

backup set directory は backup 対象の destination path がある場合だけ作成する。
backup 対象の destination path がない実行では作成しない。

dry-run では backup set directory を作成しない。
ただし、作成予定の backup path は output に表示する。

個別 backup path が既に存在する場合は上書きせず error とする。

## Commands

### init

```text
dotkoke init [--config <PATH>] [--dry-run]
dotkoke init --print
```

`init` は config file と `source root` directory を作成する。

`--config <PATH>` が指定された場合はその path に config file を作成する。
指定されない場合は `$XDG_CONFIG_HOME/dotkoke/config.toml` を作成先とする。
`XDG_CONFIG_HOME` が未設定、空文字、または relative path の場合は `$HOME/.config/dotkoke/config.toml` を作成先とする。

既存 config file は上書きしない。
config file の親 directory は必要なら作成する。

生成される config は fallback config と同等である。
config に書かれる path は `$HOME` 展開済みの absolute path である。
config には環境変数や `~` を書かない。

`init` は `paths.dotfiles` と `source root` が存在しない場合は作成する。
既存 path が directory でない場合は error とする。
`init` は git repository を作成しない。

`--dry-run` は作成予定を表示し、filesystem を変更しない。

`--print` は fallback config を stdout に出力するだけで、filesystem を変更しない。
`--print` は `--config` および `--dry-run` と併用できない。

### install

```text
dotkoke install [--dry-run]
```

`install` は source tree 全体を destination root に反映する。
`source.ignore` に一致する path は plan に含めず、destination path の作成、copy、link、backup path への移動を行わない。
各 managed file の placement method は placement rule と default method から決定する。

`install` は実行前に plan を作成する。
plan 作成時に検出可能な error がある場合、filesystem を変更しない。

`--dry-run` は plan を表示し、filesystem を変更しない。

### add

```text
dotkoke add [--dry-run] [--install] [--update] <PATH>...
```

`add` は destination root 配下の通常ファイルを `source root` 配下へ取り込む。
`<PATH>...` は 1 個以上指定する。

file が指定された場合、その file を対象にする。
directory が指定された場合、その配下を再帰走査し、通常ファイルを対象にする。
symlink は辿らず、warning を出して対象から除外する。
unknown file type は warning を出して対象から除外する。
読み取れない directory、entry の読み取り失敗、file kind の判定不能は error とする。

relative path が指定された場合は current working directory 基準で解決する。
解決後の path が destination root 配下でない場合は error とする。

対応する source-relative path が `source.ignore` の path と同じ、または `source.ignore` の path を directory とみなした subtree 配下にある file は warning を出して対象から除外し、source root 側の file を作成しない。

対象 file は destination-relative path 昇順で安定処理する。
同じ file が複数回対象になった場合は dedup する。

通常の `add` は、対応する source root 側の path が存在しない場合だけコピーする。
対応する source root 側の path に何か存在する場合は上書きせず対象から除外する。
対応する source root 側の path の親 directory は必要なら作成する。

通常の `add` は destination root 側を変更しない。
placement rule も変更しない。

通常の `add` で source root 側の file を新規作成した対象が `copy` 配置の場合、取り込み後の destination path は desired state と一致する。
通常の `add` で source root 側の file を新規作成した対象が `symlink` 配置の場合、destination path は通常ファイルのまま残るため、取り込み後も `drifted` になりうる。

`--update` が指定された場合、対応する source root 側の path が既存 managed file である対象だけを更新する。
`source.ignore` に一致する対象は managed file ではないため warning を出して対象から除外する。
`--update` は `copy` 配置の managed file だけを対象にする。
`symlink` 配置の managed file は warning を出して対象から除外する。
destination path が通常ファイルでない場合は warning を出して対象から除外し、symlink は辿らない。
`--update` は merge を行わず、destination path の内容 bytes と permission bits を managed file に反映する。

`--install` が指定された場合、この add で対象になった file だけに `install` と同じ placement 処理を適用する。
`source.ignore` に一致して warning を出して対象から除外された file には `install` と同じ placement 処理も適用しない。
全 managed file への反映は行わない。

`--install` と `--update` は併用できない。

`--dry-run --install` は取り込み予定と install 予定の両方を表示し、filesystem を変更しない。
`--dry-run --update` は更新予定を表示し、filesystem を変更しない。

### remove

```text
dotkoke remove [--dry-run] <PATH>...
```

`remove` は `source root` 配下の managed file を削除する。
`<PATH>...` は 1 個以上指定する。

file が指定された場合、その managed file を対象にする。
directory が指定された場合、その配下を再帰走査し、managed file を対象にする。
symlink は warning を出して対象から除外する。
unknown file type は warning を出して対象から除外する。
読み取れない directory、entry の読み取り失敗、file kind の判定不能は error とする。

対象 path が `source root` 配下でない場合は error とする。

対象 path が `source.ignore` の path と同じ、または `source.ignore` の path を directory とみなした subtree 配下にある場合は warning を出して対象から除外し、source root と destination root のどちらも変更しない。

対象 file は source-relative path 昇順で安定処理する。
同じ file が複数回対象になった場合は dedup する。

`symlink` 配置の managed file を remove する場合、destination path がその managed file を指す symlink なら destination path の symlink を削除する。
destination path が broken symlink の場合も削除する。
destination path が別 symlink、通常ファイル、directory、unknown file type の場合は触らない。

`copy` 配置の managed file を remove する場合、destination path は削除しない。
destination path を残したことを output で明示する。

source root 側の managed file は削除する。

`--dry-run` は削除予定を表示し、filesystem を変更しない。

### status

```text
dotkoke status
```

`status` は読み取り専用である。
source tree の managed file を基準に destination path の状態を表示する。
`source.ignore` に一致する path は表示しない。
destination root 全体を走査して未管理 file を探すことはしない。

`status` は text output のみ提供する。
JSON output は提供しない。

状態は以下のいずれかである。

- `ok`: destination path が desired state と一致している。
- `missing`: destination path が存在せず、親 path が作成可能である。
- `drifted`: destination path が存在するが desired state と一致していない。
- `blocked`: destination path の親 path の途中に install を妨げる path がある。
- `unsupported`: source tree 内に存在するが、symlink や unknown file type などのため managed file にならない。

`ok` の定義は placement method によって異なる。
`symlink` では destination path の symlink が managed file の canonical path を指している場合に `ok` とする。
`copy` では destination path の regular file の内容 bytes と permission bits が managed file と一致する場合に `ok` とする。

`copy` の `drifted` は destination path の regular file の内容 bytes または permission bits が managed file と異なる状態を含む。

`drifted` は差分が存在する状態であり、`status` は解決方向を選ばない。
managed file を desired state として destination path へ反映する場合は `install` を使う。
`copy` 配置の destination path の regular file を managed file へ反映する場合は `add --update <PATH>` を使う。
`blocked` は `install` できない状態である。
`unsupported` は source tree の symlink や unknown file type などを表す。
`source.ignore` によって除外された path は `unsupported` として表示しない。

表示順は source-relative path 昇順とする。
summary は常に表示する。

例:

```text
Status:
  ok           .zshrc (symlink)
  missing      .config/foo/config.toml (copy)
  drifted      .gitconfig (copy, content differs)
  blocked      .config/bar/config.toml (symlink, parent is a file: /home/me/.config)
  unsupported  .config/app/link (source symlink)

Summary: 1 ok, 1 missing, 1 drifted, 1 blocked, 1 unsupported
```

判定に成功した場合、差分や `drifted` が存在しても exit code は 0 とする。
configuration error、source tree scan error、filesystem inspection error がある場合は exit code を非 0 とする。

## Output

通常の output は人間が読む text とする。
machine-readable format は初期仕様では提供しない。

変更を伴うコマンドは、実行した filesystem operation または dry-run plan を表示する。
warning は stderr に表示する。
progress 表示を行う場合、通常 output を壊してはならない。
非 TTY への出力では制御文字に依存した表示を行わない。

## Safety Requirements

dotkoke は利用者の既存データを失わせないことを最優先する。

既存 destination path を置き換える場合は、削除ではなく対応する backup path へ移動する。
backup path への上書きは行わない。
dry-run と通常実行は同じ plan に基づく。
plan 作成時に検出可能な error がある場合、filesystem を変更しない。

source tree と destination tree の path 解決では、symlink 判定、存在確認、権限エラー、broken symlink、親 path が directory でない場合を明示的に扱う。
