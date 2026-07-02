# dotkoke Architecture Notes

この文書は dotkoke の実装方針を記録する。
仕様の正は `docs/specification.md` であり、この文書は仕様を上書きしない。
用語は `docs/glossary.md` に従う。

## Core Design

filesystem を変更するコマンドは、plan 作成と plan 実行を分離する。
`--dry-run` は通常実行と同じ plan を使い、plan を表示するだけで filesystem を変更しない。

実装では以下の責務を分離する。

- config の探索、parse、validation。
- source tree と destination tree の path 解決。
- filesystem inspection。
- command ごとの plan 作成。
- plan の実行。
- terminal output。

source tree の走査と destination path の検査は、通常ファイル、directory、symlink、broken symlink、unknown file type、判定不能状態を区別する。
symlink を扱う処理では、目的に応じて `symlink_metadata`、`metadata`、`read_link` を使い分ける。

## Terminal Output

CLI output の整形には `console` を使う方針とする。
`console` は terminal access、style、ANSI handling、unicode width handling を提供する。
TTY と非 TTY の違いを考慮し、pipe や log file に出力しても読みやすい text output を維持する。

長い走査や install 実行の進捗表示には `indicatif` を使う方針とする。
`indicatif` は progress bar と spinner を提供し、progress bar は通常 stderr に描画される。
通常の stdout output と progress rendering が混ざらないように扱う。

進捗表示は仕様上の必須 output ではない。
非 TTY では progress rendering を抑制または単純な text output にする。

参考:

- `console`: https://docs.rs/console/latest/console/
- `indicatif`: https://docs.rs/indicatif/latest/indicatif/

## Error Handling

通常実行経路では panic しない。
失敗は `Result` で返し、対象 path と失敗した操作が分かる error context を付ける。

plan 作成中に検出できる error は、実行開始前にまとめて検出する。
部分的な filesystem 変更を残さないため、致命的な source tree scan error や destination path の親 path の blocked 状態がある場合は filesystem を変更しない。

plan 実行中に失敗した場合は、実行済み filesystem operation が分かる context を付ける。
自動 rollback は仕様化しない。

## Testing Strategy

仕様上の振る舞いは temporary directory を使った integration-style test で検証する。

重点的に検証する対象:

- config discovery と validation。
- `symlink` と `copy` の一致判定。
- destination conflict の backup。
- broken symlink と relative symlink の扱い。
- source tree の symlink、unknown file type、source tree scan error。
- `add` と `remove` の複数 path、directory input、dedup。
- `add --install` が対象 file だけを反映すること。
- `status` の状態分類と exit code。
- `--dry-run` が filesystem を変更しないこと。

CLI help、stdout、stderr の user-visible text は snapshot 的に検証する。
