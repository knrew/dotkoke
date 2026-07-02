# dotkoke Development

この文書は dotkoke のコントリビューター向けに、開発と検証の最小手順をまとめる。
公開挙動の正は [specification.md](specification.md)、実装方針は [architecture.md](architecture.md)、用語基準は [glossary.md](glossary.md) を参照する。

## Standard Checks

Rust code を変更した場合は、少なくとも formatting、lint、test を確認する。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets
cargo test
```

対象を絞って確認する場合は、関連する package、module、test name の filter を指定する。

```sh
cargo test <module_or_test_name>
```

## Documentation

- [README.md](../README.md) は利用者向けの入口として、概要、インストール、クイックスタート、主要リンクに絞る。
- [specification.md](specification.md) は公開挙動、CLI の契約、設定スキーマ、安全性要件の正本として保つ。
- [architecture.md](architecture.md) は実装方針を記録する。仕様を上書きしない。
- この文書は開発・検証手順に絞る。
- [glossary.md](glossary.md) はプロジェクト用語と表記基準に揃える。

公開挙動、CLI option、設定 key、安全性要件を変更した場合は、同じ変更で関連する利用者向けドキュメントも更新する。
