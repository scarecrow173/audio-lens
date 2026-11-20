---
id: "KNOW-kWjzzpfxNUayNY2v49kTWz"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "architecture"
title: "システムアーキテクチャ: crate レイヤ・実行モデル・永続化・インターフェース"
created: "2026-11-20"
updated: "2026-11-20"
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

# システムアーキテクチャ: crate レイヤ・実行モデル・永続化・インターフェース

AudioLens は、ゲームのキャプチャ映像と比較用入力をアラインし、音声に何が起きたかを計測して構造化解析データを出力する解析ツールである。本書は crate 構成・レイヤ責務・起動から主要ユースケースまでの制御/データフロー・横断的な不変条件を一枚で説明し、コードを読み始めるためのメンタルモデルを与える。記述は 2026-11-20 時点のコード現状に基づく。ドメイン別の詳細は末尾の「既存 knowledge 文書への導線」に譲る。

## クレート構成と依存方向

Rust workspace は `crates/` 配下の 11 crate で構成され、al-* 間の依存グラフは一本の主幹を持つ非環グラフである。矢印は「依存する」向きを示す。

```text
al-cli ─┐
        ├─► al-service ─► al-analysis ─► al-locate ─► al-dataset ─► al-media ─► al-job ─► al-core
al-gui ─┘
(al-gui は型利用のため al-job/al-media/al-dataset/al-locate にも直接依存するが、al-analysis への直接依存はない。
 al-testkit と al-observability は主幹の外側で、al-* への依存を持たない)
```

| crate | 責務 | 主な ruviform 依存 |
| --- | --- | --- |
| `al-core` | 共有語彙のみ。`JobId`/`InputId`/`ArtifactId`/`AlignmentId` の接頭辞付き ID、`InputRole`、`StageKind`、`Validity`、`Confidence`、in-tree の sha256/base64、`RUVIFORM_REV` 定数 | なし(serde のみ) |
| `al-job` | 不変ジョブ定義、manifest/artifact registry、`Stage` trait、`StageRunner`、staged publish(`fsutil`) | io-file |
| `al-media` | コンテナ検出・デコード、video 特徴抽出、取り込み辞書、`IngestStage` | media-core、audio-core、video-core、media-io、io-avi、io-symphonia |
| `al-dataset` | Analysis Dataset 永続化(manifest・table/tensor/audio store・provenance)、`DatasetStage`/`ExportStage` | なし |
| `al-locate` | source×capture アライン(複数手法の合議)、TimeMap、mixer fit、residual 探索、`LocateStage` | dsp、math、numeric |
| `al-analysis` | cheap/reference/expensive の 3 解析ステージ、特徴・候補イベント・段階残差の算出 | numeric(直接)、dsp/audio-analysis は smoke 確認のみ |
| `al-service` | headless サービス層。`StageRunner` の唯一の所有者、pipeline 構成の正本(`make_stage`)、dataset 読み出しポート(`DatasetSession`) | viz |
| `al-cli` | bin `al`。引数解析・ログ初期化・出力整形に限定し、job 操作はすべて `al-service` 経由 | なし |
| `al-gui` | 表示層。driver-neutral core と feature で切り替える GUI lane | ui、ui-theme、viz、optional で ui-gpui/ui-webview/ui-test |
| `al-observability` | ログ有効化ポリシーの薄い adapter。利用するのは `al-cli` のみ | observability(logging) |
| `al-testkit` | 決定的 fixture(信号・AVI/MP4・temp dir・期待値ヘルパ)。dev-dep 中心、`gpui-real` 向けに optional dep でも持つ | なし |

依存方向に関する規則は 3 つある。

1. crate DAG は実行時の pipeline order(`StageKind::PIPELINE_ORDER`)をなぞるが、両者は別のグラフである。ステージの依存は各実装の `depends_on()` が宣言し、`StageRunner::validate_plan` が「依存先が plan 内で先行する」ことを検証する。例えば `Dataset` は `Ingest` だけに、`Export` は `Dataset` だけに依存し得る。
2. ステージの実装はオーケストレーション層ではなくドメイン crate に住む(`IngestStage` は al-media、`LocateStage` は al-locate、解析 3 ステージは al-analysis、`DatasetStage`/`ExportStage` は al-dataset)。`al-job` は trait と runner だけを持ち、ドメイン型を知らない。
3. GUI 依存は `al-gui` に閉じる。`tools/check-headless-deps.mjs`(`mise run check-cli-headless`)が残り 10 crate の `cargo tree` に windowing/GPU 系 crate(wry/tao/gpui/ruviform-ui-* 等)が混入しないことを fail-closed で検査する。

ruviform-*(別リポジトリの再利用基盤)は全 crate が `Cargo.lock` の単一 SHA に解決されることを `tools/ruviform-lock-rev.mjs` が検証し、`al-core/build.rs` がその SHA を `RUVIFORM_REV` として焼き込む(一致しない場合ビルド失敗)。消費の形は 2 種類で、DSP/数学 kernel(resample・NCC・STFT・Welch CSD・最小二乗等)は al-locate/al-analysis が wrap せず直接呼び、AudioLens 側がポリシーを持つ場所(ログ有効化・file 命名・decoder 選択・viz の dataset 適合)にだけ薄い adapter を置く。

## 実行モデル: 不変ジョブとステージランナー

job は `JobDefinition`(job_id、role 付き入力、free-form config、stage plan)として一度だけ作られ、変化しない。`JobDefinition::create` は入力パスを絶対化し内容の `source_sha256` を pin するため、読めない入力は実行前ではなく作成時に失敗する。再実行は常に新しい `JobId` を発行し、runner は `create_dir` で job_dir を予約する(既存なら失敗)。入力 file を差し替えても同じ job を再実行する道はない。

`Stage` trait は `kind()`/`depends_on()`/`run(ctx)` の 3 点だけを持ち、`ctx` は読み取り専用の manifest・artifact registry・config snapshot を渡す。契約として、ステージ間通信は manifest に登録された artifact id 参照に限定され、ステージ間の隠れた共有状態は許されない。runner の処理順は次のとおりである。

1. `validate_plan` が stage plan の一意性・実装の存在・依存順を検証し、`validate_inputs` が全入力の hash pin を再確認する。
2. `<job>.lock`(job_dir の sibling)を `try_lock` で取り、`create_dir` で job_dir を予約したうえで、stage を 1 つも実行する前に `manifest.json` を永続化する —— 最初の stage 途中での crash でも resume 可能な job が残る。
3. execute loop は plan 順に各 stage を処理する。manifest 上 `Completed`/`Skipped` の stage は成果物を reuse して skip し、`Pending`/`Failed` は依存確認と cancel poll のうえで `run(ctx)` を呼ぶ。返された `ArtifactRecord` を登録して `Completed`+計測時間を記録し、stage 境界ごとに manifest を保存する。`Err` はその時点で run を abort する(fail-fast)ため、部分完了の manifest が残る。cancel は stage 境界でのみ効く協調的なもので、実行中の stage の書き込みを中断しない。
4. `resume` は永続化された manifest から `JobDefinition` を再構成する(inputs/config/plan の完全一致を強制)うえ、`verify_completed_artifacts` が `Completed` 済み各 stage の artifact を実在検査(`params.byte_len` の一致や `params.files` の全員存在)する。欠落・不一致の stage は `Pending` に降格されて再実行される —— durability を緩めた(relaxed tier)成果物の喪失はこの機構で修復される。

`al-service` の `JobService` が `StageRunner` の唯一の所有者であり、`pipeline.rs` の `make_stage` が `StageKind`→実装の唯一の対応表である。CLI と GUI はどちらも stage を直接組み立てず `JobService::{run,resume,export,inspect,list}` を呼ぶ。`al-service` は daemon ではなくライブラリで、実行は呼び出しプロセス内で同期に行われ、並行実行の制御は job 単位の lock file が担う。

config は free-form の JSON で、job 作成時に manifest へ snapshot される。`resolve_config` は `config.dictionary` で指定された外部辞書ファイルの中身を `dictionary_text`+sha256 として config 内に pin するため、後からファイルが編集・削除されても resume は同じ内容を再現する。manifest の config snapshot は resume と dataset 再 open の両方で一致が強制される。

## データフロー: 入力から export まで

主ユースケース「`al run` → `al export`」の流れは次のとおりである。

```text
入力ファイル(role: capture_bug/capture_reference/source_asset/other、sha256 pin)
  → Ingest     マジックバイトでコンテナ判定 → ruviform 系 demuxer/decoder で PCM 化
               → inputs/<IN-*>/{decoded.f64le, metadata.json, video_features.json, video_thumbs.json}
  → Locate     source_asset × capture_* 全対を 48kHz 比較空間へ正規化
               → 4 手法(FullNcc/BandNcc/GccPhat/SpectralEnvelope)の evidence を合議
               → locate/<src>/<cap>/{result.json, time_map.json} + normalized PCM + residual
  → CheapAnalysis   全入力の常時オン観測(level/spec/stereo/trans/mod/noise/dig、video ROI、avsync)
  → ReferenceAnalysis 受理ペアの比較(局所 lag、ゲイン、帯域別、伝達関数、channel fit、残差 R0–R3)
  → ExpensiveAnalysis 候補イベント抽出 → 区間計画 → fine STFT で 7 family の深掘り
  → Dataset    analysis/ の確定(inputs/*.wav・metadata、locate summary、manifest.json)
  → Export     export/<name>/ へ relocatable な複製(絶対パス匿名化)+ golden snapshot
```

この順序は `StageKind::PIPELINE_ORDER` であり、plan が必須とする先行関係とは一致しない。`CheapAnalysis` は `Ingest` のみを、`ExpensiveAnalysis` は `[Ingest, CheapAnalysis]` のみを依存に宣言するため、reference を含まない cheap 候補駆動でも expensive 解析が起動できる。ペア証拠が必須の expensive family は、ペア不在時にスキップせず `"pair evidence unavailable"` の無効観測を記録する。

永続化は 2 層に分かれる。

- **job 層**: `manifest.json` が artifact の lineage(`produced_by`/`inputs`)と stage 記録を保持する正本。stage 間の受け渡しは manifest 経由の artifact id 参照で、stage 実装は相手の内部を知らない。Ingest の `decoded.f64le` は送信元 native scale の interleaved f64(正規化も downmix もしない)で、下流は `al_locate::decode` の共有 loader が manifest 経由で解決・検証して読む —— 再 decode は行われない。
- **dataset 層(`<job>/analysis/`)**: 検査・表示・export の対象となる永続 dataset。role(`inputs`/`locate`/`intermediate`/`features/{cheap,reference,expensive}`/`residual`/`events`。`features/cross_capture` は宣言済み一覧になく、最初の `register` で動的に追加される)ごとに `StoreKind::{Table,Tensor,Audio,Json}` の store が置かれる。解析 3 ステージは `open_analysis_dataset` が一本化した `open_or_create` 契約で `analysis/` に直接 append する(`schema_version`/`inputs`/`config` の完全一致、不一致は互換層なしで失敗)。`DatasetStage` は入力 PCM の WAV 化と locate summary の materialize を担う finalize 側で、ExportStage は `analysis/` 全体を relocatable に複製する。

読み出し経路は書き込みと対称に分離されている。`DatasetSession`(al-service)が dataset に対する range-only のクエリポート(`overview`/`events`/`locate`/`features`/`tensor_region` 等)を提供し、`DatasetWaveform`/`DatasetSpectrum` が ruviform-viz の port を dataset 上に実装する。waveform は永続 WAV の範囲読み、spectrogram は永続化済み STFT tensor の region 読みであり、表示のために解析を再計算しない。

## 観測の規約

解析系には、どの層を読んでも同じ振る舞いを示す横断的な規約がある。

- 「測定であって評決ではない」(ADR `measurement-over-verdict`)。出力には合否・severity・root cause を表すフィールドが存在しない。`CandidateEvent` は kind・時間範囲・`strength`(閾値超過比)・証拠リンクを持つ「着目すべき測定」であり、ID は内容由来の `EVT-<alg>-<input>-<ms>-<hash>` で決定的である。
- 「異常なし」と「測定不能」を分離する。`Validity` は `Valid` 以外に 10 の理由(`SilenceInsufficientEnergy`/`AmbiguousAlignment`/`DecodeInputUnavailable`/`Unsupported` 等)を持つ。測れなかった観測は skip せず、スキーマ完全な record として `NaN`+mapped validity で記録する(expensive 各 family は valid 経路と同一の feature-id 集合を `emit_invalid` で出す)。`Value::Scalar` の NaN は JSON null として往復する。
- 確度を勝手に上げない。`Confidence` は `Low<Medium<High` の全順序で、下流は `min(upstream, own)` を取る。non-Valid な trigger で選ばれた expensive 区間では、推定自体が `Valid` でも `trigger_val` へ demote される。
- 冪等で決定的。入力ごとの処理は順序付き slot pool(`MAX_STAGE_TASKS=4`、resident PCM を抑える上限)で並列化するが、dataset 書き込みと artifact 採番は逐次に揃える。`reset_store`(既存 store 削除)→書き込み→`register`(`(role,path)` で置換)の手順と内容由来 ID により、再実行は同一内容・同一順序の dataset を再生成する。
- publish-after-complete。file 出力はすべて staged publish(`.rvf-tmp-*` sibling→atomic rename)を通し、tensor store は `committed` flag が立たないと reader が拒否し、manifest は常に最後に書かれる。durability は呼び出し側が `PublishOptions` で選ぶ(manifest/metadata は `FileAndDirectorySync`、再生可能な bulk は `BestEffort`)。
- provenance を常に押す。入力は `source_sha256` で pin され、全 feature record/artifact は `algorithm_id`/`algorithm_version`/parent ids/`params["ruviform"]`(rev・precision `f64`・crate 一覧)を持つ。

## インターフェース層と GUI

`al-cli`(bin `al`)は `run`/`resume`/`export`/`inspect` の 4 subcommand を持ち、処理はすべて `al-service` 経由である。ログは `al-observability` で初期化され、既定は OFF(`--log`/`AL_LOG` で有効化)。sink 語彙は `{stderr, file}` に閉じて stdout を含めない(`inspect` の stdout JSON を守る)のが AudioLens 固有ポリシーで、file sink は `<base>/logs/al.YYYY-MM-DD.log`(日次・7 世代)である。

`al-gui` は driver-neutral core と feature で切り替える lane に分かれる。core は `DesktopSession`(window 寿命管理 —— 実行中 job のある window の close は kill せず defer し、job 終了後に再 close する)、`AlGuiService`(`rvf.ipc.v1` 上の `al.*` op 群: job 実行・dataset session・view payload)、`view/*`(解析結果を `ViewSpec` へ verbatim 射影。再計算しない)の 3 要素で構成される。lane は 2 本ある。

- `gpui` feature → bin `al-desktop`。現行のネイティブ lane で、GPUI 具象型は `ruviform-ui-gpui` 内部に封じる。`gpui-test` は OS を stub した headless parity 検証、`gpui-real` は実 OS window 向け test-only bin `al-gui-real-scenario` を出す(両者は `compile_error!` で排他 —— stub が実 window 証拠を汚染するのを防ぐため)。
- `webview` feature → bin `al-gui`。wry/tao ホスト + `crates/al-gui/assets/` の埋め込み frontend(Canvas2D、`rvf://` プロトコルで IPC)。**この lane は AL#189 で物理撤去が予定されており、現在 RF#465 に blocked されている**。撤去対象は `al-gui` bin・`webview` feature・`crates/al-gui/assets/`・webview-verify CI lane であり、repo 直下の `ui/`(`@audiolens/gui-headless`、ruviform ui を vendor した TS 統合層)は別物で維持対象である。

データ経路も lane と対応して 2 本ある。webview frontend と `ui/` TS 層は `AlGuiService` の IPC op を呼び、GPUI lane は `al-service` を in-process で呼んで `ViewSpec` へ射影する。どちらの経路も `al-service` の境界を越えて `al-job`/`al-dataset` の内部には入らない。

## 既存 knowledge 文書への導線

ドメイン別の詳細は次の既存文書が担う。本書で名前だけ出た概念の定義・経緯はそちらを参照する。

| 文書 | 対象 |
| --- | --- |
| `media-ingest.md` | 入力モデル・コンテナ/コーデック取り扱い・SHA-256・タイミング観測 |
| `locate-and-tracking.md` | 複数手法の合議・曖昧性検出・TimeMap/トラッキング |
| `analysis-pipeline.md` | 解析 3 層ステージ・特徴ファミリ・validity/確度伝播・候補イベント・段階残差 |
| `dataset-artifact-store.md` | Analysis Dataset の永続化スキーマ・追記プロトコル・PCM ビュー |
| `job-orchestration.md` | 不変ジョブ・manifest・ステージランナーの防御設計 |
| `gui-integration-headless.md` | ruviform controls/viz 統合・SemanticGuiTest・実 OS E2E |
| `native-gui-desktop.md` | DesktopSession・テーマ・ViewSpec 射影の詳細 |
| `viz-export.md` | dataset 由来の表示/export 製品・決定性・パッケージ |
| `observability.md` | logging・stage 計時・span 計装・プロファイリング |
| `dsp-math-kernels.md` | DSP/数学基盤(al-math・ベクトルプリミティブ・相関 plan) |
| `performance-engineering.md` | 特性化・baseline・等価性ゲート・最適化方法論 |
| `ci-architecture.md` | 独立 CI・fail-closed 集約・Ruviform アクセス |
| `ruviform-collaboration.md` | Ruviform 依存解決・契約 API・消費者検証の運用 |
| `docs-process.md` | 文書運用・status 規約・文書管理の教訓 |

関連 Issue/PR: #252(本文書)、#189(webview lane 撤去予定、ruviform#465 に blocked)、AL#131(`ui/` TS 統合層)。
