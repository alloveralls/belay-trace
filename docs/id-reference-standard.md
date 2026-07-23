# ID and reference standard

Belay の Goal、Plan、Decision、Work、Note、Review、Evidence を、文書内の
Success Criterion と Delivery Map task まで含めて一貫して参照するための規格。

## 1. ID のスコープ

ID は二層に分ける。

| 対象 | 形式 | スコープ | 例 |
| --- | --- | --- | --- |
| Entry | `<TYPE>-<timestamp>-<sequence>-<slug>` | repository | `GOAL-20260723T120000-001-safe-sync` |
| Evidence | `EVD-<timestamp>-<sequence>` | repository | `EVD-20260723T120500-001` |
| Success Criterion | `SC-NNN` | defining Goal | `SC-001` |
| Delivery task | `T-NNN` | defining Plan | `T-001` |

Entry type prefix は `GOAL`、`PLN`、`DEC`、`WRK`、`REV`、`NOTE` とする。
timestamp は `created_at` または `captured_at` の時刻を compact form にしたもので、
同一秒内 sequence とともに Belay が採番する。

`SC-NNN` と `T-NNN` はそれぞれ `001` から始め、定義文書内で単調増加させる。
並べ替え、状態変更、文言変更では再採番しない。削除・統合した ID は欠番のまま
残し、別の項目へ再利用しない。並行編集で同じ番号が作られた場合は、マージ時に
新しく追加された側だけを未使用番号へ変更する。

`SC-001` や `T-001` 単体は文書ローカルであり、repository 全体では一意ではない。
文書の外から参照するときは、次の完全修飾参照を使う。

```text
GOAL-20260723T120000-001-safe-sync#sc-001
PLN-20260723T120100-001-deliver-safe-sync#t-001
```

fragment は小文字を canonical form とする。表示ラベルでは大文字の
`SC-001` / `T-001` を使ってよい。

## 2. 定義形式

Goal の `Success Criteria` は、各トップレベル項目の先頭で ID を定義する。

```markdown
## Success Criteria

- [SC-001] 双方変更を検出できる。
- [SC-002] 競合時に既存データを上書きしない。
```

Plan の `Delivery Map` は `ID` 列で task ID を定義し、`Goal item` 列では
対象 Goal の criterion を指定する。

```markdown
## Delivery Map

| ID | Goal item | Outcome / Task | Actor | State | Verification / Evidence |
| --- | --- | --- | --- | --- | --- |
| T-001 | SC-001 | 双方変更を検出する | AI | implemented | EVD pending |
| T-002 | SC-002 | 非上書きを検証する | AI | verified | EVD-20260723T120500-001 |
```

Plan が複数 Goal を扱う場合、`Goal item` に完全修飾参照を書く。Goal への
リンクが一つだけなら、同じ Plan 内では `SC-001` の短縮形を使用できる。

## 3. 文書ごとの参照規格

| 文書 | 定義する ID | 必須の構造化リンク |
| --- | --- | --- |
| Goal | Entry ID、`SC-NNN` | 前提・根拠への `references` / `supports` |
| Plan | Entry ID、`T-NNN` | 対象 Goal/criterion への `fulfills` |
| Decision | Entry ID | 対象 Goal/criterion または Plan/task への `fulfills` / `supports` |
| Work | Entry ID | 対象 Goal/criterion への `fulfills`、Plan/task への `implements` |
| Note | Entry ID | 言及対象への `references`。単なる本文記載だけで関係を表さない |
| Review | Entry ID | 対象への `reviews`、指摘が支える対象への `supports` |
| Evidence | Evidence ID | 検証対象への `verifies` または `refutes` |

Entry の関係は frontmatter の `links` に、Evidence の関係は append-only record の
`links` に保存する。本文に完全修飾 ID を書くことは可読性のために推奨するが、
本文の文字列だけを provenance graph の edge とはみなさない。
Browse Reader は本文中の完全修飾 Entry/fragment と Evidence ID をリンク表示する。
inline code と code block 内の文字列はリンク化しない。

例:

```yaml
links:
  - relation: implements
    id: PLN-20260723T120100-001-deliver-safe-sync#t-001
  - relation: fulfills
    id: GOAL-20260723T120000-001-safe-sync#sc-001
```

```sh
belay verify record \
  --kind test \
  --verdict pass \
  --source "cargo test" \
  --summary "conflict cases passed" \
  --verifies GOAL-20260723T120000-001-safe-sync#sc-001 \
  --verifies PLN-20260723T120100-001-deliver-safe-sync#t-002
```

## 4. 整合性

Belay は新しい構造化リンクまたは Evidence target を保存するとき、Entry の存在に
加えて fragment の存在も検査する。存在しない `#sc-NNN` / `#t-NNN` は拒否する。
重複定義されて一意に解決できない fragment も拒否する。定義後に直接 Markdown を
編集して参照先を削除・重複させた場合は、`belay doctor` が drift として報告する。

Goal lint は Success Criterion の ID 欠落と重複を報告する。既存データとの互換性の
ため、旧来の Goal hash fragment、`#sc-1`、Plan の `#task-t-1` は読み取り可能な
alias として残す。ただし新規作成・新規参照では canonical form だけを使う。

ID は同一性を表し、内容の正しさや関係の意味を保証しない。たとえば Evidence が
`SC-001` を target に持つだけでは、そのテストが criterion を十分に検証したことに
ならない。意味的妥当性は review の対象であり、freshness と verdict は別途評価する。
