# Phase 2 開発計画: Goal Engineering support

[ロードマップ](../roadmap.md) Phase 2 の実装計画。
Goal のレビュー・改善を支援する。決定的なチェックは Belay 本体が、
意味的な評価はエージェント側 LLM が担う
(ロードマップ「設計原則との整合」参照)。

## 目的

Goal を「書くだけ」から「レビュー・改善されるもの」にする。
本体は LLM を呼ばず、決定的な検査と、エージェントへ渡す
構造化されたルーブリック・文脈の生成に徹する。

## ロードマップとの対応

- **完了条件**: dogfooding で lint の指摘が Goal の実際の修正に
  つながった事例が継続的に発生する
- **見直し条件**: スコアや指摘が形式的に消化されるだけなら、
  スコア表示をやめチェックリスト提示のみに縮小する

## 依存

- Phase 1(Goal entry type、必須セクション)が完了していること

## スコープ

### In

- `belay goal lint <id>`: 決定的な品質検査 + ルーブリック出力
- `belay goal improve <id>`: エージェント向け改善依頼バンドルの生成
- 曖昧語辞書(組み込み + `config.toml` で拡張)
- Agent skill の更新(lint → LLM 評価 → 人間確認のループ)

### Out

- 本体からの LLM 呼び出し(Non-goal。恒久的に対象外)
- Evidence に基づく検証は Phase 3
- Coverage 集計は Phase 4

## 設計

### 1. `belay goal lint`

新サブコマンド。`goal` を第一級のコマンド名前空間として導入する。

```sh
belay goal lint GOAL-20260701T090000-001-example
belay goal lint --all --format json
```

検査は 3 層に分ける。すべて決定的である。

| 層 | 検査内容 | 例 |
| --- | --- | --- |
| 構造 | 必須セクションの有無・空虚さ | Success Criteria が空 |
| 語彙 | 曖昧語辞書によるフラグ | 「高速に」「適切に」「など」 |
| グラフ | リンクの欠落・矛盾 | Active なのに fulfills が 0 件 |

出力(human 形式):

```text
GOAL-20260701T090000-001-example

Checklist: 6/9 passed

Missing:
- [structure] Success Criteria: section is empty
- [lexicon]   Constraints: ambiguous term "適切に" (line 12)
- [graph]     no incoming fulfills link while status is active

Rubric for semantic review: run `belay goal improve` and
pass the output to your agent.
```

- 既定はチェックリスト表示。**スコアは `--score` を付けた場合のみ**
  表示する(ロードマップの見直し条件を踏まえ、初期から任意にする)
- `--format json` は CI やエージェントが消費できる安定した
  スキーマで出力する
- 終了コード: 指摘ありでも `0`。`--strict` 指定時のみ `4` を返す
  (CI ゲート用)

### 2. 曖昧語辞書

- 組み込み辞書(日本語・英語)を `src/lint/lexicon.rs` に持つ
- `config.toml` の `[lint]` セクションで追加・除外できる

```toml
[lint]
ambiguous-terms = ["いい感じに"]
allowed-terms = ["など"]
```

- 誤検知が運用を壊すため、語彙層の指摘は常に warning とし、
  `--strict` でもエラーに昇格しない

### 3. ルーブリックと `belay goal improve`

意味的な評価(Testability、Business Alignment など 8 軸)は
本体では判定できない。本体は **評価を依頼するための構造化バンドル**
を生成する。

```sh
belay goal improve GOAL-...-example --budget 3000
```

出力(agent 形式、決定的):

1. 対象 Goal の全文
2. lint 結果(未充足チェック項目)
3. 8 軸ルーブリック(Completeness / Consistency / Testability /
   Observability / Ambiguity / Business Alignment /
   Risk Awareness / Context Fit)の評価指示
4. 関連文脈: `fulfills` / `supports` / `references` で繋がる
   Decision・Plan、および BM25 で近い過去エントリ
   (`context.rs` の選択ロジックを再利用)
5. エージェントへの出力指示: 改善後の Goal 案 / 改善理由 /
   追加された観点 / 人間に確認すべき未解決の質問

改善の適用は人間または人間監督下のエージェントが
`belay sync --prefer markdown` 経由で行う。本体は書き換えない。

### 4. モジュール構成

- `src/lint/mod.rs`: 検査エンジン(構造・語彙・グラフの 3 層)
- `src/lint/lexicon.rs`: 組み込み辞書
- `src/cli.rs`: `Goal` サブコマンド(`lint` / `improve`)
- `improve` の文脈選択は `src/context.rs` の generate を再利用する

### 5. Agent 統合の更新

SKILL.md に次のループを明記する。

1. Goal 起草後、`belay goal lint` を実行し構造指摘を解消する
2. `belay goal improve` の出力でエージェントが意味的レビューを行う
3. 未解決の質問を人間へ提示し、回答を Goal へ反映する

## 実装ステップ

1. **M1: lint エンジン(構造層)**
   - 必須セクション検査、`--format json` スキーマ確定
   - 単体テスト: セクション有無の全パターン
2. **M2: 語彙層とグラフ層**
   - 組み込み辞書 + config 拡張、リンク検査
   - 誤検知テスト(コードブロック内は対象外、など)
3. **M3: `goal improve` バンドル生成**
   - ルーブリック雛形、context 再利用、budget 制御
   - スナップショットテストで出力の決定性を保証
4. **M4: CLI 統合と `--strict` / `--score`**
   - tests/cli.rs に一連の統合テスト、終了コード検証
5. **M5: skill 更新と dogfooding**
   - belay-trace 自身の全 Active Goal に lint を通し、
     指摘 → 修正の事例を Work エントリとして記録する

## テスト計画

- lint の各検査は入力 Markdown → 指摘リストの表駆動テスト
- `--format json` はスキーマ互換性テスト(フィールド削除を検知)
- improve 出力は同一入力 → 同一出力のスナップショットテスト
- 日本語・英語混在の Goal での語彙検査

## リスク

- **疑似精度**: スコアが独り歩きする。既定でスコア非表示、
  チェックリスト主体にすることで軽減する
- **語彙検査の誤検知**: warning 固定 + allowlist で運用逃げ道を確保
- **improve バンドルの肥大化**: budget 制御を必須にし、
  既定値は `context` と同じ水準に合わせる

## オープンクエスチョン

- lint を `belay doctor` に統合するか独立コマンドのままにするか。
  Phase 2 では独立させ、doctor は「lint 未実施の Active Goal あり」
  の通知のみとする
