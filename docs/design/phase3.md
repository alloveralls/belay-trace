# Phase 3 開発計画: Evidence / Verification layer

[ロードマップ](../roadmap.md) Phase 3 の実装計画。
既存の検証基盤の結果を Evidence として集約し、
Goal / Decision / Work へ関連付ける。テストランナーは作らない。

## 目的

「テストが通った」を「どの Goal / Decision がどの証拠で
支えられているか」に変換する。Evidence は append-only・
コミット SHA 紐付き・鮮度付きの第一級データとして扱う。

## ロードマップとの対応

- **完了条件**: dogfooding で CI の結果が Evidence として取り込まれ、
  `belay doctor` が stale Evidence を検出できる状態で
  1 ヶ月以上運用される
- **見直し条件**: 登録・維持コストが確信を上回る場合、
  取り込み対象を CI 結果と human approval のみに絞る
- 収益化の転換点であるため、設計時からマルチユーザー・
  組織横断の集約を見据える(スキーマに組織側で必要になる
  発行者情報を最初から持たせる)

## 依存

- Phase 1(Goal、`fulfills` / `supports`)。
  ただし Evidence は既存の Work / Decision にも紐づけられるため、
  Phase 1 と並行開発が可能(リレーション追加のみ調整が必要)

## スコープ

### In

- Evidence データモデル(専用テーブル + append-only NDJSON mirror)
- `belay verify` コマンド群(record / import / status)
- 鮮度(freshness)判定と `belay doctor` の stale 検出
- LinkRelation `verifies` の追加
- GitHub Actions からの取り込み例(ワークフロー雛形)

### Out

- テスト実行そのもの(Non-goal)
- Coverage 集計は Phase 4
- ホスト型の組織横断集約(収益化フェーズで別途設計)

## 設計

### 1. Evidence は entry ではなく専用モデルにする

**決定**: Evidence を `EntryType` に追加せず、専用テーブル +
専用 mirror で扱う。

理由:

- Entry は revision を持ち編集可能だが、Evidence は不変
  (append-only)。更新は新しい Evidence の追加で表す
- Evidence は高頻度・高volume(CI 実行ごと)であり、
  1 件 1 Markdown ファイルの mirror は Git の diff と
  レビューを汚染する
- 一方で「Git で追跡でき、SQLite を失っても再構築できる」という
  v1 の回復性原則は維持したい

mirror 形式: `.belay/evidence/YYYY-MM.ndjson`(月次ファイル、
追記のみ、1 行 1 Evidence、決定的シリアライズ)。
`belay rebuild` は NDJSON からテーブルを再構築する。
`belay sync` の対象外とし、正本は常に NDJSON 側とする
(SQLite はインデックス)。

### 2. スキーマ

```sql
CREATE TABLE evidence (
    id INTEGER PRIMARY KEY,
    display_id TEXT NOT NULL,        -- EVD-<timestamp>-<seq>
    kind TEXT NOT NULL,              -- 下表
    verdict TEXT NOT NULL,           -- pass / fail / warn / info
    commit_sha TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    source TEXT NOT NULL,            -- 例: cargo-test, gh-actions
    issuer TEXT NOT NULL,            -- 例: ci:github, human:<name>
    summary TEXT NOT NULL,
    detail_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE evidence_links (
    evidence_id INTEGER NOT NULL REFERENCES evidence(id),
    entry_id INTEGER NOT NULL REFERENCES entries(id),
    relation TEXT NOT NULL,          -- verifies / refutes
    PRIMARY KEY (evidence_id, entry_id, relation)
);
```

- `commit_sha` と `captured_at` は必須。鮮度判定の基盤
- `issuer` は責任分界(human approval / CI / エージェント)を
  最初から区別する。収益化フェーズの監査要件の布石
- kind の初期セット: `test`, `ci-run`, `lint`, `type-check`,
  `bench`, `security-scan`, `human-approval`, `llm-eval`,
  `metric`。追加は TEXT のため migration 不要
- `LATEST_SCHEMA_VERSION` を上げ、テーブル追加を migration で行う

### 3. `belay verify` コマンド群

```sh
# 手動・スクリプトからの登録
belay verify record \
  --kind test --verdict pass \
  --source "cargo test" \
  --summary "unit + integration 212 passed" \
  --verifies WRK-...-implement-sync \
  --verifies GOAL-...-reliable-sync

# 結果ファイルの取り込み(初期は JUnit XML のみ)
belay verify import --junit target/nextest/junit.xml \
  --verifies WRK-...-implement-sync

# エントリ側から見た証拠の一覧と鮮度
belay verify status GOAL-...-reliable-sync
```

`verify status` の出力例:

```text
GOAL-...-reliable-sync (active)

  pass  test        cargo test      2026-07-01  HEAD    fresh
  pass  ci-run      gh-actions      2026-06-28  ~3 ago  stale
  info  human-approval  konoike     2026-06-20  ~9 ago  stale

fresh: 1 / 3
```

- `commit_sha` は未指定なら現在の HEAD を自動取得する
- human approval は `--kind human-approval` の record として
  統一的に扱う(専用コマンドは作らない)

### 4. 鮮度(freshness)判定

- 判定は「Evidence の commit_sha が現在の HEAD の祖先で、
  その間に対象範囲の変更があるか」ではなく、まず単純に
  **HEAD との一致 / 不一致 + 経過コミット数** で行う
  (git履歴の解析は `git merge-base` / `rev-list --count` を
  subprocess で使い、git がない環境では時刻ベースへフォールバック)
- 時刻ベースの基準は Evidence の `captured_at` と評価時の UTC 現在時刻とする。
  HEAD を取得できない、`commit_sha` が `unknown`、または git を実行できない場合に
  のみ使用し、`stale-after-days` 以内(境界を含む)を fresh とする。git が実行できても
  commit を比較できない場合はフォールバックせず stale とする
- `config.toml` でポリシーを設定できる

```toml
[verify]
stale-after-commits = 30
stale-after-days = 14
```

- `belay doctor` は「stale な Evidence しか持たない
  Active Goal / accepted Decision」を警告として報告する

### 5. CI 取り込み

- `docs/examples/github-actions-verify.yml` として雛形を提供:
  テスト実行 → `belay verify record` → NDJSON をコミット、
  もしくは artifact として保存し人間がローカルで取り込む
- どちらを推奨とするかは dogfooding(M5)で決める。
  NDJSON の CI からの自動コミットは競合が起きにくい
  (月次ファイルへの追記)が、追記順の決定性を保つため
  取り込み時に captured_at + display_id でソートし直す

## 実装ステップ

1. **M1: データモデルと migration**
   - evidence / evidence_links テーブル、display_id 採番
   - NDJSON mirror の書き出し・読み込み・決定的シリアライズ
2. **M2: `verify record` と rebuild 統合**
   - record → NDJSON 追記 + テーブル反映
   - `belay rebuild` が NDJSON から復元できること
3. **M3: 鮮度判定と `verify status` / doctor**
   - git 連携、フォールバック、stale 警告
4. **M4: `verify import --junit`**
   - JUnit XML パース(1 ファイル = 1 Evidence に集約し、
     失敗テスト名を detail_json に格納)
5. **M5: CI 雛形と dogfooding**
   - belay-trace 自身の GitHub Actions に組み込み、
     1 ヶ月運用して完了条件を判定する
6. **M6: export / context への Evidence 露出**
   - `belay export json --evidence`、context 生成での
     「この Goal を支える最新 Evidence」の含有

## テスト計画

- NDJSON round-trip(書き出し → rebuild → 同一状態)の
  プロパティ的テスト
- 鮮度判定: git リポジトリを組み立てる統合テスト
  (HEAD 一致 / 乖離 / git なし環境)
- JUnit XML の実サンプル(nextest, pytest 出力)でのパース
- 既存機能の回帰(evidence 導入が entry 系に影響しないこと)

## リスク

- **Evidence rot の逆問題(登録コスト)**: 手動 record が面倒だと
  使われない。CI 取り込みを最優先で dogfooding し、
  見直し条件(CI + human approval に縮小)を早期に判定する
- **NDJSON の Git 競合**: 月次追記ファイルはブランチ間で
  競合しうる。union merge(`.gitattributes` で `merge=union`)を
  推奨設定として文書化する
- **git 依存**: 鮮度判定が git 前提になる。フォールバックを
  必ず実装し、doctor で「git 不在のため時刻ベース」と明示する

## オープンクエスチョン

- `refutes`(Evidence が Decision を反証する)を初期セットに
  含めるか。M1 ではスキーマ上許可し、CLI からは `verifies` のみ
  受け付ける
- detail_json の上限サイズ。大きな出力はパスで参照する方式を
  M4 で検討する
