# Phase 4 開発計画: Goal Coverage

[ロードマップ](../roadmap.md) Phase 4 の実装計画。
「人間の意図がどこまでシステム全体に反映されているか」を
計測・可視化する。

## 目的

Goal の各項目が Decision / Work / Evidence へどこまで展開されて
いるかを集計し、未カバー項目を次のアクションへ繋げる。
リンクの存在(traceability)と Evidence の裏付け(verified)を
明確に区別し、ゲーム可能な指標にしない。

## ロードマップとの対応

- **完了条件**: dogfooding で verified coverage の低下が
  リリース前の実際の対応(テスト追加・Goal 修正)を誘発した
  事例が確認できる
- **見直し条件**: 数値が意思決定に使われないなら、集計サマリーを
  やめ未カバー項目のリスト表示のみに縮小する

## 依存

- Phase 1(Goal、`fulfills` / `supports`)— 必須
- Phase 3(Evidence、鮮度判定)— verified coverage に必須。
  Phase 3 完了前は traceability coverage のみ提供できるが、
  誤った安心感を避けるため **Phase 3 完了後にリリースする**

## スコープ

### In

- `belay coverage` コマンド(entry 単位 → 項目単位の 2 段階)
- traceability / verified の 2 系統の集計
- `--fail-under` による CI ゲート用終了コード
- 未カバー項目のアクション可能な一覧表示

### Out

- ダッシュボード UI(収益化フェーズのホスト型製品で扱う)
- PR 単位の差分 coverage(GitHub App 側の将来機能)

## 設計

### 1. カバレッジの定義

集計対象は status が `Active` の Goal。`Draft` は対象外、
`Completed` は `--include-completed` でのみ含める。

2 系統を常に併記し、単一の数値では出さない。

| 系統 | 定義 |
| --- | --- |
| traceability | 対象へのリンクが存在する |
| verified | fresh かつ pass の Evidence が存在する |

次元は Evidence の kind とリンク先の type から導出する。

| 次元 | traceability の条件 | verified の条件 |
| --- | --- | --- |
| decision | fulfills 元に Decision | 同左 + その Decision に fresh Evidence |
| implementation | fulfills 元に Work | 同左 + fresh pass の test/ci Evidence |
| test | verifies に kind=test | fresh かつ pass |
| monitoring | verifies に kind=metric | fresh |

- 「fresh」は Phase 3 の鮮度ポリシーに従う
- stale / fail の Evidence は verified に数えず、
  一覧では理由付き(stale / failing)で表示する

### 2. 段階 1: entry 単位の coverage(M1–M2)

まず Goal エントリ単位で集計する。

```sh
belay coverage
belay coverage GOAL-...-reliable-sync
belay coverage --format json
belay coverage --fail-under verified=60
```

出力例:

```text
Active goals: 8

               traceability   verified
decision            7/8          5/8
implementation      7/8          4/8
test                6/8          4/8
monitoring          5/8          2/8

Uncovered (verified):
  GOAL-...-fast-context   test: evidence stale (34 commits behind)
  GOAL-...-safe-rebuild   implementation: no fulfills work entry
```

- `--fail-under` 未達時は終了コード `4`(CI ゲート用)。
  それ以外の指摘は `0`

### 3. 段階 2: 項目単位の coverage(M3–M4)

Goal の Success Criteria セクションの箇条書きを項目として扱う。

- パーサは Success Criteria 内のトップレベル list item を抽出し、
  正規化テキストのハッシュ短縮形で **項目 ID** を導出する
  (`GOAL-...-example#sc-3f9a` の形式)
- リンクと Evidence は項目 ID を対象にできるようにする
  - `belay link WRK-... GOAL-...#sc-3f9a --relation fulfills`
  - `belay verify record --verifies GOAL-...#sc-3f9a`
- 項目 ID の安定性: 項目テキストを編集するとハッシュが変わり
  リンクが宙に浮く。`belay doctor` が「宙に浮いた項目リンク」を
  検出し、`belay coverage --repair` が現在の項目一覧との
  対応候補を提示する(自動再接続はしない)
- 項目単位を導入後も entry 単位の集計は残し、
  項目リンクがない Goal は entry 単位へフォールバックする

ロードマップの出力例(`Goal items: 8` …)はこの段階で実現する。

### 4. モジュール構成

- `src/coverage.rs`: 集計エンジン(リンクグラフ + evidence 参照)
- 項目 ID のパースは `src/markdown.rs` の chunk 化と同じ
  セクション認識を再利用する
- `entry_links` / `evidence_links` の対象カラムは display_id +
  任意の項目フラグメントを格納できるよう、Phase 1 / 3 の
  スキーマ設計時に TEXT で持たせておく(要 Phase 1/3 との調整)

### 5. 表示の原則

- 集計サマリーより **未カバー項目の一覧** を主役にする
  (見直し条件で「サマリー廃止・一覧のみ」に縮小できる構造)
- 各未カバー項目に「次にやること」(リンクを張る / テストを書く /
  Evidence を更新する)を 1 行で併記する

## 実装ステップ

1. **M1: 集計エンジン(entry 単位・traceability)**
   - リンクグラフ走査、次元導出、表駆動テスト
2. **M2: verified 系統と CI ゲート**
   - Phase 3 の鮮度判定を接続、`--fail-under`、JSON 出力
3. **M3: 項目 ID とパーサ**
   - Success Criteria の抽出、ID 導出、doctor の宙リンク検出
4. **M4: 項目単位の link / verify / coverage**
   - フラグメント付きリンクの CLI 受理、フォールバック集計
5. **M5: dogfooding とリリース判定**
   - belay-trace のリリース前チェックに `coverage --fail-under` を
     組み込み、完了条件(実際の対応を誘発)を観測する

## テスト計画

- 集計は「グラフ構成 → 期待値」の表駆動テストを網羅的に
  (リンクなし / stale のみ / fail のみ / 複数 Evidence 混在)
- 項目 ID: 編集による ID 変化、doctor 検出、repair 候補提示
- `--fail-under` の閾値境界と終了コード
- JSON スキーマの互換性テスト

## リスク

- **ゲーム化**: リンクを張るだけで traceability が上がる。
  verified を常に併記し、CI ゲートには verified のみ使えるよう
  制約することで軽減する
- **項目 ID の脆さ**: テキスト編集でリンクが切れる。自動再接続は
  誤接続の温床なので行わず、doctor + repair 候補提示に留める
- **次元定義の恣意性**: monitoring 次元は当面 kind=metric 頼み。
  dogfooding で実態に合わなければ次元を config で調整可能にする

## オープンクエスチョン

- Decision にも coverage(accepted Decision が Evidence で
  支えられているか)を出すか。M2 で `--type decision` として
  試験的に出し、有用性を dogfooding で判断する
