---
schema_version: 1
id: GOAL-20260726T014020-001-enable-responsible-traceable-evolutionary-develo
type: goal
title: Validate a structured Belay Route protocol for decision preparation
status: draft
created_at: 2026-07-26T01:40:20+09:00
updated_at: 2026-07-26T10:46:09+09:00
revision: 4
tags: []
links:
- relation: references
  id: GOAL-20260712T163956-001-maintain-intent-to-evidence-alignment-during-age
- relation: references
  id: GOAL-20260726T011059-001-make-belay-agent-usage-self-guiding-and-reliable
metadata: {}
---

## Summary

- Belayに蓄積された開発記録を、出典と不確実性を保った固定フォーマットの判断材料へ再構成するRoute protocolが、独立した価値を持つか反証可能な形で検証する。
- 初期Goalは、良い候補を大量に生成することではなく、既知の欠落、矛盾、未検証、未承認範囲を人間の判断前に表面化し、具体的な採用、修正、却下、保留と、その後のBelay記録を追跡できることを対象とする。
- 実開発での効果、複数組織への一般化、製品化、Browse UXは後続Goalへ分離する。

## Background and Evidence Basis

### Confirmed context and prior review

- Belayは、プロダクト開発に必要なGoal、Plan、Decision、Work、Review、Evidenceとそれらの関係を、AIと人間が参照できる永続的かつ決定的な記録として保持する。
- Belay coreはLLMを呼ばず、意味的な推論は外部のAIエージェントまたは製品レイヤーが担う方針である。
- Phase 6 Delivery Assuranceは、承認された変更のIntent、Delivery Map、実装、検証の整合を扱う。Routeはその前段で、次に何を変化または検証対象として選ぶかを扱う。
- REV-20260726T101157-001は、旧GoalがVision、初期能力、有効性評価、一般化を一つに含み、SC-005とSC-006が反証可能でないと指摘した。本改訂は初期protocol検証へ範囲を縮約する。
- GOAL-20260726T103644-001はRoute出力のBrowse UXを別Goalとして保持し、本Goalの固定出力契約に依存する。

### Human observations

- 人間がBelayへ記録していても、関連記録を読み込み、全体を把握し、次の判断へつなげるには大きな労力がかかる。
- AIに文書の読解を委任した人が、文書内の重要事項を把握しないまま、AIの説明を自分の理解として扱う事例が観察されている。
- これらは構想者による実体験と伝聞を含む弱い初期証拠であり、発生率、一般性、主要因はUnknownである。本Goalはこの観測自体を事実として一般化せず、固定シナリオでprotocol能力を先に検証する。

## Problem Hypothesis

**This is a hypothesis.**

- AIによる情報処理、候補生成、実装の速度が、人間による理解、比較、検証、意思決定、全体への再統合の速度を上回ることで、認知的な処理待ちが蓄積する。
- Belayは開発の記憶が失われる問題を軽減するが、記録を現在の判断に必要な形へ再構成する労力までは解消しない。
- Belayの型付きtraceを固定されたRoute inputへ変換し、AIの意味的推論を型付きの暫定Route outputへ制約すれば、自由対話だけの場合より、重要な欠落、矛盾、未検証、未承認範囲を一貫して人間の判断面へ出せる。
- accountabilityの帰属と、人間が判断へどの程度認知的に関与したかは別である。本Goalは前者をHuman Gateとして固定し、後者を改善できるかを仮説として扱う。

## Why This Goal

- Routeの長期Visionを実装する前に、固定フォーマット、provenance、承認境界、再統合という最小protocolが、良いAGENTS.md、一般的なSkill、強いモデルだけでは得られない再現性を持つか確認する必要がある。
- protocolの価値を固定シナリオで検出できなければ、Routeを独立機能として製品化する根拠はない。

## Success Criteria

- [SC-001] versioned Route Input、Assessment、Proposal、Human Response、Materialization Preview、Reconciliation Resultの最小契約が定義され、Fact、Human Observation、Assumption、Hypothesis、Unknown、Conflict、ProposalとBelay参照を型として区別し、決定的にvalidationできる。
- [SC-002] 固定評価シナリオに含まれる既知のblocking Constraint衝突、矛盾するDecision、未検証項目、根拠のないAssumption、未承認範囲を、Route outputが出典つきの判断対象またはstopとして表現できる。候補数、文章量、表示項目はPlanの評価変数とし、本Criterionでは固定しない。
- [SC-003] Human Responseは採用、修正、却下、保留と承認範囲を表現し、未承認のProposalからBelayのGoal、Plan、Decision、Work、Issue、Pull Request、実装開始を正式確定できない。人間が採用した内容だけが出典を保って記録される。
- [SC-004] Reconciliation Resultは、BelayのID、link、status、Evidence coverage、accepted scope、deferred scopeを決定的に照合し、意味的な再評価と区別して未解決事項を残せる。
- [SC-005] 既知解を持つ固定評価シナリオで、人間とAIの組が、提示内容の復唱では得られないblocking帰結を特定し、修正、却下または保留としてHuman Responseへ残せる。評価記録は理解済みとは表現せず、提示された事項と具体的応答だけを記録する。
- [SC-006] 事前登録した最低12回のfresh-context評価で、Route protocolはseedしたblocking条件の90%以上を人間の選択前に表面化し、通常のAIとBelayだけを使うbaselineを検出率で20 percentage points以上上回る。primary metricはblocking条件検出率とし、未承認内容の正式記録は0件である。
- [SC-007] 最低2名の独立reviewerが、Route outputとBelay記録だけから、何が承認され、何が未承認、却下、保留、未検証のままかを一致して再構築できる。複数モデル、組織、実開発への一般化は後続Goalで検証する。

## Constraints

- Belayを事実、判断、履歴、EvidenceのSource of Truthとして維持し、Routeを競合するSource of Truthにしない。
- Belay coreはlocal-firstかつdeterministicなままとし、LLM呼び出し、API key管理、非決定的な意味判断を組み込まない。
- Route reasoningは非決定的でよいが、入力、出力、schema validation、Belay参照、承認範囲、記録結果、provenanceは固定フォーマットで追跡する。
- 本Goalの検証期間中は、Proposal採用、Plan承認、実装開始、Issue作成、Pull Request作成、mergeの既存Human Gateを維持する。将来のモデル能力に応じた権限変更は本Goalの範囲外とする。
- accountabilityは人間に帰属する。認知的関与がRouteにより改善するかは仮説であり、人間の内面的理解をEvidenceで証明したと主張しない。
- Routeは矛盾、Evidence不足、重要なUnknownがある場合にstopまたは追加検証を有効な出力として表現する。
- 評価fixtureと採点基準は実行前に固定し、最低2名の独立reviewerを含める。
- Phase 6 Delivery AssuranceおよびGOAL-20260726T011059-001のBelay agent guidance改善とは責務を分離する。

## Non-goals

- 実開発でRouteが時間、手戻り、読解量を改善したと主張すること。
- 複数モデル、組織、法人運用、課金、ホスティング、RBACへ一般化すること。
- BrowseでRouteを表示または操作すること。GOAL-20260726T103644-001で扱う。
- Product Map、Requirement、Route Proposalをfirst-class persistence modelとして追加すること。
- Belay coreへLLMを組み込むこと。
- 人間の内面的理解を証明すること、またはAIと人間のどちらが正しいかを一般化すること。
- 将来のモデル能力に応じた自動判断とHuman Gateの境界を決定すること。

## Verification

- 既知のblocking条件と正解ラベルを持つ固定fixtureを作成し、Route protocolありと、通常のAIとBelayだけのbaselineを同じBelay stateからfresh contextで実行する。
- 少なくとも12回の評価を事前登録し、blocking条件検出率、未承認内容の正式記録件数、出典欠落、Human Responseの具体性を採点する。
- 最低2名の独立reviewerがblind scoringし、相違を記録して合意結果をEvidenceへ残す。
- schema validation、参照整合、承認境界、Reconciliation Resultは決定的テストで検証する。
- 本GoalとPlanの承認、評価fixtureの固定、実装開始はそれぞれ別Human Gateとする。

## Completion, Review, and Withdrawal

- 完了条件: SC-001からSC-007にpassing Evidenceがあり、独立reviewでblocking findingがなく、人間が初期protocol検証の結果を受け入れる。
- 見直し条件: Route outputが判断材料を増やすだけでblocking条件検出率を改善しない、またはreviewer間で承認状態を一致して再構築できない場合、schemaとprotocolを縮小して再評価する。
- 撤退条件: 良いAGENTS.md、一般的なSkill、強いモデルとBelayの組み合わせが、固定評価でRoute protocolと同等の検出率、provenance、承認境界、再構築性を示す場合、Routeを独立機能化しない。

## Risks

- 固定フォーマットがstructured wrongnessを作り、誤った分析へ過度な権威を与える。
- 評価fixtureに過適合し、実開発で役立たないprotocolを合格させる。
- baseline条件が弱すぎる、またはRouteを知る評価者の学習効果で差が誇張される。
- schema項目を増やしすぎて、Goal自身が解こうとする認知負荷を再生産する。
- 暫定Proposalまたはdecision responseが、下流で承認済み事実や人間の理解証明として誤読される。

## Assumptions

- **This is a hypothesis.** Belayの型付きtraceとEvidenceは、一般的な会話履歴より、既知のblocking条件を出典つきで再構成する入力に適している。
- **This is a hypothesis.** 固定された入力、出力、承認境界は、モデルの自由文応答だけより重要事項の検出と再構築を一貫させる。
- **This is a hypothesis.** 判断面を圧縮して提示することで、人間の認知的関与を維持できるが、内面的理解を証明することはできない。

## Unknowns / Decisions Needed

- 初期protocolを文書と評価harnessだけで検証するか、独立moduleまたはSkillのreference implementationを含めるか。
- Route outputを評価中に保存しないか、local operational stateとして保存するか。
- Belayの既存context出力だけでRoute Inputを構成できるか、追加の決定的export contractが必要か。
- Human Responseの記録形式で、応答の事実と人間の理解を誤認させない表現をどう強制するか。
- fixtureの難易度、baseline prompt、blind scoringをどう固定し、評価者の学習効果を抑えるか。
