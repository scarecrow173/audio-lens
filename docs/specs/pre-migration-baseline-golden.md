---
id: "SPEC-034P3HtT4o8PMr9CfcRf9l"
type: "spec"
status: "proposed"
title: "pre-migration baseline: 現行 golden・実メディア・性能の固定"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/98"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "validation-strategy.md"
    - "observability-foundation.md"
    - "../adr/pre-migration-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/pre-migration-baseline-issue-98.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# pre-migration baseline: 現行 golden・実メディア・性能の固定

## What Should Be Built

Ruviform 移行(Issue #95)の before/after 比較基準として、現行
AudioLens の挙動・精度・メディア対応範囲・性能を、特定の
commit・環境・入力で固定した「pre-migration baseline」を
リポジトリに保存する。baseline は次の 4 面からなる:

1. **再現情報の一覧** — 比較に用いる commit SHA、toolchain
   (rustc/cargo/mise tools)、OS/CPU、入力サンプルの SHA-256、
   実行設定、再現手順。
2. **golden の保存** — 固定した実メディア入力に対する主要な
   精度/locate/解析結果・validity・provenance・時刻・生成物の
   観測値を、run 間で変動しない面に正規化して保存する。
3. **性能 baseline** — 同じ入力に対する stage 別 wall
   時間、run レベルの CPU・peak RSS・I/O 推定量、
   alloc、生成物サイズと e2e 時間を記録し、再計測可能な
   計測スクリプトを残す。測れない粒度(下記)は「実測
   未実施」と明記する。
4. **比較契約** — 数値出力の「bit-exact 維持が必要な契約」と
   「浮動小数誤差を許容できる契約」の区分を定義し、アプリ
   観測値の意味が変わらない条件を明文化する。

## Why Now

Issue #95 の採用済み方針 5 は「現行のアルゴリズム・実サンプル・
Golden・性能測定を比較基準として保全し、数値精度/処理時間/
メモリ/メディア対応範囲を評価した後に旧実装を削除する」を
要求する。移行の各段階(#100 メディア I/O、#101 DSP カーネル、
#103 Dataset/Provenance 回帰)で「何が変わったか」を現行結果
と客観比較するためには、比較対象である現行の観測値とその
取得手順が、誰でも再計測できる形で固定されている必要がある。
baseline がなければ移行差分の評価は印象に依存する。

## Users and Value

- 移行実装者: Ruviform 置換後に同じ入力・同じ手順で再計測し、
  数値・性能・coverage の差分を機械的に照合できる
- レビュアー/承認者: 移行差分が「契約上許容される差」と
  「意味が変わる差」のどちらかを、固定済みの区分で判定できる
- 後続 Issue(#100/#101/#103/#104): before/after の根拠
  データとして baseline を参照できる

## Scope

### In Scope

- 比較用 commit・toolchain・OS/CPU・入力サンプル hash・
  実行設定・再現手順の固定と記録
- 固定入力に対する golden の保存(locate 結果、cheap/
  reference/expensive 解析結果、validity・confidence・
  provenance、時刻観測、生成物索引)
- 数値契約の区分(bit-exact 必須 vs 浮動小数誤差許容)と
  「アプリ観測値の意味が変わらない条件」の定義
- 性能 baseline: stage 別 wall(manifest `duration_ms`)、
  run レベルの e2e wall・CPU 時間・peak RSS(標本値)・
  I/O(実測または推定量)、alloc 計測、生成物サイズ集計、
  再計測スクリプト。stage 別の busy/CPU/RSS/IO は外部
  ポーリングでは stage 境界に帰属できないため実測対象外
  (実測未実施として記録する)
- 固定 run matrix(ADR 参照): `canonical` + 5 バリアント +
  `small` + `resume` + `aa`。各入力・stage・input-ids を
  `baseline/config.json` に記録する
- Issue #98 列挙項目 → 担保手段(e2e run / 既存 test /
  追加 test / 実測未実施)のカバレッジ台帳
  (`baseline/coverage.json`)の整備
- 既存テスト・ベンチの棚卸しと、欠けている守るべきケースの
  最小増補(対象: 下記対象列挙に対応する未カバー領域)
- baseline の保存場所・正規化ルール・比較手順の文書化

対象範囲(Issue #98 の列挙に対応):

- Decode: WAV の元 PCM 値/format、AVI/MP4 と AAC fallback、
  PTS/timebase、VFR、multitrack、欠落/不連続の素材
- `al-math`: f64 FFT/Real FFT、STFT、NCC/GCC-PHAT、
  bandpass/resample、linalg、NaN/Inf・無音・ゼロ長・短長入力
- `al-locate`: 全手法・合議/ambiguous argmax・anchor/loop/
  drift/gap/skip、band_ncc、window_anchors_in、残差
- `al-analysis`: Cheap/Reference/Expensive、band_gain、
  CSD/coherence、feature validity・support・confidence、
  複素中間スペクトル
- `al-job`/`al-dataset`: 結果生成、再開、provenance、
  tensor、生成済み出力

### Out of Scope

- `al-viz` の HTML・Visualization Dataset の維持 golden 化
  (#105/#106 で廃止。移行前比較材料として既存出力の存在・
  サイズ・hash の記録のみ行う)
- 解析アルゴリズム自体の変更(計測・固定・必要最小限のテスト
  増補が本件の範囲)
- 既存データの再読込互換(Pre-release 方針により要求しない。
  差異はデータ再生成で対処)
- ruviform リポジトリ側の変更
- CI 基盤への組み込み(CI 未整備。後続課題)

## Requirements

- 保存する baseline は値を捏造しない。実測できない項目は
  「実測未実施」と明記して区別する
- golden は run 間で変動する値(乱数 ID、絶対パス、計時値)
  を正規化した面で記録し、正規化ルールを文書化する
- 入力は再現可能であること: LFS 管理の実サンプルは SHA-256
  で固定し、派生入力(ffmpeg 変換 AVI)は生成手順とハッシュを
  記録する
- 計測スクリプトは同一手順で再計測でき、出力形式が移行後の
  再計測結果と直接比較可能であること
- 生成するフィクスチャは非メディア形式(JSON 等)を優先する
- テスト増補は「既存テスト・ベンチで守られていない守るべき
  ケース」に限定する

## Acceptance Criteria

- [ ] `baseline/environment.json`・`inputs.json`・
  `config.json` が保存され、再現可能な実サンプル・
  派生入力の生成手順・SHA-256 ハッシュ・環境・設定・
  再現手順(`baseline/README.md`)の一覧がある
- [ ] 固定 run matrix 各実行の golden
  (`runs/<name>/golden/`・`artifacts.index.json`)が
  保存され、正規化ルールが ADR に定義されている。
  「主要な」範囲は `coverage.json` の台帳で定義される
- [ ] `coverage.json` が Issue #98 の列挙項目すべてを
  e2e run / 既存 test / 追加 test / 実測未実施のいずれかに
  割り当てている(台帳が列挙カバレッジの証明)
- [ ] 性能 baseline(`performance.json`・`bench/results.json`)
  と計測スクリプトがあり、同じ手順で再計測できることが
  確認されている
- [ ] bit-exact 維持契約と浮動小数誤差許容契約の区分と
  comparator 意味論が `contracts.json` + ADR に明文化
  されている
- [ ] `resume` run の normalized golden が `canonical` と
  一致する(resume 等価)
- [ ] A/A(同一入力 2 回実行)の比較結果が記録され、
  bit-exact 契約面が一致するか、非決定面が観測・記録
  されている
- [ ] 比較スクリプトの摂動自己検証(意図的な差分が報告
  されること)が確認されている
- [ ] 既存データの再読込互換を要求しないこと、差異はデータ
  再生成で対処することが明記されている
- [ ] 実測未実施の項目(stage 別 busy/CPU/RSS/IO 等)が
  明記されている

## Deferred Design Concerns

- 大きな実サンプル e2e の CI 常時回帰 | CI 未整備・実行時間が
  長い(約6分) | CI 整備時 | baseline の陳腐化を検知できない

## Sources and Relations

- `docs/discovery/pre-migration-baseline-issue-98.md`:
  既存カバレッジ・非決定要因・計測経路の調査結果
- `docs/specs/validation-strategy.md`: golden fixture の全体
  方針(本 spec は実メディア baseline の固定を追加する)
- `docs/adr/pre-migration-baseline.md`: 保存形式・正規化・
  比較契約・計測方法の決定
