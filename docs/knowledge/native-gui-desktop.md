---
id: "KNOW-DkaV632pgTaNlez5btBaNC"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "architecture"
title: "ネイティブ GUI: DesktopSession・テーマ・ViewSpec 射影"
created: "2026-11-19"
updated: "2026-11-19"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ネイティブ GUI: DesktopSession・テーマ・ViewSpec 射影

`al-gui` の GPUI デスクトップ基盤は、`UiRuntime` を内包する driver-neutral `DesktopSession`/`DesktopPolicy`、composition root `al-desktop`、製品テーマ/preferences、view model adapter の層で構成される(AL#166 配下の連続 slice)。本書は session 設計、close/exit ポリシー、テーマ、ViewSpec 射影の知見をまとめる。

## driver-neutral session

`DesktopSession` は `&mut dyn WindowDriver` だけに依存し GPUI/OS 型を持たない —— `Tick`→`drain_jobs_routed`→`post_event`、`Action(Intent)`→`dispatch_window_intent`。GPUI 型は `bin/al-desktop.rs` のみに閉じ込め、composition root 分離が headless 検証を可能にする。live window 集合の正本は `WindowDriver::windows()`(契約上の designated source of truth)で、session 側の shadow set は廃止した。`RoutedEvent.window=None`(ownerless)は全 live window へ `post_event`。

## CloseRequested と exit の決定順序

CloseRequested の policy 優先順位: 製品 veto → `Veto`、最終 live window かつ job 実行中 → `Defer`+`deferred_close` 登録、それ以外 → `Approve`。`Veto` でなく `Defer` を選んだ理由は、Veto が close 意思を取消すのに対し Defer は close-pending として意思を保持し、job 終端で `request_close` を再発行して新しい CloseRequested として policy を再評価させるためである(review 第4ラウンド)。`resume_deferred_closes` は Tick drain 直後に `!has_running_jobs()` なら再発行する。

`exit()` 発行は `maybe_exit` 一点に集約し、`exit_pending && windows().is_empty() && !has_running_jobs()` のときのみ。`UiRuntime` の job worker は JoinHandle を持たない detached thread のため、実行中に exit すると未終端 kill され「window close が job を暗黙停止しない」受入を破る。`exited` latch で exit 後の重複発行・event dispatch・新規 job spawn を閉塞する。`request_exit` は `destroy_window` 直呼びではなく全 live window へ `request_close` を発行し veto/Defer を踏み越えない。

policy 既定値: `cancel_jobs_on_destroy = false`(job を継続)、`exit_on_last_window = true`。

## テーマと preferences

`theme/base.tokens.json`(DTCG profile、light/dark)+`product.tokens.json`(semantic `color.*` を base palette へ alias、mode 差は `dev.ruviform.modes`)。`theme.rs` の `documents()`+`resolve(mode)` は `ThemeResolver` へ委譲。`GuiPreferences` は theme_mode/window/base_dir を保持する。

## View model と射影

`view/source.rs` の `StreamTimeView::ticks` は `start`/`end` を両端 `len` に clamp(start>len・空 stream で panic しない)。`WaveformView::window` も同じ両端 clamp(`seconds_to_sample` の u64::MAX saturate で `start+1` overflow を `min(frames)`+`saturating_add` で閉塞)。`timebase_hz==0` の `seconds_at` は `None`。`StreamTimeView::from_metadata` は永続化 shape `av_timing` の `Vec<StreamTiming>` のみ解釈 —— 架空 shape を受理するとデッドコードになる。

schema 必須列は欠損で `Invalid` とし、0 補完で `SampleRange{0,0}` を捏造しない。`strength` の JSON null は永続化 NaN 表現として `f64::NAN` に復元する(dataset 側 `cell_to_json` の NaN→null と対称)。`FeatureRecord`/`LocateResult`/`StreamTiming` は再定義せず typed 保持で表現ドリフトを避ける。

## ViewSpec 射影(AL-G2〜G10)

画面は「解決済みモデル→Spec の 1 対 1 写像」が基本形。`source_screen` で Waveform/Meter/TimeAxis を射影し、Waveform/Meter は同じ `window` 結果を一度だけ読む。Meter の `peak` は `max(|min|,|max|)`、gap(`count==0`)は `NaN` のまま。TimeAxis(ruler)は viewport の frame 範囲から刻みを決定(区間数 10 以下、tick は viewport 内のみ、1ms〜10日まで拡張 —— 約100分超長尺で Err になる問題を修正)。

`spectrogram_screen` は `ruviform-viz` の `Tile` を列ごとに `SpectrogramColumn` へ写し、値と gap を再計算しない。列の frame 区間は `Spectrogram::tile` の等分割と同じ式で、`count` は「列の有限セル数」として扱う(上流契約化は RF#443)。`workspace_screen` は Source/Spectrogram/Analysis の 3 tab Tabs を組み、読めない節は理由 Label に置換する。`desktop_screen` は root Tabs(Jobs・Run・Dataset)を組み、各 tab が独立に読み失敗 tab は `unavailable: <理由>` Label Panel になる。

`desktop_screen` は「要求された job id」と「manifest 内の job id」を区別し、必ず要求 id で `open_dataset` する(review B1 修正: manifest が書き換えられていても別 job の dataset を表示しない)。`job_list` は 1 行 1 job の Table、`run_screen` は `RunOutcome::from_manifest` で manifest stage を写す。

## Tabs 選択契約

root Tabs は `selection: Some(DESKTOP_TARGET)`(`al.desktop`)、workspace Tabs は `Some(WORKSPACE_TARGET)`(`al.workspace`)。`ACTION_TAB_SELECT` は ruviform の `TAB_SELECT_ACTION` を再利用(AL が別定義すると表現が二重化するため)。`service::command()` を `(target, action)` match にし、index を `u64` かつ `index < limit` で範囲検証、`{"status":"applied","index":n}` で応答する —— ruviform 側事前検証と重複するが IPC 境界の防御として残す。表示 tab の正本は ruviform `ViewHost` で、consumer 側に第二の状態を持たない。

## job receipt と command

`rvf.ui.command.v1` の command 契約で Jobs 表の対話選択・Run tab の Resume/Cancel Button・session の選択追跡/dirty 再射影を実装した。`command.rs` に op 名・target・action・`decode_action` を集約する。receipt の decode・phase 遷移は ruviform 側契約を利用し AL 側再実装はしない(責務境界)。`Cancelled` の resume job 終端 receipt は ruviform の分類依存で AL 側直接検証は未実施。

## 設計上の教訓

- 「job を kill しない」と「不可視な無期限待機の禁止」を両立するには `Defer`(close-pending 保持)+`resume_deferred_closes` の組合せが有効。
- 強制 `destroy_window` 直呼びは veto 契約の意味を壊す —— quit/cleanup でも `request_close` を再発行して通常 flow に乗せる。
- 永続化 schema の読み出しは「実在する shape のみ」受理し、0 補完・架空 shape を拒否する(捏造防止)。
- drawn parity の期待値は contract の規則だけから導き「空の期待値を通さない」ことで偽陽性を防ぐ(期待値が実装と同じ式の複製だと同じ誤りを両方に含む)。
- Tabs の非 active 子は mount されない —— 全画面を一度に検証できず、tab 切替ごとの mount で検証する。
- 上流に API が無い規則は複製+テストで上流実出力に固定し、上流 Issue で差し替え条件を記録する運用(RF#443 の `spectrogram_columns` 複製は ruviform#443 が OPEN のため維持)。

関連 Issue/PR: AL#166、RF#387、RF#389、RF#421、RF#426、RF#429、RF#434、RF#435、RF#436、RF#442、RF#443、RF#446、RF#448、RF#451、RF#453、RF#456、RF#458、ruviform PR #458、AL PR #168。
