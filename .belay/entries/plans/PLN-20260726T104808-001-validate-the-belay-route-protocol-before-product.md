---
schema_version: 1
id: PLN-20260726T104808-001-validate-the-belay-route-protocol-before-product
type: plan
title: Validate the Belay Route protocol before productization
status: draft
created_at: 2026-07-26T10:48:08+09:00
updated_at: 2026-07-26T10:48:19+09:00
revision: 4
tags: []
links:
- relation: references
  id: GOAL-20260726T103644-001-make-belay-route-decision-state-legible-and-trus
- relation: references
  id: REV-20260726T101157-001-review-belay-route-goal-draft
- relation: fulfills
  id: GOAL-20260726T014020-001-enable-responsible-traceable-evolutionary-develo
metadata: {}
---

## Intent Brief

### Problem

- Belayは開発の事実、判断、履歴、Evidenceを保持できるが、人間がそれらを現在の判断へ再構成する認知負荷は残る。
- 強いAIと良いpromptだけでも類似の支援が可能なため、Routeが独立機能として必要かはUnknownである。
- 旧Route GoalはVision、初期能力、有効性評価、一般化を混在させ、REV-20260726T101157-001で検証不能と指摘された。
- 固定フォーマットは比較可能性とprovenanceを与える一方、誤った分析を整然と提示するstructured wrongnessを生む可能性がある。

### Desired Outcome

- Belayの型付きtraceをversioned Route Inputへ変換し、外部AIによる意味的推論を、決定的にvalidation可能なAssessment、Proposal、Human Response、Materialization Preview、Reconciliation Resultとして扱える最小protocolを定義する。
- 既知解を持つ固定シナリオで、通常のAIとBelayだけのbaselineよりblocking条件を一貫して表面化できるか検証し、継続、縮小、撤退を判断する。

### Success Signals

- Route protocolの各documentがschema validationを通り、Belay参照、分類、provenance、承認範囲を機械的に検査できる。
- seedしたblocking条件の90%以上を人間の選択前に表面化し、baselineを20 percentage points以上上回る。
- 未承認内容がBelayの正式記録または実装承認へ変換される事例が0件である。
- 2名以上の独立reviewerが、承認、未承認、却下、保留、未検証の状態を一致して再構築できる。
- Goalの撤退条件を適用できるだけのEvidenceが残る。

### Constraints

- Tier 3としてGoal承認、Plan承認、実装開始、評価fixture固定を別Human Gateにする。
- Belay coreと既存CLIはLLMを呼ばず、Route reasoningは外部AIが担当する。
- Route protocolはBelayの公開された出力を入力とし、SQLiteやmanaged Markdownの内部構造へ直接依存しない。
- 初期reference implementationはBelay本体から依存されないrepository-localな独立packageまたはharnessとし、production `belay route` commandを追加しない。
- Routeの推論結果は暫定情報であり、BelayのSource of Truthと競合させない。
- Browse対応はGOAL-20260726T103644-001の後続作業とし、本Planに含めない。
- 評価記録は人間の理解済みを主張せず、提示内容、具体的応答、採点結果だけを記録する。

### Non-goals

- 実開発で時間、読解量、手戻りが改善したと主張すること。
- Routeを独立サービスまたは法人向け製品として実装すること。
- 複数モデル、組織、権限、課金、ホスティングへ一般化すること。
- Product Map、Requirement、Route ProposalをBelayのfirst-class persistence modelへ追加すること。
- Browse UI、承認UI、自動Issue、自動PR、自動実装を追加すること。
- 将来の高性能モデルに合わせたHuman Gate緩和条件を決めること。

### Assumptions

- **This is a hypothesis.** versionedな入出力契約と承認境界は、通常の自由対話よりblocking条件の検出と判断履歴の再構築を一貫させる。
- **This is a hypothesis.** 初期能力は本番機能を作らず、固定fixture、外部AI、standalone validator、manual Human Responseで評価できる。
- **This is a hypothesis.** 最低12回の評価と2名の独立reviewerは、製品有効性ではなくprotocolの初期能力を反証するには十分である。
- **This is a hypothesis.** 既存の`belay context compile`またはexport出力から、内部storageへ依存せずRoute Inputを構成できる。

### Unknowns / Decisions Needed

- standalone packageの実装言語と配置。推奨はBelayに依存されない小さなRust packageまたは同等の独立harnessだが、contract設計後に最小実装を選ぶ。
- Route outputを評価中の一時データとするか、raw evaluation artifactとしてversion controlへ残すか。
- baseline promptをどこまで通常利用に近づけ、Route protocolの知識を除外するか。
- fixtureの難易度と、複数blocking条件を一つのcaseへ含めるか。
- 2名のreviewerのblind scoringを、構想者とfresh-context Claudeで十分とするか。
- 90%と20 percentage pointsという初期閾値が妥当か。Plan承認前に人間が修正可能であり、評価開始後は変更しない。

## Scope and Approach

### 1. Preserve the vision without mixing it into the experiment

- `docs/design/route.md`へ、追跡可能な進化的開発、BelayとRouteの境界、accountabilityと認知的関与、将来の権限変化、後続Goalを記録する。
- 初期protocol Goalの成否判定には、Visionの将来要件を混入させない。

### 2. Define stable protocol envelopes

- Route Input
- Route Assessment
- Route Proposal
- Human Response
- Materialization Preview
- Reconciliation Result

各documentはschema version、Belay reference、input fingerprint、Fact、Observation、Assumption、Hypothesis、Unknown、Conflict、Proposal、provenance、limitations、authority stateを必要な範囲で型付けする。候補数と文章量は固定せず、`stop`、`insufficient-context`、`no-safe-route`を正規の結果として扱う。

### 3. Keep inference outside Belay and validation deterministic

- 外部AIへRoute Inputとprotocol instructionを渡し、構造化outputを生成するreference flowを作る。
- standalone validatorはschema、参照形式、承認状態、未承認materialization、Reconciliation整合を決定的に検査する。
- Belay側のread contractが不足する場合、内部storageを直接読む回避策を取らず、追加interfaceをDecision候補として人間へ提示する。

### 4. Pre-register a fixed evaluation

- blocking Constraint conflict
- contradictory Decision
- implemented but unverified work
- unsupported Assumption
- unapproved or deferred scope

を含む既知解fixtureを作る。各caseは正解ラベル、重大度、期待されるstopまたはHuman Decision、許容される代替表現を持つ。

Route conditionとbaseline conditionへ同じBelay stateを渡し、実行前にprompt、run数、primary metric、採点rubric、reviewer、失敗条件を固定する。

### 5. Separate behavioral evidence from claims of understanding

- Human Responseには採用、修正、却下、保留、理由、承認範囲を記録する。
- Evidenceは`decision-response`または同等の限定された事実を記録し、`human understood`とは表現しない。
- reviewerは復唱ではなく、seedしたblocking帰結が人間の選択前に表面化し、具体的応答へ反映されたかを採点する。

### 6. Reconcile and decide whether Route should continue

- 評価結果をGoal Criteriaへ対応付け、accepted、unaccepted、deferred、unverifiedを再構築する。
- 成功時も直ちに製品化せず、実開発有効性を検証する後続Goalを提案する。
- baselineが同等なら独立機能化を撤退し、必要なcontractまたはSkill改善だけをBelay側へ還元する。

## Delivery Map

| ID | Goal item | Outcome / Task | Actor | State | Verification / Evidence |
| --- | --- | --- | --- | --- | --- |
| T-001 | SC-001, SC-007 | Route Vision、Phase 6との境界、accountabilityと認知的関与、protocol用語をdurable designへ分離する | AI + Human | not-started | fresh-context semantic review; Goalとdesignの重複確認 |
| T-002 | SC-001, SC-003, SC-004 | 6種類のversioned Route document contract、authority state、provenance、failure resultを定義する | AI | not-started | schema examplesとdeterministic validation tests |
| T-003 | SC-001, SC-003, SC-004 | Belayの公開read outputだけを使うstandalone validatorとreference flowを作り、未承認書き込みを拒否する | AI | not-started | unit/integration tests; Belay coreからRouteへの依存がないことのdiff review |
| T-004 | SC-002, SC-005, SC-006 | 5種類以上のseeded blocking条件、正解ラベル、許容表現を持つ固定fixture corpusを作成する | AI + Human | not-started | fixture lint; 人間によるfixture freeze approval Evidence |
| T-005 | SC-005, SC-006 | baseline prompt、Route prompt、12回以上のrun matrix、primary metric、blind scoring rubric、失敗条件を事前登録する | AI + Human | not-started | pre-registration review; 評価開始前のhuman approval Evidence |
| T-006 | SC-002, SC-003, SC-005, SC-006 | Route conditionとbaseline conditionをfresh contextで実行し、Human Responseを含むraw結果を採取する | AI + Human | not-started | schema-valid raw artifacts; 未承認正式記録0件 |
| T-007 | SC-006, SC-007 | 最低2名の独立reviewerがblind scoringと承認状態再構築を行い、相違を調停する | Human + independent AI | not-started | detection rate、baseline差、reviewer agreement、review entries |
| T-008 | SC-001..SC-007 | Goal、protocol、評価結果、Evidence、未解決事項をreconcileし、継続、縮小、撤退を決定する | AI + Human | not-started | coverage、fresh-context review、human acceptanceまたはwithdrawal Decision |

## Human Gates

- Gate 1: 本Planと改訂Goalの承認。承認しても実装は開始しない。
- Gate 2: 実装開始の明示承認。`jj new`、Work作成、source変更はこの後に行う。
- Gate 3: fixture、prompt、metric、閾値、run matrixのfreeze承認。承認後は結果を見て変更しない。
- Gate 4: 評価結果にもとづく継続、縮小、撤退の判断。

## Validation Strategy

- Goal lintとPlan構造のfocused review
- contract sampleのschema validation
- invalid classification、missing provenance、unknown reference、unsupported version、unapproved materialization、stale inputのnegative tests
- 同一fixtureからのRouteとbaselineのfresh-context run
- blind scoringとreviewer disagreementの保存
- Belay core、CLI、Browse、storage schemaが変更されていないことのdiff review
- `belay sync`、`belay doctor`、`belay coverage`、`jj st`、`jj diff`

## Risks and Responses

- fixture overfitting: hidden holdout caseを最低1件含め、protocol authorが採点しないrunを設ける。
- weak baseline: baselineにも同じBelay contextと一般的な高品質分析依頼を与え、Route固有schemaとrubricだけを除外する。
- schema bloat: required fieldを最小化し、追加情報をoptional blockへ分離する。
- ceremonial Human Response: 理解確認ではなく、seedしたblocking帰結への具体的な採用、修正、却下、保留を採点する。
- structured wrongness: Fact、Assumption、Hypothesis、Unknownを分離し、sourceなしFactをvalidatorで拒否する。
- accidental productization: production CLI、Browse、hosted service、automatic writeを本Planから除外する。

## Review Requirements

- protocol authorとは別のfresh-context reviewerがGoal、Plan、contract、fixture、評価結果をレビューする。
- fixtureとscoringのreviewerは、可能な範囲で期待されるRoute出力を見ずに正解ラベルとraw outputを採点する。
- security、privacy、外部送信が発生する実装へ変わる場合は、Planを更新し`requires_human_review: true`の専門reviewを追加する。

requires_human_review: true
