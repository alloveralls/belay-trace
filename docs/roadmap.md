# Belay Roadmap

このドキュメントは、AI コーディングエージェント時代における Belay の長期的な方向性と、
そこへ至る段階的なロードマップを整理したものである。

各フェーズは確定した計画ではなく **検証すべき仮説** である。そのため各フェーズに
完了条件と見直し条件を付け、検証戦略(後述)に従って進める。

## ミッション

**人間の意図と AI の実行を繋ぎとめる。**

> Ensure that AI always works with the right intent, the right context,
> and the right evidence.
>
> AI が常に、適切な意図・適切な文脈・適切な証拠に基づいて働けるようにする。

belay は登山用語で「相手を確保する、安全索で繋ぎとめる」を意味する。
Belay は、AI が速く遠くへ進むほど、人間の意図・制約・証拠・判断履歴から
切り離されないようにするための安全索であり、アンカーであり、協働の接続基盤である。

## 背景と問題意識

AI コーディングエージェントの進化により、コード生成だけでなく、テスト生成、レビュー、
CI 修正、設計提案も自動化されつつある。品質保証の重心は次のように移っていく。

| 従来 | これから |
| --- | --- |
| 人間がコードを書く | 人間が Goal を定義する |
| 人間がコードをレビューする | AI が実装し、AI がテストを書く |
| 人間がテストを書く | CI や Evals が検証する |
| — | 人間は Goal・制約・判断・証拠の妥当性を監督する |

### テストも AI が書くなら、テストだけでは足りない

AI が実装を書き、AI がテストも書く場合、テストが通ることは「人間の意図した成果物に
到達した」ことを必ずしも意味しない。テストは仕様に対する検証にはなるが、
仕様そのものが間違っていれば、正しいテストでも間違った成果物を保証してしまう。

したがって、より重要になるのは **Goal / Specification の品質** である。
コード生成・テスト生成はコモディティ化していく一方、人間の意図を AI が誤解しにくく、
かつ検証可能な形へ構造化する技術 — **Goal Engineering** — はまだ十分に
ツール化されていない。Belay はここを支援する。

### 「正しい Goal」の性質

Goal の正しさは絶対的・数学的な正しさではない。AI に人間の意図したゴールを正確に
伝えられ、人間が求める最終成果物に辿り着けるかどうかが問題である。
良い Goal は次の性質を持つ。

- 曖昧さが少ない
- 検証可能である
- 成功条件が明確である
- 制約条件が明示されている
- 非ゴールも明示されている
- 既存の Decision や Context と矛盾しない
- 実装・テスト・運用監視へ展開可能である

## 将来モデル: Goal → Decision → Work → Evidence

Belay の将来のドメインモデルは次のように整理できる。

```text
Human Intent
  ↓
Goal / Specification
  ↓
Decision
  ↓
Work / Implementation
  ↓
Evidence
  ↓
Memory / Learning
```

この構造により、単に「何をやったか」ではなく、以下を追跡できる。

- なぜその変更が必要だったか
- どの Goal に基づくか
- どの Decision に従ったか
- どの Evidence で正しさを支えているか
- 後から何を学習すべきか

## ポジショニング: 隣接ツールとの違い

Goal を第一級にする方向は、既存の複数のツール領域と重なる。差別化の核を
先に明確にしておく。

- **Issue tracker(Jira, GitHub Issues など)との違い**:
  Issue はタスクの進行管理が目的で、完了すれば閉じられて終わる。
  Belay の Goal は Decision・Work・Evidence へ型付きリンクで接続され、
  完了後も「なぜ・何を根拠に」を辿れる知識グラフとして残る。
  進行管理は Issue tracker に任せ、Belay は意図と証拠の追跡に徹する。
- **ADR(Architecture Decision Records)との違い**:
  ADR は Decision 単体の記録であり、Goal や Evidence への接続を持たない。
  Belay の Decision は ADR 相当の内容を、Goal(なぜ)と
  Evidence(正しさの根拠)の両側へリンクできる点が異なる。
- **Spec-driven development ツール(GitHub spec-kit、Kiro の spec mode
  など)との違い**: これらは「仕様 → 実装」の前方向の生成フローに重点がある。
  Belay は生成フローそのものは持たず、仕様・判断・実装・検証結果を
  **事後も含めて双方向に追跡できる永続レイヤー** を提供する。
  生成フロー系ツールと組み合わせて使える設計を保つ。

共通する差別化の核は、**ローカルで Git と一緒に動き、Goal から Evidence までが
型付きリンクで繋がり、決定的に再構築できる** ことである。

## 設計原則との整合: LLM をどこに置くか

v1 の Belay は local-first・deterministic を設計原則とし、embeddings を
必要としない。Phase 2 以降の Goal lint / improve や context compile は
意味的な推論を含むため、この原則との整合を先に決めておく。

**方針: Belay 本体は LLM を呼ばない。**

- Belay 本体が担うのは **決定的にチェック・構造化できる部分** に限る。
  必須フィールドの有無、リンクの欠落、曖昧語辞書によるフラグ、
  スキーマ検証、カバレッジ集計など。
- 意味的な評価・改善(「この成功条件は本当に検証可能か」など)は、
  Belay が評価ルーブリックと構造化された入出力を提供し、
  **エージェント側の LLM に実行させる**。これは v1 が skill
  (`belay init --install-skill`)として統合を配っている路線の延長である。
- この分担により、Belay 本体は API キー管理・非決定性・推論コストを
  持ち込まず、オフラインでも決定的な部分は常に動作する。

Phase 5 の context compiler も同様に、BM25 とリンクグラフによる
決定的な選択を維持し、LLM による要約・圧縮はエージェント側に任せる。

## 検証戦略

このロードマップは仮説の列である。次の方法で検証する。

- **Dogfooding を一次検証とする**: belay-trace 自身の開発を Belay で
  トレースする。各フェーズの機能は、まず自プロジェクトで一定期間
  運用して有効性を確認してから確定させる。
- **各フェーズに完了条件を置く**: 機能が存在することではなく、
  実際の開発判断で使われたことを完了の基準にする。
- **各フェーズに見直し条件を置く**: 仮説が外れたと判断する基準を先に
  書いておき、外れた場合は縮小・再設計・撤退を明示的に選ぶ。

## フェーズ

フェーズ番号は依存関係を示すもので、厳密な直列実行を意味しない。
Phase 1(Goal)と Phase 3(Evidence)は独立に価値が出るため並行可能である
(Evidence は既存の Work / Decision にも紐づけられる)。
Phase 2 は Phase 1 に、Phase 4 は Phase 1 と Phase 3 の両方に依存する。

### Phase 0: Traceability foundation(現在 / v1)

記録ツールとしての基盤。ここから始めるのが正しい。

- [x] Entry types: Plan / Decision / Work / Review / Note
- [x] Typed links(`implements` など)
- [x] SQLite operational store + deterministic Markdown mirror(`belay sync`)
- [x] FTS5/BM25 検索(`belay search`)
- [x] バジェット付き context 生成(`belay context`)
- [x] Export(Markdown / JSON / NDJSON)
- [x] `belay doctor` / `belay rebuild`
- [x] Agent 統合(`AGENTS.md` snippet、Claude/Codex skill)

このフェーズの価値は Traceability。ただし品質保証という意味ではまだ弱い。

残作業として、`belay context` の budget 配分の構造的な弱点
(広く浅くへの退化、固定オーバーヘッド、出力順序)の修正を
Phase 1 に先行して行う。詳細は
[docs/design/phase0.md](design/phase0.md) を参照。

### Phase 1: Goal as a first-class object

Goal を第一級の entry type として導入し、意図を構造化して記録できるようにする。

- **`belay add goal`**: Goal を entry type に追加する
- **Goal スキーマ**: 成功条件、制約、非ゴール、検証方法、リスク考慮などの
  構造化フィールドを持たせる
- **Typed links の拡張**: Goal → Decision → Work → Evidence の関係を
  リンクとして表現できるようにする(`fulfills`, `supports`, `verifies` など)
- **既存機能との統合**: search / context / export / sync / doctor が
  Goal を自然に扱えるようにする
- **マイグレーション**: entry type の追加は SQLite スキーマと managed
  Markdown 形式の変更を伴う。スキーマバージョンを明示し、既存
  リポジトリは `belay rebuild` で新スキーマへ移行できることを保証する。
  旧形式の Markdown は読み取り互換を維持する

**完了条件**: belay-trace 自身の開発を Goal 起点で 1 ヶ月以上運用し、
Goal → Decision → Work のリンクがレビューや判断で実際に参照される。

**見直し条件**: 運用して Goal が Plan と実質的に区別されず形骸化する場合、
独立した entry type をやめ、Plan の構造化拡張として再設計する。

### Phase 2: Goal Engineering support

Goal は書くだけでは不十分で、レビュー・改善・検証されるべきである。
「設計原則との整合」の方針に従い、決定的なチェックは Belay 本体、
意味的な評価はエージェント側 LLM が担う。

- **`belay goal lint`**: Goal の品質をチェックする。本体は必須フィールドの
  欠落・リンク切れ・曖昧語辞書によるフラグなど決定的な検査を行い、
  意味的な評価用に構造化されたルーブリックを出力する

  ```text
  Goal Score: 72/100

  Missing:
  - Success metrics
  - Rollback condition
  - Monitoring requirement
  - Security consideration
  - Edge cases
  ```

- **`belay goal improve`**: Goal の改善をエージェントに依頼するための
  構造化プロンプトと文脈を生成する
  (現在の Goal / 改善観点 / 関連する Decision / 未解決の質問)

- **評価軸**:
  - Completeness: 抜け漏れがないか
  - Consistency: 矛盾していないか
  - Testability: 検証可能か
  - Observability: 成功・失敗を観測できるか
  - Ambiguity: 曖昧語が残っていないか
  - Business Alignment: 本当に目的達成につながるか
  - Risk Awareness: セキュリティ・性能・法務・運用リスクが考慮されているか
  - Context Fit: 過去の Decision、既存設計、制約と整合しているか

- **Human-AI interface としての問い返し**: Belay は人間に対しても次を問えるべき。
  - 本当に解きたい問題は何か
  - その制約は必須か
  - 成功とは何か
  - 何を作らないのか
  - 以前の Decision と矛盾していないか
  - この Goal で本当に目的を達成できるのか

**完了条件**: dogfooding で lint の指摘が Goal の実際の修正につながった
事例が継続的に発生する(指摘したが無視される状態が常態化しない)。

**見直し条件**: スコアや指摘が形式的に消化されるだけで成果物の品質に
影響しない場合、スコア表示をやめ、チェックリスト提示のみに縮小する。

### Phase 3: Evidence / Verification layer

テストランナーを作るのではなく、既存の検証基盤(cargo test, pytest, GitHub Actions,
E2E tools, linters, security scanners, benchmarks, LLM eval frameworks)の結果を
**Evidence として集約し、Goal や Decision へ関連付ける**。
`belay-test` ではなく `belay-verify` 的発想である。

- **`belay verify`**: Goal / Decision / Work に対する Evidence を集約する。
  取り込み経路は、CLI からの手動登録、テスト結果ファイル
  (JUnit XML など)の取り込み、CI からの登録を想定する
- **Evidence の種類**(テスト以外も含む):
  - Unit / Integration / E2E / Property / Mutation test result
  - Benchmark、Linter result、Security scan、Type check、CI run
  - Human approval、LLM rubric evaluation
  - Screenshot diff、Production metric、Error rate、Rollback result
- **鮮度(freshness)を第一級で扱う**: Evidence はコードが変わった瞬間に
  古くなる。各 Evidence に取得時刻と対象コミット SHA を必須で紐づけ、
  現在の HEAD との乖離から stale を判定する。`belay doctor` は
  stale な Evidence に依存している Goal / Decision を検出して報告する
- **モデル化の原則**: Evidence は単なるログではなく
  「Goal や Decision を支える証拠」としてモデル化する。
  Evidence は不変(append-only)とし、更新は新しい Evidence の追加で表す

**完了条件**: dogfooding で CI の結果が Evidence として取り込まれ、
`belay doctor` が stale Evidence を検出できる状態で 1 ヶ月以上運用される。

**見直し条件**: Evidence の登録・維持のコストが得られる確信を上回る場合、
取り込み対象を CI 結果と human approval のみに絞る。

### Phase 4: Goal Coverage

コードにテストカバレッジがあるように、Belay には Goal Coverage があってよい。
「コードの何行が実行されたか」ではなく、
「**人間の意図がどこまでシステム全体に反映されているか**」を測る指標。

```text
Goal items: 8
Decision coverage: 7/8
Implementation coverage: 7/8
Test coverage: 6/8
Monitoring coverage: 5/8
Overall Goal Coverage: 62%
```

- **カバレッジの定義を厳密にする**: リンクが存在するだけでは
  「カバーされた」と数えない。リンク先に **fresh かつ passing な
  Evidence** が存在する場合のみカバー済みとする。リンク存在ベースの
  数値(traceability coverage)と Evidence 裏付けベースの数値
  (verified coverage)は区別して表示する
- リンクを張るだけで数値が上がる指標はゲーム可能であり、
  「Goal scoring の疑似精度」と同じ罠に陥ることを設計上の前提とする
- 未カバー項目を可視化し、次のアクションへ繋げる

**完了条件**: dogfooding で verified coverage の低下がリリース前の
実際の対応(テスト追加・Goal 修正)を誘発した事例が確認できる。

**見直し条件**: 数値が意思決定に使われずダッシュボード化するだけなら、
集計サマリーをやめ、未カバー項目のリスト表示のみに縮小する。

### Phase 5: Context compiler & Human-AI interface

AI が賢くなるほど、ボトルネックは人間自身が意図を明確に言語化できていないこと、
そして AI へ渡す文脈の選別になる。

- **`belay context compile`**: AI に渡すべき Context を圧縮・選別して生成する
  - 関連する Goal
  - 関連する Decision
  - 過去の失敗
  - 制約
  - Evidence
  - 重要な非ゴール
- v1 の `belay context` を、Goal / Evidence を含むグラフ全体を対象とした
  compiler へ発展させる
- 選択は BM25 とリンクグラフによる決定的なアルゴリズムを維持する。
  LLM による要約・圧縮が必要な場合はエージェント側に任せる
  (「設計原則との整合」参照)

**完了条件**: エージェントが作業開始時に `belay context compile` の出力を
標準で読み込む運用が定着し、文脈不足による手戻りが減ったと確認できる。

**見直し条件**: エージェント側の文脈取得能力の向上により選別の価値が
薄れる場合、compile 機能は検索 API の提供に縮小する。

### Phase 6: Delivery Assurance

Goal / Plan / Work / Evidence が存在しても、作業中に現在地を見失ったり、
人間の曖昧な発言を AI が黙って補完したり、実装済みを検証済みと
誤認したりする問題は残る。Phase 6 では既存のトレースを使い、
意図・計画・実装・証拠を継続的に照合する。

- **Intent Brief**: Tier 2 以上の初回に、人間の発言を Problem / Outcome /
  Success Signals / Constraints / Non-goals / Assumptions / Unknowns として
  エージェントが暫定構造化し、人間が訂正できる形で提示する
- **Delivery Map**: Goal の Success Criterion ごとに、成果物 Task、状態、
  検証方法、Evidence を対応付け、AI と人間の共通の現在地にする
- **Reconciliation loop**: 意味のある作業単位、仕様変更、handoff、完了前に
  Intent Brief / Goal / Map / diff / Evidence を再照合する
- **Completion assurance**: `implemented` と `verified` を分離し、fresh context
  の review と human acceptance までを完了判定に含める
- **段階導入**: 初期版は Agent skill と Plan 本文の規約だけで開始する。
  dogfooding で価値が確認できた検査だけを `plan lint`、`reconcile`、
  opt-in completion gate として本体へ移す

**完了条件**: Tier 2 以上の dogfooding で Intent Brief と Delivery Map が
標準利用され、現在地の確認、実装漏れの発見、仕様解釈の修正のいずれかに
実際に使われる。

**見直し条件**: Map が恒常的に stale になる場合は Success Criterion ごとの
未充足一覧へ縮小する。通常の Plan と Coverage だけで同等の判断ができる場合、
独立機能化しない。

詳細は [docs/design/phase6.md](design/phase6.md) を参照。

---

## 長期価値の根拠

AI は今後さらに進化し、人間の意図の汲み取り、仕様の自律的な策定、設計・実装・運用改善の
自律的な遂行も可能になっていく。それでも Belay の価値が残ると考える理由は、
AI が賢くなっても次の制約は残るからである。

- 推論コスト、レイテンシ、コンテキストウィンドウ
- ガバナンス、監査性
- 企業固有の価値観、リスク許容度
- 過去の判断履歴
- 人間自身の曖昧な意図

Belay は AI の能力不足を補うツールではなく、
**AI を効率的・安全・意図整合的に働かせるための基盤**である。

## 収益化の方向性(仮説)

収益化もフェーズと同様に検証すべき仮説である。
現時点で最も筋が良いと考える構造を記す。

### 基本構造: 個人は無料、組織は有料

- local-first CLI は無料・OSS のまま配布し、開発者からの
  bottom-up 採用を広げる。個人・単一リポジトリの利用には課金しない
- 価値が跳ね上がるのは組織レベルである。複数リポジトリ横断の
  Decision 検索、チーム共有の判断履歴、組織全体の Goal Coverage、
  権限管理は、ローカル SQLite では原理的に提供できない。
  ここが自然な課金境界になる
- 「未解決の課題」に挙げた複数人での同時運用・sync 競合は、
  解くこと自体が有料プロダクト(ホスト型 sync / チームサーバー)になる。
  技術的な宿題と収益化が同じ場所にある

### 本命: AI 開発の監査・ガバナンス層

- EU AI Act、SOC 2、ISO/IEC 42001(AI マネジメント)などの流れにより、
  「AI が書いたコードを、誰の意図に基づき、何を根拠に本番投入したか」を
  証明する要求は強まっていく
- Goal → Decision → Work → Evidence のモデルは、そのまま監査証跡になる。
  Phase 3 の append-only な Evidence とコミット SHA の紐づけが基盤
- 支払い動機の質が良い。生産性ツールは「あれば嬉しい」で予算が渋いが、
  コンプライアンスは「ないと困る」で予算が別枠かつ大きい
- 想定する売り物: audit-ready なレポート出力、改ざん不可の証跡保証、
  Human approval のサインオフワークフロー(責任分界の明確化)

### 補完: CI ゲートとしての integration

- verified Goal Coverage が閾値未満なら PR をブロックする、
  stale な Evidence を PR 上で警告する、といった GitHub App を
  Phase 3〜4 の自然な商品化として提供する
- CI に組み込まれる製品は解約されにくく、per-seat / per-repo
  課金が成立しやすい

### 課金しない領域

- **Goal lint / improve の意味的評価そのもの**: モデルの進化に
  コモディティ化される領域であり、本体に LLM を組み込まない方針
  (「設計原則との整合」参照)とも整合しない
- **個人開発者への課金**: トレースツールは単独利用では価値の実感が
  遅い。無料で配り、事実上の標準になることを狙う側である

### 競争リスクと対抗軸

- GitHub、エージェントベンダー、IDE ベンダーが「自社プラットフォームに
  監査ログあり」としてバンドルしてくる可能性がある
- 対抗軸は **ベンダー中立性**。Claude でも Codex でも他のエージェント
  でも、どのツールを使っても一つの証跡グラフに集約される中立な
  トレース層であること。`--install-skill` の複数エージェント対応は
  この布石であり、意識的に守る

### フェーズとの対応

- **Phase 0〜2**: 無料 CLI として採用を広げる段階。収益化はしない
- **Phase 3**: 収益化の転換点。Evidence layer の設計時から
  マルチユーザー・組織横断の集約を見据えておく
- **Phase 4〜5**: ホスト型ダッシュボード、監査レポート、
  CI ゲートを有料層として展開する

## Non-goals

- **テストランナーを作らない**: cargo test / pytest / CI を置き換えない。
  結果を Evidence として集約・関連付けするのが役割
- **既存の検証・観測ツールの置き換えに向かわない**: 差別化しづらく、
  scope creep の典型経路である
- **Issue tracker を置き換えない**: タスクの進行管理は既存ツールに任せる
  (「ポジショニング」参照)
- **Belay 本体に LLM 呼び出しを組み込まない**: 意味的な推論は
  エージェント側に任せ、本体は決定的な構造化・検査・集計に徹する
  (「設計原則との整合」参照)

## Risks / Blind Spots

- **Scope creep**: Goal、Evidence、CI、Evals、Observability まで抱えると巨大化する
- **Test runner 化の罠**: 既存ツールの置き換えに向かうと差別化しづらい
- **Goal scoring の疑似精度**: スコアが高くても本当に正しい Goal とは限らない
- **Evidence rot**: Evidence はコードの変更で古くなる。鮮度管理
  (Phase 3)を欠くと、カバレッジや検証結果が実態と乖離した
  誤った安心感を与える
- **カバレッジのゲーム化**: リンク存在ベースの指標は形式的に満たせる。
  verified coverage との区別(Phase 4)を欠くと指標が形骸化する
- **AI 依存の循環**: AI が Goal をレビューし、AI が実装し、AI がテストする場合、
  独立した検証軸が必要
- **Human bottleneck**: 最終的には人間自身の意図が曖昧なことが問題になる可能性がある
- **Enterprise adoption**: 企業利用では監査、権限、機密情報、法務、責任分界が重要になる

## 未解決の課題

- **対象ユーザーとフェーズの対応**: 各フェーズがソロ開発者・チーム・
  エンタープライズのどこを狙うのかは未定義である。現行の設計
  (local-first、単一リポジトリ、Git 経由の共有)はソロ〜小規模チームに
  最適化されている。「収益化の方向性」ではエンタープライズ
  (監査、権限、責任分界)を収益の本命仮説と位置づけたが、
  複数人での同時運用・sync 競合・組織横断集約をどのフェーズで
  どう設計するかは未定である。Phase 3 の Evidence layer 設計時までに
  具体化する必要がある。

## まとめ

Belay は記録ツール(Traceability)から始め、長期的には次へ進化する。

1. Traceability tool(現在)
2. Goal Engineering tool
3. Evidence management layer
4. Context compiler
5. Human-AI interface
6. AI work orchestration 基盤

その中心にある思想は一貫している。

**Belay は、人間の意図と AI の実行を繋ぎとめるためのツールである。**
