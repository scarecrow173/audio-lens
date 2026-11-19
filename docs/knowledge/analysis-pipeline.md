---
id: "KNOW-EpXLDAeWGabg0W1wZJcxHC"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "architecture"
title: "解析パイプライン: 3層ステージ・特徴ファミリ・候補イベント"
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

# 解析パイプライン: 3層ステージ・特徴ファミリ・候補イベント

`al-analysis` クレートは、定位済みの素材/キャプチャを対象に観測特徴を算出し Analysis Dataset に書き込む解析層である。基本原則は「測定を記録し、評決は出さない」ことで、イベントは確認済みバグではなく `strength` を持つ「着目すべき測定」として扱う(ADR `measurement-over-verdict`)。本書はステージ構造・特徴ファミリ・validity/確度伝播・候補イベント・段階残差の設計をまとめる。

## ステージ構造と依存

解析は 3 層の Stage からなる。`CheapAnalysisStage` は `depends_on=[Ingest]` で常時オン、`ReferenceAnalysisStage` は `[Ingest, Locate, CheapAnalysis]`、`ExpensiveAnalysisStage` は `[Ingest, CheapAnalysis]` で reference なしの cheap 候補駆動でも起動できる。`LocateStage` は source/capture の片方が無い job では `Err` ではなく `Ok(vec![])` を返す。依存検証は runner 側の `validate_plan` に一本化しており、CLI 独自のチェックは設けていない。

dataset への追記は「検証付き open + 置換セマンティクス + 明示失敗」の三点で冪等性を担保する。`open_or_create` は manifest の `schema_version`/`inputs`/`config` の完全一致を検証し、不一致・破損は `InvalidData` で失敗(互換層・マイグレーションなし)。`write_*` は対象 `role/name` の既存ストアルートを削除して再作成し、`register` は `(role,path)` 一致で既存エントリを置換する。作成主体は最初に `analysis/` を必要とするステージ(通常 Cheap)で、`InputReference` には pinned sha256 が必須である。

## cheap ファミリ(常時オン観測)

`cheap::analyze_input` は 1 入力の 7 ファミリ全特徴を生成する。`level.*`(RMS/integrated/trajectory/crest)、`spec.*`(centroid/bandwidth/entropy/flatness/flux/rolloff/slope/ピーク/ノッチ)、`stereo.*`(coherence/ILD/IPD/MS ratio/pair corr、2ch 入力のみ)、`trans.*`(onset/self-similarity)、`mod.*`(AM depth/freq/lag drift)、`noise.*`(floor/hum/tonal)、`dig.*`(click/clip/discontinuity/dup block/quant/stuck/zero run)。時系列(trajectory)は JSON に展開せず `intermediate/` ロールの tensor として保存し、JSON レコードには `Value::Tensor { array, region }` 参照を載せる。

設計上の細部として、計算不能な値は `Silence`/`InsufficientSupport` で明示し、silence 入力でもレコード自体は出す。spectral flux の先頭フレームは前任フレームが無いため 0 を入れて tensor の行数を揃える。`dig.quant_step_est` はヒューリスティックで、高ビット深度素材では感度が落ちる既知の限界がある。

## reference ファミリ(ペア比較)

`analyze_pair` は受理済み alignment ペアを対象に、ラグ系(`ref.lag_*` の局所両方向相関 τ(t)、median/MAD/drift/step)、ゲイン系(`ref.gain_ls`/`gain_400hz`/`rms_ratio`)、帯域系(1/3 オクターブ、`ref.band_correlation_min`、band coherence、PSD 比)、伝達関数系(Welch CSD→`H(f)`/位相/群遅延)、チャンネル系(`ref.mixer_fit`/`channel_response`/極性/M-S/crosstalk)、類似度系、および段階残差 R0–R3 を算出する。

帯域別ラグ補償ゲイン `ref.band_gain_m2`(#38)は、PSD 比ではなく「帯域ごとの最良ラグ探索(既定 ±40ms)→ 符号付き LS ゲイン → 極性別の相関上位半分の中央値」が正本である。非コヒーレントな PSD 比は周波数特性の判断に使えず高域リフトを偽装し得るためで、`ref.band_gain_psd_ratio` は低信頼の二次観測として別 ID に残した。帯域の準備は `InputCache::prepare_with_bands`+`bandpass_many` を再利用し、絶対相関または MSC < 0.5 の場合は low validity に落とす。欠損セルは NaN+support 0 で別群で補完しない。

残差ラグ推定で片方向 `ncc` を使わない理由は、同長セグメントでは探索範囲が 0 に縮退するためで、`corr_at` の両方向相関を使う。provenance は producer の algorithm/alignment/parents を引数化しないと共有 writer が誤った来歴(`cheap.v1`・alignment None)を刻む —— 実際にレビューで検出され修正された。

## 段階残差 R0–R3(カスケード設計)

段階残差は「各モデルが前段残差に対して fit し、前段残差から差し引く」カスケード方式である。R1(スカラ LS)・R2(ridge mix)は最小二乗の性質で単調減少が保証されるが、R3(FIR)はそうではないためエネルギー実測ゲートを入れた。全モデルを cap から独立に差し引く当初設計では、R3 が R2 を悪化させ得る。R3 の FIR は Welch `H = E[conj(X)·Y]/E[|X|²]` の irfft で畳み込み長は 4096 tap に制限する。未受理 alignment は空セグメント+`AmbiguousAlignment` で統一的に invalid 行を出し、ステージは abort しない。

## validity・確度・resolution の伝播規約

`Confidence` は `Ord`(Low<Medium<High)で下流は入力と自身の確度の min を取る。`Validity` は順序を持たせず、依存元からのマッピングで伝播する: alignment 不確か→`AmbiguousAlignment`、帯域コヒーレンス不足→`LowCoherence`、無音/エネルギー不足→`SilenceInsufficientEnergy`、decode 不可→`DecodeInputUnavailable`、その他→`ComputationFailed`。上流が非 Valid なら下流は推定せず「無効な観測」として記録する —— 数値を出さないことが規則である。

`FeatureRecord.resolution_class` は 7 クラス enum(Sample/Fine5ms/Mid20ms/Broad100ms/Wide500ms/Seconds/Segment)で、クラス名と実 window/hop の両方を永続化する。`Resolution.sample_rate`(#33 で承認)は原入力レートではなく解析対象 PCM の正規化後レートを保持する。ペア横断の summary 測定(level_transfer、mix 系、local_time_map)は `Channel(0)` に紐付けず `push_pair` で記録し、実際に使う channel を params に記録する —— 単一 capture channel を主張しない設計。

## 候補イベントと expensive トリガ

`CandidateEvent` は kind・時間範囲・strength・validity・confidence・証拠リンク `{"artifact","rows"}` を持ち、ID は `EVT-<alg>-<input>-<ms>-<hash>` の内容由来ハッシュで決定的である(ステレオ per-channel 行や異なるペア由来の行が衝突しないよう channel/artifact をハッシュ入力に含める)。抽出の実行主体は `ExpensiveAnalysisStage` 先頭で、cheap/reference を dataset から読み `events/candidates` テーブルへ書いてからトリガ評価を行う fused 構成(#33 で仕様承認。「候補のみ再実行」はできないが観測の保存は独立 artifact)。

閾値ルールは `config.analysis.events.*` でオーバーレイ可能にし、`extract_candidates` が FeatureRecord に適用する。設計上の教訓が二つある。(a)「閾値超過」を一律 `abs()` で比較すると負の閾値(noise floor −40dB 等)で常時ヒットするため、比較ルールに per-rule の `map` 変換を持たせ双方向量だけ絶対値化する。(b) `noise.floor_dbfs` の高値はドロップアウトではないため `ElevatedNoiseFloor` kind を別立てした —— 誤分類より後から専用 kind を足す方がよい。

`plan_regions` はイベントを前後窓で拡張し近接区間をマージして解析区間を作り、トリガ理由(`candidate:<event id>`/`always_run`/`manual`)を保持する。区間が Valid トリガを持たない場合は推定を早期終了し全 feature を `NaN`+マッピング済み validity で記録する。expensive 出力は `features/expensive/<input>__rNNN.json` の別パスで、cheap/reference 行は書き換えない。イベントゼロでも 0 行の `events` テーブルを書き、`not_produced` と区別する。

## expensive 深掘りファミリ

`analyze_region` の深掘り特徴は `expensive/{transfer,ir,codec,dynamics,noise,channel,timing}.rs` のサブモジュールに分割されている(振る舞い不変の分割をファミリ追加に先行させた)。主なファミリ: 高域 cutoff と辞書差、ケプストラム由来 echo/pre-echo、SRC ミラー相関、極性非対称/THD、dynamic range/attack/release、repeat/gap 検出、spectral hole/コーデック痕跡、IR decay、dynamics curve、noise 分類、帯域別混合係数、timing/splice。

ケプストラム遅延推定の設計三原則: log スペクトルから平滑包絡を引いてリップルのみ残す、逆 FFT 長を STFT bin 数に揃える、argmax はフラット/縮退でも必ず存在するため prominence > 0.5 のときのみ Valid としそれ以外は `InsufficientSupport`。ペア証拠必須のファミリはペア不在時にスキップせず `pair evidence unavailable` の invalid 観測として記録する。

キャプチャ間経路校正(#41)は job スコープの観測で、共通ソースの `transfer_h` 振幅比を bin 単位の中央値に集約する。共通ソースなしは `Validity::Unsupported`(観測不能)で、支持不足の `InsufficientSupport` と区別する。

## 運用・レビュー上の教訓

- 「候補なし/支持なし」を `0` や `NaN`+`Valid` で出すと有効測定と区別できない。評価件数(有効)と評価値(未支持)は別フィールドで分離する(#29 の 9 項目で精緻化)。
- 循環表現(IFFT 出力)の窓解析は、ピーク位置 wrap を考慮して一度だけ並べ直さないと pre-peak リンギングが別窓に混入する。
- フィルタ後配列を元 index で参照する誤ラベル化(不均一 ch 長で ch1 を ch0 と誤認)は典型バグ。フィルタは (slice, 元 index) の組を保持する。
- 読み出し失敗を `.ok()` で黙殺するとデータ破損を「候補なし」に誤変換する。`parse_feature_records`・辞書読み込みは明示エラー。
- 仕様と実装の差異は「機能的欠落なし」で閉じず、「実装/仕様改訂/defer」の選択肢と承認結果を対応表で記録する(#33 の運用)。未実装を適合済みにしない。

関連 Issue/PR: #29、#30、#32、#33、#38、#41、PR #26。
