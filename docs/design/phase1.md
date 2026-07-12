# Phase 1 開発計画: Goal as a first-class object

[ロードマップ](../roadmap.md) Phase 1 の実装計画。
Goal を第一級の entry type として導入し、意図を構造化して記録できるようにする。

## 目的

「なぜこの作業をするのか」を Plan(どう進めるか)から分離し、
成功条件・制約・非ゴールを構造化した Goal として記録・リンクできるようにする。

## ロードマップとの対応

- **完了条件**: belay-trace 自身の開発を Goal 起点で 1 ヶ月以上運用し、
  Goal → Decision → Work のリンクがレビューや判断で実際に参照される
- **見直し条件**: Goal が Plan と実質的に区別されず形骸化する場合、
  独立した entry type をやめ、Plan の構造化拡張として再設計する

## スコープ

### In

- `EntryType::Goal` の追加(prefix `GOAL`、directory `goals/`)
- Goal の構造化セクション(テンプレートと検証)
- LinkRelation の拡張: `fulfills`, `supports`
- 既存コマンド(add / show / status / link / search / sync /
  export / context / doctor / rebuild)の Goal 対応
- スキーマ・Markdown 形式のマイグレーション

### Out(後続フェーズ)

- Goal の品質評価(`goal lint` / `goal improve`)は Phase 2
- Evidence と `verifies` リレーションは Phase 3
- Goal Coverage の集計は Phase 4

## 設計

### 1. EntryType::Goal

`src/entry.rs` の `EntryType` に `Goal` を追加する。

| 項目 | 値 |
| --- | --- |
| prefix | `GOAL`(`NOTE` と同様の 4 文字) |
| directory | `goals` |
| default status | `Draft` |

許可ステータスは既存の `EntryStatus` を再利用し、新変種は追加しない。

```text
Draft → Active → Completed
              ↘ Superseded / Abandoned
```

- `Draft`: 起草中。lint / coverage の対象外にできる
- `Active`: 合意済みで作業の基準になっている
- `Completed`: 達成。Phase 3 以降は Evidence の裏付けを推奨
- `Superseded` / `Abandoned`: 置き換え・撤回

### 2. Goal の構造化: metadata ではなく本文セクションで表現する

構造化フィールドは `metadata_json` ではなく、
**本文の必須セクション(Markdown 見出し)** として表現する。

理由:

- 既存の `generate_chunks` が section 単位で chunk 化しており、
  検索・context 生成が追加実装なしでセクションを認識できる
- Markdown mirror が人間にとって編集可能な一次形式であるという
  v1 の原則(deterministic mirror)を保てる
- Phase 2 の lint は「必須セクションの有無・空虚さ」の決定的検査として
  自然に実装できる

必須セクション(`belay add goal` がテンプレートとして雛形を生成):

```markdown
## Summary
## Success Criteria
## Constraints
## Non-goals
## Verification
## Risks
```

- Success Criteria / Non-goals は箇条書きを推奨
  (Phase 4 で項目単位の coverage 集計に使うため)
- v1 の `parse` / `render` は本文を素通しするため、
  セクションの存在検証は Phase 1 では `doctor` の警告に留め、
  エラー化は Phase 2 の lint に委ねる

### 3. LinkRelation の拡張

`src/entry.rs` の `LinkRelation` に追加する。

| relation | from → to | 意味 |
| --- | --- | --- |
| `fulfills` | Decision / Work / Plan → Goal | Goal の達成に寄与する |
| `supports` | 任意 → Decision / Goal | 判断・目標の根拠を与える |

`verifies`(Evidence → 任意)は Phase 3 で追加する。
`entry_links.relation` は TEXT 格納のため SQLite スキーマ変更は不要。

### 4. マイグレーションと互換性

- **SQLite**: `entries.type` / `entry_links.relation` は TEXT のため
  DDL 変更は不要。ただし挙動の変化(新しい type / relation の受理)を
  明示するため `LATEST_SCHEMA_VERSION` を 3 に上げ、
  no-op マイグレーションとして記録する
- **Markdown**: front matter 形式は変わらないため
  `MARKDOWN_SCHEMA_VERSION` は 1 のまま維持する
- **後方互換**: 旧バイナリは `goal` type をパースできず validation
  エラーになる。これは仕様とし、README に最低バージョンを明記する
- **`belay init`**: `goals/` ディレクトリを作成する。既存リポジトリは
  `belay doctor` が `goals/` 欠落を検出し、`belay sync` /
  初回 `belay add goal` が作成する
- **`belay rebuild`**: goals/ 配下の mirror を含めて再構築できること

### 5. Agent 統合の更新

`.belay/agent/` の `AGENTS.md.snippet` と Claude / Codex SKILL.md を
更新し、次のワークフローを明記する。

- 作業開始時に関連 Goal を検索し、なければ人間に Goal 起草を促す
- Work / Decision を作る際は `fulfills` で Goal へリンクする

## 実装ステップ

TDD(tests/cli.rs の統合テスト + 各モジュールの単体テスト)で進める。

1. **M1: モデル拡張**
   - `EntryType::Goal`(prefix / directory / statuses / FromStr /
     Display / ALL)
   - `LinkRelation::Fulfills` / `Supports`
   - 単体テスト: round-trip、ステータス遷移、リレーション検証
2. **M2: ストレージとマイグレーション**
   - `LATEST_SCHEMA_VERSION = 3`(no-op マイグレーション)
   - `repository.rs` / `store.rs` の goals/ ディレクトリ対応
   - `rebuild` / `sync` / `doctor` の goal mirror 対応
3. **M3: CLI とテンプレート**
   - `belay add goal` で必須セクションの雛形を生成
     (`--body` / `--body-file` / `--stdin` 指定時は雛形を出さない)
   - `search --type goal`、`status`、`link` の受理
   - 統合テスト: add → link → show → sync → rebuild の一連
4. **M4: doctor / export / context の対応確認**
   - doctor: goals/ 欠落、必須セクション欠落の警告(warning 扱い)
   - export / context が goal を含むことの検証
5. **M5: Agent 統合とドキュメント**
   - snippet / SKILL.md テンプレート更新、README 更新
6. **M6: Dogfooding 開始**
   - belay-trace 自身のリポジトリで Phase 2 以降の Goal を
     `belay add goal` として登録し、運用を開始する

## テスト計画

- 単体: EntryType / LinkRelation の parse・render・遷移表
- 統合(tests/cli.rs): goal のライフサイクル一式、
  旧スキーマ DB からの自動マイグレーション、
  goal mirror を手編集 → sync → conflict 解決
- 回帰: 既存 5 type の全テストが変更なしで通ること

## リスク

- **Plan との役割重複**: dogfooding で「Goal と Plan のどちらに書くか」
  迷いが頻発するなら見直し条件に該当する。運用ガイドを snippet に
  明記して先手を打つ
- **テンプレートの過剰構造化**: 必須セクションが多すぎると起草の
  ハードルが上がる。Phase 1 ではセクション欠落を警告に留める

## オープンクエスチョン

- Goal 同士の階層(親 Goal / サブ Goal)を許すか。
  Phase 1 では `references` で代用し、必要性を dogfooding で判断する
