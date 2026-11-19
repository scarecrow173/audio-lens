---
id: "KNOW-CeamlTyzSlo2fkExKcrO"
type: "knowledge"
status: "approved"
title: "ソース定位: 複数手法の合議・曖昧性検出・トラッキング"
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

# ソース定位: 複数手法の合議・曖昧性検出・トラッキング

AudioLens のソース定位(`al-locate` クレート)は、ゲームのキャプチャ映像から抽出した音声と比較用入力(素材音声)のアライメントを推定する領域である。単一手法の相関ピークを信用せず、複数の独立手法による合議、同点ピークの構造的検出、長尺素材の窓単位トラッキングという三層の設計で成立している。本書はその設計判断と根拠、および再利用できる教訓をまとめる。

## 4手法と証拠モデル

定位は 4 系統の独立手法が `LocateMethod` trait を介して並行に評価する。`full_ncc`(全区間 NCC)、`band_ncc`(帯域分割 NCC)、`gcc_phat`、そして `spectral_envelope`(スペクトル包絡の逐次内積)である。各手法は `MethodEvidence` —— 候補位置・スコア・ピーク対背景比・ピーク幅・第1/第2ピーク比・validity・棄却理由 —— を返し、受理も棄却も証拠として永続化する。棄却証拠を消さないことで、「合議が何を見て決めたか」を後から検証できる。ピーク対背景比は median |corr| ベースで、DC バイアスを持つ信号は背景推定が歪むため、テスト fixture はゼロ平均ノイズに揃える決まりがある。

## 合議受理と失敗モード

`consensus::evaluate` は、デフォルト `min_agreeing_methods = 2` 以上の手法が `position_tolerance` 内で一致し、かつスコア閾値を超えた場合にのみ受理する。クラスタリングは当初、隣接差だけを見る single-link で、連鎖的に許容差を超えた投票を一つのクラスタに吸収していた。レビューで「クラスタ先頭から `position_tolerance` 以内のみ併合し、全ペア差 ≤ tolerance を保証する」方式に改められた。最大クラスタが採用候補で、第2クラスタも閾値を満たす場合は無言で選ばず `AmbiguousAlignment` として明示棄却する。クラスタの代表位置は下側中央値とする。

失敗モードは validity で表現し、曖昧性もその一種である。golden 行列には「無音・強周期での偽相関ピーク受理」「低コヒーレンス時の optimal-lag 過大推定」が名前付き拒否ケースとして登録されている。

## argmax 曖昧性の構造的検出(Issue #59)

相関曲線の argmax が同点(実質的に区別不能)である場合、手法は受理してはならない。この判定を第一級契約として実装した。判定式は `M = max_i |corr[i]|`、`tie_window = max(TIE_REL_EPS·M, TIE_ABS_FLOOR)`、`T = { i : |corr[i]| >= M − tie_window }` であり、`|T| >= 2` で曖昧とする。サンプル数計数なので plateau を run に潰さず、NaN は全比較 false で T に入らない。`argmax_tie_count` は `pub(crate)` で、eps は外部設定に露出しない private const である。

確定値は `TIE_REL_EPS = 1e-11`、`TIE_ABS_FLOOR = 1e-11`。spec の候補レンジ(相対 1e-9..1e-6、floor 約 1e-9)から意図的に下方へ再較正した決定で、理由は二つある。受理系 fixture の最小マージンが 1.65e-8(`case_loop_material` × `spectral_envelope`)しかなく、1e-9 では 16 倍しか離れず「十分に小さい」と言えなかったこと。もう一つは、scalar↔dispatch の実測偏差が ≤ 約 5e-16 と想定よりはるかに小さかったこと。両側の余裕を最大化する位置へ中央寄りした結果、下側に約 2 万倍、上側に約 1.7×10³ 倍のクリアランスを得た(計測の事実は `docs/evidence/locate-verification.md` に記録)。

合議への組み込みは、既存の3分岐(no-peak → InsufficientSupport、非有限/非正 → LowCorrelation、p2b 不足 → LowCorrelation 候補保持)の後に曖昧分岐を置く。弱さで棄却済みの曲線を曖昧へ格上げしないためである。投票フィルタは `ComputationFailed` と `AmbiguousAlignment` を除外し、全手法が曖昧なら「`all voting methods produced ambiguous (tied) peaks`」の専用文言、合意不足なら `"; N method(s) excluded as ambiguous"` を文言に付加する。`best_window_match` も `argmax_tie_count(&corr) >= 2 → continue` を3段全てに適用する。`merged_votes` は非曖昧の証拠を優先するタプルキーでソートする。

実測で判明した重要な事実として、周期信号の厳密同点は `full_ncc` と `spectral_envelope` でのみ発生する。`band_ncc` は帯域フィルタの円周畳み込みエッジ過渡、`gcc_phat` はスペクトル寄与差で厳密周期が崩れるためである。計画段階の「全4手法が曖昧」という想定は、この「曖昧2件の投票除外 + 非曖昧2件の合意不足」という混在機構へ修正された。

この構造的検出の副産物として、ulp 順序依存の古い問題が解消された。`spectral_envelope` の逐次内積を `dsp::corr0` に置き換えると、周期入力で全 visited lag が厳密 1.0 になり stable-sort argmax が lag 0 に倒れる変更があった。当初はこれが golden を破壊するため残存許容として逐次式を維持していたが、「同点 → 曖昧性として構造検出 → 投票除外」が定着したことで、拒否が argmax 順序の ulp 依存ではなく構造的に維持されるようになり、`corr0` 化を完了した。教訓は、浮動小数の累算順差が同点 argmax のタイブレークを変え、合議の受理/拒否を決定的に変え得るということ。「値の契約上等価」と「振る舞い等価」は別物であり、曖昧性の構造的検出があれば ulp 依存を除去できる。

## トラッキング: anchor・segment・TimeMap

長尺素材は全体を一つのオフセットでは表現できず、source を窓(`anchor_window`/`anchor_hop`/`anchor_gap`、既定 512/512/256)に分割して窓ごとに定位する anchor を生成する。初期実装の anchor は常に source=0 の擬似区間であり、drift/jitter/skip/gap/resume を表現できなかった。レビューで窓 anchor 化され、長尺内のずれが観測可能になった。さらに、検証なしの出現クラスタ使用が偽ピークの anchor 列破壊を招いたため、出現は合議検出位置の Voronoi セルで検証する。検証済み出現が2件以上なら出現ごとのセル内で窓定位しループグリッド上の source 座標で anchor を構成、1件なら capture 全域走査してプレーン座標を取る。

`build_time_map` は gap/rewind で区間分割し、run 内のオフレート窓でさらに分割して `Segment` 列を作る。`SegmentKind` は Continuous/Drift/Jitter/Skip/Gap/Resume/Unmatched。単一 anchor の区間は傾きを観測できないため Continuous とする(以前は無条件 Jitter で、レビュー修正)。連続オフレート窓は同一 rate クラスの最大 run として1区間にまとめ、クラス遷移時のみ共有 anchor を境界に再利用する。これは窓ごとに piece を確定して `w[1]` を再投入し anchor 重複と区間オーバーラップを起こしていた実装の修正である。結果の segment は capture 時系列に単調・非重複である。

ループ反復は `identify_loop_occurrences` が出現ごとにレコード化し、fuzzy match に潰さない。`loop_index` は anchor 起点の符号付き通し番号(anchor より前は負)、`occurrence_position` は検出位置をそのまま保持する(capture 原点グリッドへの吸着はレビューで除去)。

## 混合推定と残差探索

`mixer::fit_mix` は capture ch ごとの source→capture 混合行列を最小二乗で推定する。係数は符号付き(極性反転は負係数)で、リッジ正則化を適用する。条件数はリッジ適用前の正規行列に対して計算する。正則化で病態の診断値を隠さないためで、特異系はリッジ>0 で解き条件数は Infinity のまま記録する。

`residual_search` は受理アライメントの混合を推定・減算し、残差キャプチャで子 locate を再実行する(支配的 BGM の下に別素材を探す用途)。来歴として親ソース・親 alignment id・推定ゲイン・混合行列・減算前後エネルギー・確度を保持し、確度は「除去できたエネルギー比」と「子結果の受理」の平均とする。残差 PCM 自体は当初破棄されていたが(Issue #32 残件)、`locate/residual/<parent>_<artifact_id>.wav` として Float64 で保存するように改められた。残差 child alignment の `capture_pcm` は実際に探索した減算後残差を明示参照し、元キャプチャ normalized_pcm への暗黙 fallback は禁止する。

## 入力正規化と計算基盤

ingest の PCM はソースネイティブスケール(整数 PCM は整数値のまま f64)で、`sample_format` を見て比較前に正規化する。NCC/GCC-PHAT はスケール不変だが、スペクトル包絡と残差エネルギーは絶対スケール依存のためである。

サンプルレート不一致は当初リジェクト方針だったが、「source→capture の基準レートへ polyphase FIR でリサンプル統一」に転換された(Issue #37 系)。線形補間はスペクトルイメージが帯域ゲイン等を汚染するため不可で、Blackman 窓 sinc・unit-DC 係数・群遅延補償を持つリサンプラを al-math に実装した。正規化済み PCM は `locate/normalized/<artifact_id>.f64le` に永続化し、再利用キーは params+元成果物 ID(input ID だけで共用しない)。

BandNcc は入力単位の `InputCache`(入力 ID/ch/正規化レート/信号版/区間/FFT長/帯域境界/フィルタ方式をキー)で bandpass 重複を除去する。並列メソッドには `Arc` 不変データを渡す。長尺ソースは `ResamplePlan` で rate/4 間引きの coarse→fine 探索に分割し、粗候補最大8件を原レート近傍で精密化する。重要な契約(F3)として、最終証拠は全メソッドで元の全探索域から生成する。粗候補の背景比・競合比・幅は最終証拠へ流用せず、fallback 理由(粗候補なし/境界候補/精密候補なし)は記録する。F3 がある限り全探索の相関計算は省略できず、性能改善の中心はフィルタ済み信号再利用になる。

`time_map.json` は accepted pair ごとに job dir へ persist され、`DatasetStage::materialize_locate` が dataset `locate` role へ複写する(AL#191)。src/cap は job manifest の params から取り、欠落は skip ではなく `StageError` で fail-fast する —— manifest 宣言済み artifact の配置欠落はデータ不整合であり、silent drop を避けるためである。

## 再利用できる教訓

- 低情報・棄却は失敗ではなく証拠として永続化する。`accepted` フラグで「受理」と「証拠はあるが棄却」を区別し、棄却結果も消さない。
- レビューで見つかった構造的欠陥(single-link 連鎖、擬似 anchor、無検証クラスタ)はいずれも「局所条件だけで決めていた」ことが原因で、グローバル不変条件(全ペア差 ≤ tolerance、単調・非重複、検証済み出現のみ)を明示して修正された。
- 契約変更の所有権はタスク単位で明示する。部分 ingest 契約・time_map materialize・曖昧性バージョン bump(`version: "2"`、pre-release 方針で互換層なし)はいずれも単一の所有者タスクが持ち、消費側は二重実装しない。
- ドリフト fixture のように手法間位置が許容差を超えてずれる正当ケースは、合議閾値を広げた `--config` オーバーレイ(`locate.consensus.position_tolerance: 512`)で検証する。`ConsensusConfig` に container 級 `#[serde(default)]` を付けた部分オーバーレイ機構がこれを可能にする。
- 残存許容(値の等価だが振る舞いが変わる置換を意図的に残す)は理由をコードコメントと実装記録に残し、構造的解消(曖昧性検出)ができた時点で除去する。

関連する主な Issue/PR: #32、#37、#40、#52、#59、AL#191、PR #49。
