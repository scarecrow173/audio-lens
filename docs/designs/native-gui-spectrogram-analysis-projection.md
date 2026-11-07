---
id: "DESIGN-Hr5Mn9Yw3StP7qXzL2jBdE"
type: "design"
status: "draft"
title: "Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity の設計"
created: "2026-11-08"
updated: "2026-11-08"
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
  implements:
    - "../specs/native-gui-spectrogram-analysis-projection.md"
  implemented-by:
    - "../plans/native-gui-spectrogram-analysis-projection.md"
  depends-on:
    - "../specs/native-gui-product-screen-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-spectrogram-analysis-projection.md"
    - "https://github.com/scarecrow173/ruviform/issues/434"
    - "https://github.com/scarecrow173/ruviform/issues/435"
    - "https://github.com/scarecrow173/ruviform/issues/436"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-spectrogram-analysis-projection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity の設計

## 概要

第 3 slice までの `view::screen`(Source 画面)に、Spectrogram と Job/Run/Analysis の射影を加える。
決めることは次の 5 点である。

1. Spectrogram の列写像(列の frame 区間・count・magnitude_range の出所)。
2. Job/Run/Analysis を Label/Table のどの組み合わせで表すか。
3. al-desktop の mount 形(どの画面を一つの window に載せるか)。
4. drawn parity の期待値の出所。
5. 上流の欠落(観測できない plan・Tile の列契約)の扱い。

## 責務の境界

| モジュール | 責務 | GPUI 型 |
| --- | --- | --- |
| `view::screen`(拡張) | `Tile` → `SpectrogramSpec`、`ViewBin` → Waveform/Meter/TimeAxis(既存) | 持たない |
| `view::analysis_screen`(新規) | events・alignment の typed 行 → 節(Label + Table) | 持たない |
| `view::run_screen`(新規) | `RunOutcome`・`JobRow` → 状態 Label と stage 表・job 一覧 | 持たない |
| `view::workspace`(新規) | dataset 引数から Source・Spectrogram・Analysis を読み、Tabs の 1 画面へ束ねる | 持たない |
| `view::mod`(拡張) | Label/Table/Panel の構成と数値・tag の共通写像(一箇所) | 持たない |
| `bin/al-desktop.rs` | `workspace_screen` の結果を `ViewHost` へ mount し `GpuiDriver` へ渡す | 持つ(composition root) |
| `tests/gpui_parity.rs`(feature `gpui-test`) | `GpuiObserver` のみで観測し、Fake 期待値と照合 | 持たない |

`view` 配下は ruviform-ui / ruviform-viz / al-service の driver-neutral 型だけに依存する。

## D1: Spectrogram は Tile の列を 1 対 1 で写す

```text
Tile { frames, cols, rows, values(row-major, row 0 = lowest Hz) }
  → SpectrogramSpec { viewport = frames, rows, magnitude_range, columns }
     SpectrogramColumn[c] { start, end, count, magnitudes[r] = values[r*cols + c] }
```

- 値は再計算しない。`magnitudes` は Tile の値をそのまま写す。`NaN` のセルは gap として残す。
- 列 `c` の区間は `Spectrogram::tile` の内部規則と同じ等分割とする(`span = max(frames.len, 1)`、
  `cf0 = span·c/cols`、`cf1 = max(span·(c+1)/cols, cf0+1)`)。区間は `frames.start` だけずらす。
  上流に公開規則が無いため複製する。複製の正しさは、上流の `tile()` の実出力に対する単体テスト
  (`columns_split_span_like_ruviform_tile_when_not_divisible`)で固定する。
  規則の上流固定は scarecrow173/ruviform#443 で依頼する。
- `count` は列の**有限セル数**とする。`count == 0` ⇔ 全セルが `NaN` の gap 列。
  `SpectrogramColumn::count` の契約上の意味(有限 sample 数)とは異なる。Tile は sample 数を持たないため、
  この差は設計上の逸脱として IR に記録する(上流の判断は ruviform#443)。
- `viewport` は `tile.frames`(STFT frame 単位)とする。Source 画面の viewport(audio frame 単位)とは
  単位が異なるため、同じ Panel に置かない(D3)。
- `magnitude_range` は consumer が宣言する。`workspace` では「0 から表示 tile の有限最大値まで、
  有限値が無い場合・最大が 0 の場合は `0..1`」とする。Tile の値を変換しないため、range の決定だけが
  consumer の判断として残る。
- 上限: 読み出す STFT frame は先頭から `SPECTROGRAM_MAX_FRAMES = 4096`、出力は `128 × 512 = 65536` セル
  (`MAX_SPECTROGRAM_CELLS` ちょうど)。全量の一括読み出しを避けるための bounded query である。
  表示範囲は Label(`frames a..b of N`)で明示する。

## D2: Job/Run/Analysis は Label と Table で表す

| 対象 | kind | 内容 |
| --- | --- | --- |
| Run の状態 | `Label` | `job <id>: completed c/t, failed f`(stage の status を数えた表示文字列) |
| stage 表 | `Table` | 列 `stage / status / duration_ms / error`。失敗理由と実行時間を欠落させない |
| job 一覧 | `Table`(List = 一列) | 列 `job`。行は job id |
| events 件数 | `Label` | `events n of N`(N は dataset 全体の件数) |
| events 表 | `Table` | 列 `event / kind / input / start_s / end_s / strength / confidence / validity` |
| alignment 表 | `Table` | 列 `source / capture / alignment / status / validity / confidence / agreeing_methods / capture_start / capture_end` |

- **非有限の数値は欠損セル**(`None`)とする。`strength` が `NaN`(JSON では `null`)のとき 0 を補わない。
  Table は `NaN` を拒否するため、欠損セルが唯一の表現になる。
- **行数の上限**: 各 Table は `MAX_TABLE_ROWS`(4096)を超えると `ViewError::Invalid` を返す。
  events は `session.events(0, MAX_TABLE_ROWS, …)` で bounded に読み、総件数は Label で示す。
- **節単位の構成**: events と alignment は別々に読み、読めない節だけを理由の Label に置き換える。
  dataset に `events/candidates` が無くても alignment 表は表示する(`workspace` の実 e2e で確認)。
- **Job/Run の mount は行わない**: al-desktop は job の選択と run 結果の入力源を持たない。
  射影関数とテストまでを本 slice で完了し、mount は AL-G4 へ送る(spec の Non-goals)。

## D3: al-desktop は Tabs の 1 画面に Source・Spectrogram・Analysis を載せる

- dataset 引数 `<dataset-root> <input-id>` を指定したときは `workspace_screen` の結果(Tabs)を mount する。
  省略時は従来どおり空の Panel。
- 採用理由: 3 画面は時間軸の単位が異なる(Source = audio frame、Spectrogram = STFT frame)。
  同じ Panel に並べると ruler と viewport の単位が食い違う。Tabs なら各画面が自分の単位を保てる。
  Ruviform の `TabsSpec` は既存の kind で、新しい契約拡張を要さない。
- 棄却案: Source の Panel に Spectrogram を子として追加する案。時間軸を揃えるために
  STFT frame → audio frame の換算(`FrameGrid`)を画面構成に持ち込むことになり、換算の責務が
  射影層に広がるため棄却する。
- Ruviform の `Tabs` は **active の子だけを mount する**(`ViewHost::observe` は非 active の子に対して
  `None`)。テストは tab を切り替えて各画面を mount する。
- 読み出しの失敗(`events/candidates` 未生成、`intermediate/` 未生成など)は各節・tab の Label に示し、
  画面全体は開く。Source(`inputs/<id>` の読み出し)の失敗だけは `workspace_screen` のエラーとする。

## D4: drawn parity の期待値は mount した spec から導く

- `GpuiObserver::drawn(window)` の結果を、同一 `ViewSpec` の payload から独立に導いた期待値と照合する。
  期待値の規則は contract の描画規則だけを写す:
  - Meter: `count == 0` と非有限 peak を除外。x は `(at - start) / max(span, 1)`、範囲外を除外、
    peak と rms を `[0, 1]` に clamp、rms が非有限なら `None`。
  - TimeAxis: x は同じ式、`[0, 1]` の外の tick を除外。
  - Waveform: `count != 0` かつ min/max が有限の bin の frame 範囲。
- 期待値の導出は GPUI 側の plan 関数を使わない(同じ関数を呼ぶと照合が自明になるため)。
- 数値は `f32` 経由のため、比較は絶対誤差 `1e-6` とする。
- 描画 plan が空の期待値を通さない(`assert!(!want.is_empty())`)ことで、空同士の一致による偽陽性を防ぐ。

## D5: 上流の欠落は NotRun と Issue で扱い、AudioLens に回避策を置かない

- `DrawnPlan` は Spectrogram・Label・Table を公開しない(`627e4bc2` の `collect_drawn` は `_ => {}`)。
  この観測は AudioLens からできないため NotRun とし、scarecrow173/ruviform#442 を起票した。
  AudioLens 側で描画を再実装して照合する回避策は置かない(照合対象そのものを作り直すことになるため)。
- Tile の列区間・count の規則は上流に無い。D1 の複製は一時的な措置であり、理由と解消条件
  (ruviform#443 で規則が固定されたら `spectrogram_columns` を上流の関数へ置き換える)を IR に残す。
- Label/Table の描画は `GpuiObserver` の snapshot にも現れない。Fake 層の a11y 木(`ViewHost::a11y_node`)
  との突き合わせは、本 slice では行わない(GPUI 側で観測手段が無いため)。

## 代替案と棄却理由

- **Spectrogram を Source の Panel の子にする**: D3 のとおり、時間軸の単位が揃わないため棄却。
- **列の count に有限 sample 数を求める**: Tile から導けず、再解析になるため棄却(D1 の有限セル数を採る)。
- **Job/Run を Panel title の文字列で表す**: 表の構造が失われ、契約の迂回になるため棄却(spec の方針どおり)。
- **drawn の期待値を GPUI の plan 関数で作る**: 照合が自明になるため棄却(D4)。

## 検証方針

- headless: `cargo test -p al-gui`(spectrogram・analysis_screen・view の e2e)。
- gpui-test: `mise run test-ui-gpui`(gpui_parity の drawn 照合を含む)。
- 数値非回帰: `mise run test`(workspace 全件)。
- 境界: `mise run check-cli-headless`(headless 依存木に GUI crate が無いこと)。
