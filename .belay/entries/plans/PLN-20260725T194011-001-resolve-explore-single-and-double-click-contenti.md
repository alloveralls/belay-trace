---
schema_version: 1
id: PLN-20260725T194011-001-resolve-explore-single-and-double-click-contenti
type: plan
title: Resolve Explore single and double click contention
status: approved
created_at: 2026-07-25T19:40:11+09:00
updated_at: 2026-07-25T20:27:15+09:00
revision: 5
tags: []
links:
- relation: references
  id: WRK-20260723T192431-001-refine-browse-readability-and-trace-navigation
- relation: follows-up
  id: PLN-20260722T225244-001-add-a-local-trace-provenance-browser
- relation: fulfills
  id: GOAL-20260725T194008-001-make-explore-node-activation-reliable-and-unambi
metadata: {}
---

## Intent Brief

### Problem

- GitHub Issue #15では、Explore graphのノードをダブルクリックして関連ページを開こうとしても、最初のsingle `tap`が近傍展開を開始し、詳細ページを開けないことがあると報告されている。
- 現行実装は同じnodeに即時の`tap` handlerと`dbltap` handlerを登録しており、両操作を排他的に調停していない。
- 現行Playwright E2EはExplore APIの段階展開を直接検証しているが、実際のpointer interactionによるsingle/double click競合を検証していない。

### Desired Outcome

- ダブルクリックは詳細ページへの移動、シングルクリックは近傍展開として、利用者が選んだ操作だけが実行される。
- 修正はExplore frontendの小さなinteraction調停に限定し、Browseのread-only性、provenance semantics、API、accessible fallbackを維持する。
- 回帰を実際のbrowser interaction testで検出できる。

### Success Signals

- 関連ページを持つnodeへの反復ダブルクリックが、近傍を展開せず毎回そのページへ移動する。
- シングルクリックは短い判定窓の後に一度だけ近傍を展開し、ページ移動しない。
- Browseの既存E2E、Rust test、lint、build相当の検証が通る。
- 検証結果がGoal Success Criteriaに対応するEvidenceとして残る。

### Constraints

- GOALのread-only、internal API、vendored asset、CSP、offline runtime制約を守る。
- Issue #15の修正に不要なgraph layout、data model、server routeを変更しない。
- 操作判定はcancel可能で、nodeごとの重複timer、stale callback、二重expandを防ぐ。
- 実装開始、PR作成、mergeはそれぞれ明示的な人間承認を必要とする。

### Non-goals

- Explore UX全体の再設計、single-clickとdouble-clickの役割変更。
- touch専用gestureまたは新しいUI controlの追加。
- Cytoscape.jsの更新。
- Issue #15のcloseまたはPR作成。

### Assumptions

- これは仮説である: 即時`tap`が開始するfetch、node追加、layout更新が`dbltap`認識または遷移を不安定にしている主因である。
- 既存の「single clickで展開、double clickで詳細を開く」という公開済みinteraction contractは維持する。
- 短いcancel可能なsingle-click判定窓で、APIやserver側を変更せず競合を解消できる。

### Unknowns / Decisions Needed

- Unknown: 問題の再現率と、mouse・trackpad・browser間の差。
- Unknown: 応答性とdouble-click認識を両立する最小の判定窓。
- 実装時にCytoscape event sequenceとbrowser testを用いて判定窓を決め、magic numberには意図を説明する。
- 現時点で実装開始を承認する人間判断が必要である。

## Scope and Approach

1. `tap`時はnode IDとcancel可能なtimerを保持し、判定窓が満了した場合だけ`expand`する。
2. 同じnodeの`dbltap`時は保留中のsingle-click処理をcancelしてから、`href`が存在する場合だけ詳細ページへ移動する。
3. graph更新後に古いcallbackが走らないよう、timerのcleanupと重複操作の扱いを明示する。
4. Playwrightから実nodeをsingle/double clickし、URL遷移とgraph状態を観測して両経路を検証する。
5. focused fresh-context reviewでtimer lifecycle、競合、accessibility regression、test flakinessを確認する。

## Delivery Map

| ID | Goal Criteria | Outcome / Task | State | Verification |
| --- | --- | --- | --- | --- |
| T-001 | SC-001, SC-002 | Explore nodeのsingle/double clickを排他的に調停し、保留timerとstale callbackを安全にcleanupする | not-started | `src/browse.js`のfocused reviewとinteraction test結果 |
| T-002 | SC-001, SC-002 | 実nodeへの反復double clickが詳細へ移動して展開せず、single clickが一度だけ展開するPlaywright回帰testを追加する | not-started | 新規Playwright testを単独・suite内で実行しpassをEvidence化 |
| T-003 | SC-003 | Accessible Goal listとkeyboard/canvas-independent導線が変更されず利用可能であることを確認する | not-started | 既存accessibility導線のPlaywright assertionと変更差分review |
| T-004 | SC-004 | fmt、clippy、全target test、Playwright E2E、rebuild、doctorを実行する | not-started | 各commandのpassing outputをGoalに紐づくEvidenceとして記録 |
| T-005 | SC-001, SC-002, SC-003, SC-004 | fresh contextで実装diff、Goal、Plan、Evidenceを照合し、findingを解消または明示する | not-started | Review entryをWorkに`reviews`でlinkし、全SC coverageを確認 |
| T-006 | SC-001, SC-002, SC-003, SC-004 | 人間が修正後の操作結果を受入れ、Goal達成可否を判断する | not-started | human-acceptance Evidence |

## Acceptance and Evidence

- `implemented`はcode/test定義が存在する状態、`verified`は該当結果を確認したEvidenceが存在する状態として分離する。
- T-002は単にtimeout完了を待つのではなく、URL、node/edge数、またはAPI request回数など観測可能な結果を検証する。
- 全Success Criterionを検証するEvidenceとhuman acceptanceが揃うまでGoalをcompletedとして扱わない。

## References

- GitHub Issue #15: https://github.com/alloveralls/belay-trace/issues/15
- Existing Browse Plan: PLN-20260722T225244-001-add-a-local-trace-provenance-browser
- Existing Browse Work: WRK-20260723T192431-001-refine-browse-readability-and-trace-navigation
