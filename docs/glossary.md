# dotkoke Glossary

この文書は dotkoke の仕様文書で使う主要用語を定義する。
仕様の正は `docs/specification.md` であり、この文書は用語の意味を補足する。

## CLI terms

- command: `init`、`install`、`add`、`remove`、`status` など、dotkoke の操作名。
- option: `--config`、`--dry-run`、`--install` など、名前付きの指定。CLI ドキュメントでは `flag` ではなく `option` を使う。
- argument: `<PATH>` のような位置引数、または `--config <PATH>` の `PATH` のような option value。
- usage: CLI または reference に示す command syntax。

## source root

`paths.dotfiles` と `source.root` から決まる、管理対象の元 file を置く root directory。
具体的には `{paths.dotfiles}/{source.root}` である。

## source tree

`source root` 配下の filesystem tree。
dotkoke はこの tree を走査して managed file を見つける。

## destination root

managed file の配置先 root directory。
通常は利用者の home directory である。

## destination tree

`destination root` 配下の filesystem tree。
dotkoke は managed file に対応する destination path だけを扱い、destination tree 全体から未管理 file を探すことはしない。

## destination path

managed file の配置先 path。
destination path は存在しないこともあり、存在する場合も通常ファイル、directory、symlink、broken symlink、unknown file type などになりうる。

## source-relative path

`source root` から見た相対 path。
managed file の識別、`source.ignore`、placement rule の照合に使う。

## destination-relative path

`destination root` から見た相対 path。
destination path と backup path の対応付けに使う。

## managed file

`source root` 配下の通常ファイルのうち、`source.ignore` に一致しない file。
dotkoke が `install`、`add --update`、`remove`、`status` で管理対象として扱う単位である。

## excluded path

`source.ignore` によって managed file から除外される source-relative path。
excluded path は `install`、`add`、`remove`、`status` のいずれでも managed file として扱わない。

## placement method

managed file を destination path に配置する方法。
指定できる値は `symlink` と `copy` である。

## placement rule

managed file の source-relative path に placement method を割り当てる設定。
一致する placement rule がない managed file には `placement.default_method` を使う。

## desired state

placement method によって定まる destination path のあるべき状態。
`install` は destination path を desired state に近づける。

## destination conflict

destination path が存在し、desired state と一致していない状態。
`install` は destination conflict のある path を backup path へ移動してから desired state を作成する。

## backup root

backup を保存する root directory。
`paths.backup` で指定する。

## backup set directory

1 回の実行で作成される backup 用 directory。
backup set directory は backup root 配下に作成され、その実行で backup される destination path をまとめて保持する。

## backup path

backup される destination path の移動先 path。
destination-relative path を backup set directory 配下に維持する。

## plan

実行前に決定される filesystem operation の一覧。
`--dry-run` と通常実行は同じ plan に基づく。

## dry-run

plan を表示し、filesystem を変更しない実行 mode。

## status state

`status` が destination path ごとに表示する状態。

- `ok`: destination path が desired state と一致している。
- `missing`: destination path が存在せず、親 path が作成可能である。
- `drifted`: destination path が存在するが desired state と一致していない。
- `blocked`: destination path の親 path の途中に install を妨げる path がある。
- `unsupported`: source tree 内に存在するが、symlink や unknown file type などのため managed file にならない。

## Notation Rules

- CLI documentation では `flag` ではなく `option` を使う。
- `source-relative path` と `destination-relative path` は hyphenated form で書く。
- `dry-run` は hyphenated form で書く。
- `backup set directory`、`backup root`、`backup path` を使い分け、まとめて `backup directory` と曖昧に書かない。
- `symlink` は filesystem object の種類を指すときに使い、CLI output や仕様上の用語としては `symbolic link` と混在させない。
