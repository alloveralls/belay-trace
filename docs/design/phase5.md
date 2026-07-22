# Phase 5 開発計画: Context compiler & Human-AI interface

[ロードマップ](../roadmap.md) Phase 5 の実装計画。
v1 の `belay context` を、Goal / Evidence を含むグラフ全体を対象と
した context compiler へ発展させる。選択は決定的アルゴリズムを
維持し、LLM による要約はエージェント側に任せる。

## 目的

エージェントが作業を始めるとき、「関連する Goal・Decision・制約・
非ゴール・過去の失敗・最新の Evidence」を、バジェット内で
決定的に選別した一つの文脈として渡せるようにする。

## ロードマップとの対応

- **完了条件**: エージェントが作業開始時に `belay context compile`
  の出力を標準で読み込む運用が定着し、文脈不足による手戻りが
  減ったと確認できる
- **見直し条件**: エージェント側の文脈取得能力の向上により選別の
  価値が薄れる場合、compile 機能は検索 API の提供に縮小する

## 依存

- Phase 1(Goal)— 必須
- Phase 3(Evidence)— 推奨。未導入でも動作するが Evidence
  セクションが空になる
- Phase 2 / 4 は不要(あれば lint 状態・coverage を含められる)

## スコープ

### In

- `belay context compile`(既存 `context` の上位互換)
- 目的別プロファイル(task-start / review / goal-drafting)
- 型を考慮した選択(非ゴール・制約・失敗履歴の優先度)
- Evidence の要約行(最新・fresh のもの)の含有
- Agent skill の更新(作業開始時の標準手順化)

### Out

- LLM による要約・圧縮(エージェント側の責務)
- embeddings(v1 原則を維持。BM25 + リンクグラフで足りない
  ことが実証されるまで導入しない)

## 設計

### 1. 既存 `context` との関係

- `belay context <query>` は互換のまま残す
- `belay context compile` は新しい選択パイプラインを使う。
  安定後、`context` の内部実装を compile に統一する

```sh
belay context compile "implement repository sync" \
  --budget 4000 --format agent
```

### 2. 選択パイプライン

v1 の「BM25 → one-hop リンク」を一般化し、4 段にする。
すべて決定的で、同一入力・同一リポジトリ状態なら同一出力。

1. **シード選択**: クエリの BM25 上位 + 明示指定
   (`--seed GOAL-...` で Goal 起点にできる)
2. **グラフ展開**: typed link を重み付きで辿る。
   リレーションごとの重みは固定表(config で調整可)
   - `fulfills` / `verifies` は強く辿る(意図と証拠の鎖)
   - `references` は弱く辿る
   - `supersedes` は新しい側を採用し、古い側は 1 行の
     「superseded された」注記に縮約する
3. **必須枠の確保**: バジェット配分の前に、シード Goal の
   Non-goals / Constraints セクションと、直近の失敗
   (Abandoned Work / Rejected Decision のタイトル行)を
   優先的に確保する。「やらないこと」は省略されやすいが
   手戻り防止の価値が最も高いため、先に枠を取る
4. **バジェット充填**: 残りを v1 と同じ規則(ランク加重、
   entry ごとに最低 1 evidence unit、90% 上限)で埋める

### 3. 単一の選択方針

`context compile` は用途によらず単一の決定的な選択パイプラインを使う。
用途別の重み付けは実装せず、必要な差は task の記述と `--seed` で明示する。

### 4. 出力形式(agent)

```text
# Context: implement repository sync
(compiled by belay, budget=4000)

## Goals (1)
GOAL-...-reliable-sync (active)
  [Success Criteria / Constraints / Non-goals の本文]

## Non-goals & constraints (must respect)
- ...

## Decisions (2)
DEC-... (accepted) …

## Evidence snapshot
- pass test cargo-test 2026-07-01 fresh → GOAL-...#sc-3f9a
- (stale) ci-run gh-actions 34 commits behind

## Past failures (2)
- WRK-... (abandoned): タイトル
- DEC-... (rejected): タイトル

## Sources
[全 display_id の一覧。エージェントが belay show で深掘りできる]
```

- Evidence は本文を含めず 1 行要約のみ(Phase 3 の
  `verify status` と同じ整形を再利用)
- 末尾の Sources により、compile は「入口」であって
  全文の代替ではないことを明確にする

### 5. モジュール構成

- `src/context.rs` を `src/context/` に分割:
  `seed.rs` / `expand.rs` / `reserve.rs` / `fill.rs` / `render.rs`
- 重み表は `config.toml` の `[context]` で上書き可能
  (既定値はバイナリ内蔵)

### 6. Agent 統合の更新

SKILL.md の標準手順を、用途を task と `--seed` で表す
`belay context compile "<task>"` に統一する。

## 実装ステップ

1. **M1: パイプライン骨格の抽出**
   - 既存 context.rs を段階構造へリファクタリング
     (出力不変のリファクタリングとしてスナップショットで保証)
2. **M2: グラフ展開の重み付けと supersedes 縮約**
3. **M3: 必須枠(非ゴール・制約・失敗履歴)**
4. **M4: 単一の優先順位と Evidence 要約行**
5. **M5: `context compile` CLI と skill 更新**
6. **M6: dogfooding**
   - belay-trace での全エージェント作業を compile 起点にし、
     手戻り事例の増減を Work エントリの記録から観測する

## テスト計画

- 決定性: 同一リポジトリ状態での出力バイト一致
  (スナップショットテスト)
- バジェット遵守: 境界値(極小 / 巨大バジェット)
- 必須枠: バジェットが小さくても非ゴール・制約が残ること
- supersedes 鎖、リンク循環があっても停止すること
- v1 `context` の回帰(既存出力の互換維持)

## リスク

- **選別の価値の陳腐化**: エージェントの検索能力向上で
  compile が不要になる可能性。見直し条件どおり、その場合は
  検索 API(構造化 JSON 出力)へ縮小する。パイプラインを
  段階分割しておくのはこの縮小を容易にするため
- **プロファイルの複雑化**: 3 種で開始し、config での自由定義は
  要望が実証されるまで作らない
- **決定性の破れ**: 時刻依存(鮮度表示)が出力に入る。
  鮮度は「コミット数差」で表記し、wall-clock を出力に含めない

## オープンクエスチョン

- compile 結果のキャッシュ(同一 HEAD なら再計算しない)を
  持つか。まず計測し、遅い場合のみ導入する
- MCP サーバーとしての提供(エージェントがツール呼び出しで
  compile を叩く形)。skill での CLI 呼び出しで十分かを
  dogfooding で見極めてから判断する
