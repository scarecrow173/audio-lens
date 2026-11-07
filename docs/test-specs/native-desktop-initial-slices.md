---
id: "TSPEC-0i3Bc6BhkywzIBeHpby4xP"
type: "test-spec"
status: "approved"
title: "Native desktop 初期 slice AL-G1〜G3 検証仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/native-desktop-initial-slices.md"
    - "../designs/native-desktop-initial-slices.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-desktop-initial-slices.md"
    - "../plans/native-desktop-initial-slices.md"
    - "../tasks/native-desktop-initial-slices.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/native-desktop-initial-slices.md"
    - "../designs/native-desktop-initial-slices.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native desktop 初期 slice AL-G1〜G3 検証仕様

## 構造(既定の Feature/Rule/Example 表記を使用)

```yaml
features:
  desktop-session:
    rules:
      - name: tick-drains-jobs
        examples:
          - fake driver で job event をキューし Tick inject → post_event が live window へ届く
          - RoutedEvent{window:None} は全 live window へ配送される
      - name: action-dispatches-intent
        examples:
          - Action(Intent) inject → dispatch_window_intent 経路で response event が owner window へ届く
      - name: close-lifecycle
        examples:
          - veto なし CloseRequested → Approve で respond_close
          - set_close_veto(true) した window の CloseRequested → Veto
          - veto なし CloseRequested + respond_close Approve 後の Destroyed → live から除去
          - 最終 live window かつ job 実行中の CloseRequested → Defer: window は live のまま close-pending、close_deferred が true、job/outcome は進行中(不可視待機ではなく window・取消導線・pending 状態を残す)
          - 実行中の再 close も同じ判定で Defer に留まる(冪等、spam 突破不可)
          - job 終端 drain で保留中 window へ request_close が再発行され、通常 flow 再評価 → Approve → Destroyed → exit(再評価が実際に veto/policy を通ることを検証)
      - name: destroy-policy
        examples:
          - exit_on_last_window=true かつ全 window Destroyed → driver.exit() が呼ばれる
          - live window が残っている限り exit しない
          - cancel_jobs_on_destroy=false(default) → cancel_window_jobs が呼ばれない
      - name: exit-defers-to-job-terminal
        examples:
          - 実行中 job ありで最終 Destroyed(OS 強制経路: Defer 不能) → exited=false のまま、job 完走 drain 後に session が exit() を発行し loop が自然終了(worker が "finished" を記録してから exit — process 終了での黙示 kill を検証)
          - cancel_jobs_on_destroy=true で Destroyed → cancel signal 後、cancelled 終端 drain まで exit() が遅延し loop が自然終了(destroy pump 直後に exited=false を assert する ordering 検証)
          - 正常 close + cancel_jobs_on_destroy=true → Defer 時点で cancel signal、window は live のまま cancelled 終端 drain で close 再開 → destroyed + exit
          - request_exit は exit_on_last_window=false でも全 live window へ request_close(veto-able)を発行し exit_pending を立てる — job 実行中は最終 window が Defer で残り、終端 drain で close 再開後に exit()(destroy_window 直呼びは veto/job 保留を越えるため不可)
          - request_exit は製品 veto を越えない: veto window は残り exited=false、veto 解除後の通常 close で完結
          - exit_on_last_window=false では最終 Destroyed 後も exit しない(終了要求が立たない)
  theme:
    rules:
      - name: resolves-light-and-dark
        examples:
          - resolve("light") / resolve("dark") が共に Ok
          - mode により fg/bg color が異なる
      - name: product-layer-overrides
        examples:
          - product 層 token が base primitive を alias 参照し解決される
      - name: invalid-mode-errors
        examples:
          - resolve("auto") が Err
  preferences:
    rules:
      - name: serde-round-trip
        examples:
          - 非 default 値を持つ GuiPreferences を to_string/from_str して同値
      - name: corrupted-file-falls-back
        examples:
          - 不正 JSON ファイルの load_or_default が default を返す
      - name: resolved-mode
        examples:
          - System + system=Some("light") → light; System + 無効値 → FALLBACK("dark")
          - Light → light、Dark → dark
      - name: window-spec
        examples:
          - WindowPrefs{width=1280,height=800,resizable=true} → WindowSpec へ写像される
  view-models:
    rules:
      - name: event-row-maps-typed-enums
        examples:
          - events 行 JSON → EventRow{confidence=Confidence::High, validity=Validity::Valid, start_s/end_s は f64}
          - candidates 行(参照先パスの artifact 由来)も同 mapping
      - name: alignment-row-preserves-ranges
        examples:
          - locate summary 行 → AlignmentRow{capture_start/capture_end} が typed SampleRange で保持(source 側は detail artifact 経路)
      - name: feature-table-typed
        examples:
          - features JSON → Vec<FeatureRecord>、_artifact field を無視、truncated flag を保持
      - name: waveform-bounded
        examples:
          - window(0.1s..0.2s, bins=64) が ≤64 ViewBin を返し全量ロードしない
          - seconds_to_sample が u64::MAX へ saturate する巨大 start_s でも panic せず frames へ clamp される
      - name: spectrum-tile
        examples:
          - tile(&TileRequest{frames 0..16, bins 0..8}) が clamp して Tile を返す
      - name: time-map-offset-rate
        examples:
          - capture_sample から source 秒 = source_range.start 基準の offset+rate 写像
      - name: stream-time-bounded
        examples:
          - ticks(i..j) が PTS window を返し seconds_at が timebase_hz で換算される
          - ticks の start/end は両端 len へ clamp され、start>len・start>end・空 stream ticks(1,2) でも panic せず空 slice を返す
          - metadata の av_timing は Vec<StreamTiming> 配列として解釈され単一 object は拒否
          - timebase_hz==0 の malformed stream で seconds_at が None(inf/NaN を作らない)
  integration:
    rules:
      - name: real-dataset-e2e
        examples:
          - WAV fixture(source+capture)→ job run(ingest→locate→cheap→dataset) →
            JobRow/WaveformView/SpectrumView/TimelineView/実 AlignmentRow+LocateResult→TimeMap/FeatureTable が typed contract で返る
          - clip fixture → job run(ingest→cheap→expensive→dataset) → 実 EventRow が typed contract で返る
          - AVI fixture(movi_order timing)→ job run(ingest→dataset) → metadata.av_timing から 2 stream の StreamTimeView が返る
      - name: headless-boundary-preserved
        examples:
          - 既定 feature の cargo check -p al-gui が gpui 非依存で通る
          - check-ui-gpui/lint-ui-gpui/test-ui-gpui が gpui feature で通る
```

## 検証マトリクス

| 範囲 | 入口 | 記録 |
| --- | --- | --- |
| fmt | `mise run fmt` | exit code |
| typecheck | `mise run check` | exit code |
| lint | `mise run lint` | exit code |
| unit+integration | `mise run test` | exit code |
| headless 境界 | `mise run check-cli-headless` | exit code |
| UI check | `mise run check-ui` / `mise run test-ui` | exit code |
| gpui lane | `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` | exit code |
| ruviform lock | `mise run check-ruviform-rev` | exit code |

## 未達(NotRun/Unsupported)として明記

- 実 OS への `al-desktop` 起動・視覚確認: 環境依存のため本件の
  受入条件に含めない(gpui lane は compile/lint/test まで)。
- AL-G4 実 OS lifecycle・AL-G5 visual/numeric/CLI 正式受入。
- RF #387 renderer による ViewSpec 実描画接続(後段統合条件)。
- 実 desktop 入力 slot(RF #389)。
