# Phase 0 開発計画: context budget のコスト・品質改善

[ロードマップ](../roadmap.md) Phase 0(Traceability foundation)の
品質改善。`belay context` の budget 制御に見つかった構造的な弱点を、
Phase 1 に先行して修正する。日常利用に直結するため最優先で実施する。

## 目的

`belay context` が同じバジェットでより多くの有効情報を返すようにする。
「バジェットを超えない」「決定的」という既存の保証は維持したまま、
配分アルゴリズムの退化・固定オーバーヘッド・出力品質を改善する。

## 背景(コードレビューでの発見)

対象: `src/context.rs`(必要に応じて `src/cli.rs` のヘルプ、
`tests/cli.rs` の期待値)

1. **広く浅く問題(最重要)**: 最低保証パス
   (`fit_minimum_evidence` のループ)が関連度カットオフなしに
   最大 32 候補(primary 12 + linked 20)を admit する。
   agent 形式は 1 エントリあたり定型行 35〜55 トークン +
   最低 evidence 40 トークンを消費するため、既定バジェット 2500
   (selection 枠 2250)は最低保証だけで枯渇し、
   `distribute_remaining_budget` のランク加重が発動しない。
   BM25 1 位も linked の 30 位も同じ 40 トークンのスタブになる
2. **タスクエコー**: `task_budget = selection_budget / 3` により、
   長いタスク文はヘッダーで最大 750 トークンを消費する
3. **配分の取りこぼし**: 各 rank の `share` は初回の `remaining`
   から一括計算され、使い切れなかった分が後続 rank へ回らない
4. **定型行の過剰**: エントリごとの `Next: belay show <id>`、
   `Tags: none`、ヘッダーの `Selection limit` 行など、agent 形式で
   1 エントリ 15〜25 トークンの削減余地がある
5. **evidence の順序破壊**: `evidence_priority` のソートキー第 3 要素
   が `text` のため、同一クラス内の文が辞書順に並び、抜粋の文脈が
   壊れる(キーに text を入れたのは隣接 `dedup()` のためと推定)
6. **release での budget 未検証**: 最終保証が `debug_assert!` のみ。
   `distribute_remaining_budget` の縮小ループに `len() == 1` で
   break する脱出口があり、理論上は超過したまま返るパスがある
7. **軽微な無駄**: `query_terms` の 3 回計算、DB コネクションの
   3 回オープン、chunk 取得の per-result prepare と N+1 クエリ

## スコープ

### In

- 上記 1〜7 の修正(`src/context.rs` 中心)
- agent 形式の出力フォーマット変更と、それに伴う CLI ヘルプ・
  README・統合テスト期待値の更新

### Out(Phase 5 送り)

- トークン数の差分更新による O(候補数) 化(現行キャップでは
  実測上問題にならないため、候補数が増える Phase 5 で行う)
- プロファイル・必須枠・グラフ重み付けなどの機能追加

## 設計

### 1. バジェット適応の admission(発見 1, 3)

最低保証パスの前に、admit する候補数をバジェットから導出する。

```rust
const TARGET_ENTRY_TOKENS: usize = 150;
const MIN_ADMITTED: usize = 3;

let admission_cap = (selection_budget
    .saturating_sub(estimate_tokens(&header))
    / TARGET_ENTRY_TOKENS)
    .clamp(MIN_ADMITTED, candidates.len());
```

- 候補順(primary を BM25 順、次に linked)の先頭
  `admission_cap` 件のみを最低保証パスの対象にする
- 既定バジェット 2500 → selection 2250 → cap ≈ 14。
  最低保証後に約 1,000 トークンが残り、ランク加重が機能する
- `TARGET_ENTRY_TOKENS` は「定型行 + 意味のある抜粋」の目標値。
  定数として持ち、config 化は要望が出るまでしない
- admit されなかった候補は **Sources フッター** に display_id のみ
  列挙する(`Also related: DEC-..., WRK-...`)。バジェット内に
  収まる件数まで先頭から含め、収まらなければ行ごと省略する。
  エージェントは `belay show` で深掘りできるため、広さは
  この 1 行で維持する
- **share の繰り越し**: `distribute_remaining_budget` で各 rank が
  使い切れなかった share を次の rank の share に加算する
  (`carry` 変数の追加のみ。決定性は保たれる)

### 2. タスクエコーの定数キャップ(発見 2)

```rust
const TASK_ECHO_BUDGET: usize = 64;
let task_budget = TASK_ECHO_BUDGET.min(selection_budget / 3).max(1);
```

- 小バジェット時の挙動(selection/3)は下位互換として維持し、
  上限だけを 64 に固定する

### 3. agent 形式の定型行圧縮(発見 4)

human 形式は人間の可読性を優先して現状維持。agent 形式のみ変更する。

| 項目 | 変更 |
| --- | --- |
| `Next: belay show <id>`(各エントリ) | 削除。フッターに 1 行 |
| `Tags: none` | tags が空なら行ごと省略 |
| `Selection limit:`(ヘッダー) | 削除(Budget 行は残す) |
| `Why relevant:` | `Why:` に短縮 |

フッター(agent 形式、末尾に固定):

```text
Read more: belay show <display-id>
Also related: <admit されなかった候補の display_id 列>
```

- 1 エントリあたり約 15〜25 トークン、既定設定で合計
  200〜350 トークンの削減を見込む
- 出力形式の変更なので `cli.rs` の long help
  ("gives higher-ranked entries a larger share" の記述を
  admission cap の説明に更新)と README の該当節も同時に直す

### 4. evidence の文書順維持(発見 5)

- `load_candidates` で、ソート前に `HashSet<(String, String)>` で
  重複除去(初出を残す)し、ソートキーを `(class, important)` に
  変更する。stable sort なので同一クラス内は文書順が保たれ、
  決定性も維持される
- `dedup()` 呼び出しは削除する

### 5. release ビルドでの budget 保証(発見 6)

- `distribute_remaining_budget` の縮小ループの `len() == 1` 脱出口を
  廃止し、最後の 1 unit は pop ではなく `truncate_evidence` による
  縮小で対応する
- `generate` の最終段で `estimated_tokens > selection_budget` の場合
  は `truncate_at_boundary(output, selection_budget)` で強制的に
  収める(防衛線。到達しない設計だが release でも保証する)。
  `debug_assert!` は「防衛線が発動しないこと」の検出用に残す

### 6. 軽微な無駄の除去(発見 7)

- `query_terms(task)` は `generate` で 1 回計算して引き回す
- DB コネクションは `generate` で 1 回開き、`load_candidates` と
  `linked_results` に貸与する(関数シグネチャの変更)
- chunk 取得の `prepare` はループ外に出す。N+1 の IN 句化は
  行わない(32 行では効果がなく、複雑化に見合わない)

## 互換性

- **出力形式**: agent 形式は変わる。context 出力は API ではなく
  CLI 出力であり、既知の消費者は skill 経由のエージェントのみの
  ため、バージョン番号などは設けない。CLI ヘルプと README の
  記述を実態に合わせて更新する
- **human 形式**: 変更なし(タスクエコーの短縮のみ影響)
- **スキーマ・Markdown**: 変更なし。マイグレーション不要

## 実装ステップ

早く効果が出る順に並べる。各ステップは独立してマージ可能。

1. **M1: 即効の削減(発見 2, 4)**
   - タスクエコーキャップ、agent 形式の定型行圧縮、フッター導入
   - CLI ヘルプ・README 更新、統合テスト期待値の更新
2. **M2: admission cap と share 繰り越し(発見 1, 3)**
   - `admission_cap` 導入、Sources フッター、carry 実装
   - バジェット別(64 / 500 / 2500 / 10000)の挙動テスト
3. **M3: 文書順維持(発見 5)**
   - dedup 方式変更、ソートキー変更、順序検証テスト
4. **M4: release 保証(発見 6)**
   - 脱出口の廃止、最終防衛線、budget 総当たりテスト
5. **M5: 無駄の除去(発見 7)**
   - コネクション・prepare・query_terms の整理(挙動不変、
     スナップショットで同一出力を保証)

## テスト計画

- **配分の発動検証**: 既定バジェットで上位エントリの evidence が
  最低保証(40 トークン)を上回ることを検証する(現状は退化して
  いて上回らない — このテストが M2 の受け入れ条件)
- **バジェット遵守の総当たり**: budget を 64〜4000 まで刻みで回し、
  `estimated_tokens <= budget * 9 / 10` を release 相当の
  アサーションで検証(既存 `truncation_uses_boundaries...` の拡張)
- **文書順**: 複数文のセクションで、出力順が入力順と一致すること
- **Sources フッター**: cap 超過の候補が ID として列挙されること、
  極小バジェットでは行ごと消えること
- **スナップショット更新**: 出力形式変更に伴う `tests/cli.rs` の
  期待値をレビューしやすい単位(M1 / M2)で分けて更新する

## 完了条件

- 既定バジェットでの `belay context` 出力において、
  定型行(ヘッダー・ラベル・フッター)の占める比率が
  30% 未満になること
- 上位 3 エントリの evidence 量が下位エントリより有意に多いこと
  (ランク加重の発動)
- 全バジェット値で出力が selection 枠を超えないこと(release)

## リスク

- **出力形式変更によるエージェントの混乱**: skill / snippet の
  記述が旧形式を前提にしていれば同時に更新する
- **admission cap の過小**: 小バジェット + 高関連候補多数の場合に
  有用な候補が Sources 行へ落ちる。`MIN_ADMITTED = 3` と
  Sources 行の存在で回復手段(belay show)は残る
