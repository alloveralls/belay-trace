# Phase 6 開発計画: Delivery Assurance

[ロードマップ](../roadmap.md) Phase 6 の実装計画。
人間の曖昧な依頼を Intent Brief として暫定構造化し、Goal 達成までの
Delivery Map を AI と人間の共通の現在地として維持する。

Belay がすでに保持する Goal / Plan / Decision / Work / Evidence を使い、
作業中と完了時に「何が定義済み・実装済み・検証済みで、何が未解決か」を
継続的に照合する。新しい Issue tracker や LLM 内蔵機能は作らない。

## 目的

次の失敗を、完了後ではなく作業中に発見できるようにする。

- 人間の発言に不足していた前提を AI が黙って補完する
- Plan はあるが、実装の現在地と一致しなくなる
- 実装済みを検証済みと誤認する
- Success Criterion に対応する実装または検証が漏れる
- 実装中に変わった仕様やスコープが記録されない
- テストが通ったことだけを根拠に、Goal 達成を宣言する

## ロードマップとの対応

- **完了条件**: Tier 2 以上の dogfooding で Intent Brief と Delivery Map が
  標準利用され、現在地の確認、実装漏れの発見、仕様解釈の修正のいずれかに
  実際に使われる
- **見直し条件**: 更新コストが高く地図が恒常的に stale になる場合、
  Delivery Map を Success Criterion ごとの未充足一覧へ縮小する
- **撤退条件**: 通常の Plan と Coverage だけで同等の判断ができ、
  追加構造が意思決定または手戻り削減に寄与しない場合、独立機能化しない

## 依存

- Phase 1(Goal)— 必須
- Phase 3(Evidence)— 必須
- Phase 4(Goal Coverage)— 推奨
- Phase 5(Context compiler)— 推奨
- Phase 2(Goal Engineering)— 推奨。Intent Brief から Goal を改善するときに使う

## 設計原則

### 1. 人間の完全な言語化を前提にしない

エージェントは「要件を詳しく説明してください」と白紙を返さず、まず
人間の発言から Intent Brief の解釈案を作る。人間はゼロから仕様を書くのではなく、
解釈案の誤りと不足を訂正する。

曖昧さは消去せず、Fact / Assumption / Unknown / Human decision に分ける。
可逆的で影響の小さい Assumption は明示して進め、結果を大きく変える判断、
セキュリティ、データ破壊、外部契約は人間の確認まで Blocked とする。

### 2. Plan と現在地を分ける

- Goal: 達成したい状態
- Intent Brief: 人間の発言に対する現在の解釈
- Plan: 進め方についての仮説
- Delivery Map: Goal に対する現在の達成状況
- Work: 実際に行った変更
- Evidence: 達成を支える根拠

Plan の変更は失敗ではない。変更された Plan、Intent Brief、Delivery Map が
互いに不整合なまま残ることを失敗として扱う。

### 3. 実装と検証を分ける

Delivery Map では `implemented` と `verified` を別状態にする。
コードの存在、テストの存在、passing Evidence、最終的な人間の受入を
同じ「完了」に畳み込まない。

### 4. タスク管理製品にならない

Delivery Map の Task は、担当者の細かな行動ではなく、Success Criterion を
満たすための観測可能な成果物または検証単位とする。期限、工数、バックログ、
スプリント、担当者負荷の管理は Issue tracker に任せる。

### 5. 意味判断と決定的検査を分ける

- Agent skill: 意図解釈、問い返し、タスク分解、意味的な再照合
- Belay core: 必須欄、ID、状態遷移、リンク、Evidence の有無の検査
- Human: 価値判断、重要な Assumption、最終受入

## 初期データ形式

Phase 6.1 では新しい entry type や SQLite schema を追加しない。
Intent Brief と Delivery Map は Plan 本文の規約として表現する。

### Intent Brief

Tier 2 以上では実装開始前に必ず作る。

```markdown
## Intent Brief

### Problem
- 現在地と実装漏れを把握しにくい。

### Desired Outcome
- 作業途中と完了時に Goal 達成状況を説明できる。

### Success Signals
- 各 Success Criterion に Task と検証方法がある。
- 未実装と未検証を区別できる。

### Constraints
- Belay 本体は LLM を呼ばない。

### Non-goals
- Issue tracker を置き換えない。

### Assumptions
- 小さく可逆的な判断は明示した上で進めてよい。

### Unknowns / Decisions Needed
- 人間の確認を必須にする checkpoint の範囲。
```

各セクションは必須とするが、該当事項がない場合は `None identified` と書ける。
空欄を許すと「検討したがない」と「検討していない」を区別できないためである。
数値の confidence score は疑似精度を生むため導入しない。

### Delivery Map

Task ID は Plan 内で安定させ、並べ替えや状態変更で再採番しない。

```markdown
## Delivery Map

| ID | Goal item | Outcome / Task | Actor | State | Verification / Evidence |
| --- | --- | --- | --- | --- | --- |
| T-1 | SC-1 | 競合時の動作を設計する | AI | verified | DEC-... |
| T-2 | SC-1 | 双方変更を検出する | AI | implemented | test pending |
| T-3 | SC-1 | 上書きしないことを検証する | AI | not-started | cargo test ... |
| T-4 | SC-2 | 既定動作を決める | Human | blocked | human decision |
```

初期状態は次の 6 種に限定する。

- `not-started`
- `in-progress`
- `blocked`
- `implemented`
- `verified`
- `dropped`

`dropped` には理由と承認元を残す。Task を削除して未完了を隠さない。

## 標準ワークフロー

### Frame

1. `belay context compile` で関連 Goal、Decision、制約、失敗履歴を取得する
2. ユーザー発言と既存文脈から Intent Brief の解釈案を作る
3. 結果を大きく変える Unknown だけを、推奨解釈と影響を添えて質問する
4. 可逆的な Assumption は明示して次へ進む

### Map

1. Goal の各 Success Criterion に安定した ID があることを確認する
2. Criterion ごとに成果物 Task と検証 Task を定義する
3. Task ごとに Actor、State、Verification を設定する
4. どの Criterion にも対応しない Task は必要性を説明する
5. Plan と Delivery Map を人間へ提示し、Tier に応じた承認を得る

### Execute

1. Delivery Map の Task ID を現在の作業単位として使う
2. 実装中に発見した Task、Unknown、Assumption を黙って処理せず追記する
3. Work と Evidence を該当 Goal item へリンクする
4. コードが存在するだけなら `implemented`、passing Evidence があれば
   `verified` とする

### Reconcile

次の checkpoint で Intent Brief、Goal、Delivery Map、実際の diff、Evidence を
再照合する。

- 意味のある Task が完了したとき
- 新しい仕様、制約、リスクを発見したとき
- 当初の設計またはスコープを変更したとき
- セッションの中断、圧縮、handoff の前
- ユーザーから現在地を求められたとき
- 完了を宣言する前

固定形式で次を報告する。

```text
Current state
- verified: 2/5
- implemented, unverified: 1/5
- in progress: 1/5
- blocked: 1/5

Goal coverage
- SC-1: verified
- SC-2: partial
- SC-3: not started

Changed assumptions
- ...

Human decisions needed
- ...

Next action
- ...
```

### Assure

完了前に fresh context で次を監査する。

- 全 Success Criterion に Task と検証結果がある
- `implemented`、`blocked`、重要な Unknown が完了扱いされていない
- Evidence が対象 Criterion を実際に検証している
- diff が Intent Brief の Constraints / Non-goals と矛盾しない
- 仕様変更と dropped Task に理由と承認がある
- 人間が最終成果物と意図の一致を受け入れている

## 段階的ロードマップ

### Phase 6.1: Agent-first MVP

**狙い**: CLI や schema を増やす前に、Intent Brief と Delivery Map が
実際の迷子、漏れ、誤解を減らすか検証する。

初回リリースに必ず含める。

- Tier 2 / 3 で Intent Brief を必須化する
- Plan 本文に Delivery Map を作る
- `belay-trace` skill に Frame / Map / Execute / Reconcile / Assure を追加する
- AGENTS snippet に checkpoint と fresh-context completion review を追加する
- 現在地を固定形式で報告する
- Goal item と Work / Evidence の既存リンク機能を利用する
- 新しい entry type、DB schema、Task CRUD コマンドは作らない

**自由度**:

- Intent Brief の意味的解釈と Task 分解は中程度の自由度
- 必須セクション、State、完了条件は低い自由度
- 実装方法と専門設計判断は高い自由度

**完了条件**:

- Tier 2 以上の dogfooding 5 件で、実装開始前に Intent Brief が作られる
- 全件で Success Criterion と Delivery Map が対応付けられる
- checkpoint で AI と人間が同じ現在地を確認できる
- 少なくとも 1 件で、Assumption、未実装、未検証、仕様差分のいずれかを
  完了前に発見する

**見直し条件**:

- 5 件中 2 件以上で Map が実態から stale になる場合、更新 checkpoint と
  Map の粒度を見直す
- 人間の訂正が Intent Brief の文章校正だけに留まり、実装判断に影響しない場合、
  Brief の項目を削減する

### Phase 6.2: Deterministic lint

**狙い**: MVP で有効だった規約だけを Belay core の検査へ移す。

- `belay plan lint <plan-id>` を追加する
- Intent Brief の必須セクションと空欄を検査する
- Delivery Map の列、Task ID の一意性、State を検査する
- Goal item のない Task、Verification のない実装 Task を警告する
- `verified` なのに Evidence を参照しない Task を警告する
- `dropped` なのに理由または承認元がない Task を警告する
- JSON 出力と `--strict` を提供する

意味的な Goal 妥当性、Task 十分性、Evidence の内容は判定しない。

**完了条件**:

- dogfooding で lint が実際の欠落または状態不整合を検出する
- 同じ Plan から常に同じ結果を出す
- 誤検知を無視する運用が常態化しない

**見直し条件**:

- Markdown table の解析が壊れやすい場合、専用構文を増やす前に
  checklist 形式または埋め込み JSON との比較実験を行う

### Phase 6.3: Reconciliation report

**狙い**: AI と人間が現在地を同じ入力から再生成できるようにする。

- `belay reconcile <goal-id>` を追加する
- Goal item → Task → Work → Evidence の状態を集約する
- `not-started` / `implemented but unverified` / `blocked` を列挙する
- stale Evidence を verified と数えない
- changed Assumptions、Unknowns、Human decisions を表示する
- `context compile` の task-start / review 出力へ要約を含める

初期版はレポート専用とし、自動で Plan や State を書き換えない。

**完了条件**:

- fresh session が `belay reconcile` だけで現在地と次の不足を説明できる
- handoff 後のエージェントが broad scan なしで作業を再開できる
- 手作業で数えた状態と CLI 集計が一致する

**見直し条件**:

- Delivery Map と既存 Goal Coverage が実質的に同じ出力になる場合、
  新コマンドを作らず `belay coverage --detail` へ統合する

### Phase 6.4: Completion gate

**狙い**: 「実装した」から「Goal を達成した」への完了判定を明確にする。

- review profile に Intent Brief、Delivery Map、diff、Evidence を含める
- fresh-context reviewer 用の決定的な入力 bundle を生成する
- 未検証、Blocked、重要な Unknown、未承認の dropped Task を列挙する
- human acceptance Evidence を Goal または Plan に記録する
- CI 向けに、重大な未充足がある場合のみ失敗する gate を提供する

初期 gate は opt-in とする。警告の精度と運用コストが確認されるまで、
すべてのリポジトリでマージ必須にはしない。

**完了条件**:

- gate が少なくとも 1 件の早すぎる完了宣言を防ぐ
- false positive による恒常的な bypass が発生しない
- human acceptance の actor、時刻、source、scope を追跡できる

**見直し条件**:

- gate が形式的なリンク追加で通過できる場合、CI blocking を外し、
  未充足一覧と human review に戻す

### Phase 6.5: Eval and optional productization

**狙い**: 有効性を再現可能に評価し、必要な部分だけ製品化する。

- 過去または合成した Tier 2 / 3 タスクを固定評価セットにする
- skill なし / ありを fresh context で複数回実行する
- Intent Brief の重要な訂正率、漏れの発見率、手戻り、完了誤判定、
  token / time overhead を測る
- raw prompt、出力、diff、Evidence、評価理由を保存する
- 評価で必要性が実証された場合のみ、Task の first-class model を検討する

**完了条件**:

- 事前に定義した評価基準で、品質または現在地把握の改善が確認できる
- 改善が特定モデル、特定タスク、漏れた事前文脈だけに依存しない
- 運用コストを含めて継続採用の判断ができる

**見直し条件**:

- 結果の一貫性だけが上がり、正確性または漏れ検出が改善しない場合、
  Skill の手順を強化せず Intent / Evidence の内容を再検討する

## 実装順序

1. **M1: Intent Brief と Delivery Map の規約確定**
   - 必須項目、Task 粒度、State、問い返し境界
2. **M2: Agent integration MVP**
   - shared skill / generated skills / AGENTS snippet / tests の更新
3. **M3: Dogfooding 5 件**
   - Brief の訂正、Map の stale、漏れ発見、運用負荷を記録
4. **M4: `plan lint`**
   - dogfooding で有効だった決定的検査のみ実装
5. **M5: reconciliation report**
   - Goal / Task / Work / Evidence の現在地を集約
6. **M6: completion gate**
   - fresh review bundle と opt-in CI gate
7. **M7: forward eval**
   - fresh context で skill なし / ありを比較

M4 以降は M3 の結果を見て再設計できる。初期版で M4〜M7 を先取りしない。

## 測定

### Primary

- 完了前に発見した未実装 Success Criterion 数
- 完了前に発見した未検証 Task 数
- 人間が修正した重要 Assumption / Intent 解釈数
- stale な Plan / Delivery Map による迷子の発生件数
- Goal 達成前の誤った完了宣言数

### Guardrail

- Intent Brief と Map の作成・更新時間
- 1 Task あたりの trace entry 増加数
- stale Map の割合
- 人間への質問数と、実装判断に影響した質問の割合
- token 使用量

数値目標はベースラインがないため現時点では **Unknown.** とする。
Phase 6.1 の最初の 5 件でベースラインを取り、Phase 6.2 以降の採否基準を決める。

## テスト計画

### Phase 6.1

- Skill の trigger 記述が Tier 2 / 3 の例で起動すること
- Intent Brief に Assumption / Unknown が残る例を forward-test する
- reversible / irreversible の問い返し境界を事例で確認する
- generated Codex / Claude skill と shared source の一致を検証する

### Phase 6.2 以降

- Intent Brief の必須セクション全組み合わせ
- Delivery Map の重複 ID、未知 State、欠落 Verification
- Goal item fragment、Evidence freshness、dropped approval
- 同一入力に対する lint / reconcile のバイト一致
- 日本語・英語混在 Plan
- Map に循環・大量 Task があっても停止し budget を守ること

## リスク

- **整然とした誤解**: AI が誤った Intent Brief と Map を一貫して維持する。
  人間の訂正と fresh-context review を外さない
- **儀式化**: 小変更に適用すると運用コストが価値を上回る。Tier 2 / 3 に限定する
- **Issue tracker 化**: Task CRUD、期限、工数、担当者管理へ拡張しない
- **Map の stale 化**: checkpoint を増やすだけで解決しない場合、Task 粒度を粗くする
- **疑似精度**: 完了率や confidence score を品質の代理にしない。
  未充足一覧と Evidence を主表示にする
- **自己検証**: 実装者と reviewer が同じ文脈を共有すると誤解を検出しにくい。
  完了監査は fresh context を最低線とする
- **Skill trigger の非決定性**: Tier 2 / 3 での起動要求は skill description
  だけに依存せず、AGENTS snippet にも置く

## Non-goals

- 人間の曖昧さを完全に消すこと
- AI が事業上の価値判断を代行すること
- Issue tracker、project management、sprint board の置き換え
- Belay 本体から LLM を呼ぶこと
- すべての設計知識を一つの Skill に収録すること
- Task 完了率を品質スコアとして扱うこと

## オープンクエスチョン

- Intent Brief と Delivery Map を同じ Plan に置くか、Goal 本文の一部にするか。
  Phase 6.1 では同じ Plan に置き、独立して参照する需要を観測する
- 人間による Intent Brief の明示承認を Tier 2 でも必須にするか。
  初期版は Tier 2 では訂正機会の提示、Tier 3 では明示承認を必須とする
- Delivery Map の Task を first-class object にするか。
  Phase 6.5 の評価まで保留する
- `reconcile` を新コマンドにするか、`coverage --detail` に統合するか。
  Phase 6.1 の出力利用実績を見て決める
