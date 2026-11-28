---
id: "KNOW-7xWs8dSzCMG493oYPAwa8I"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "lesson"
title: "Media Parser 形式的検証: Kani 境界設計・solver 支配要因・stub/mutation の知見"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/256"
    - "https://github.com/scarecrow173/audio-lens/issues/274"
    - "https://github.com/scarecrow173/audio-lens/pull/275"
    - "https://github.com/scarecrow173/audio-lens/pull/276"
    - "https://github.com/scarecrow173/audio-lens/pull/281"
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

# Media Parser 形式的検証の知見

本書は、AL#256 で導入された Media Parser の Kani/proptest 構成を、AL#274 で Property 起点の assurance pack へ再設計した過程で得られた知見をまとめる。証跡の正本は `evidence/al256/`、`evidence/al274/`、`evidence/al274-boundary/`、および各 proof slice の `evidence/al274-*-proof/` である。本書はそれらの内容を要約して自己完結させる。

## 基本原則: verdict と requirement を混ぜない

最も重要な運用知見は、Pass/Fail/NotRun/Unsupported と Required/Deferred/NotApplicable を別軸で扱うことである。

- Kani が timeout した proof は Fail ではないが Pass でもない。Required assurance は NotRun のまま残す。
- Kani 環境がない環境で lane が緑でも Unsupported であり、proof passではない。
- runner や CI job の exit 0 は orchestration の成功であり、個別 proof の `VERIFICATION:- SUCCESSFUL` と混同しない。
- CI green は統合レーンの成立を示すだけで、タイムアウトした symbolic proof の成立を意味しない。
- Property Tested は sampled assurance であり、Bounded Exhaustive local guarantee と同一視しない。

この区別がないと、「検証したい」という意図が「検証できた」という事実へすり替わる。AL#274 の再設計では、保証範囲と未証明領域を ledger に残すことを優先した。

## AL#256 の初期構成と限界

AL#256 では、WAV codec と MP4/AVI container を対象に、Kani proof、proptest、runner、証跡を導入した。当初の Kani 対象は、公開 API や writer output decode を比較的大きな単位で symbolic に呼ぶ whole-call/end-to-end proof が中心だった。

この構成では input-domain split によって signed/unsigned 入口や WAV/MP4/AVI の対象を分けても、多くの proof が実用予算内で verdict を返さなかった。baseline 時点の canonical 21 proof では、`detect_format_signature_rules` のみが Pass で、残り 20 件は 1500 秒/proof の canonical budget に到達した。1500 秒到達は proof の実効コストではなく timeout 到達である。

旧構成の限界は次である。

- whole-call proof は parser 全体の分岐、slice、allocation、panic machinery、formatting を同時に solver へ渡す。
- symbolic byte 列 48/64 byte でも、API 全体の制御フローを通すと solver state が急増する。
- writer output decode の symbolic round-trip は product contract として有用だが、Kani の bounded bitvector solving には重い。
- timeout を繰り返す proof は Required assurance として機能しない。実行可能でない証明は保証を与えない。

## PR #275: proof 分割だけでは不十分だった

PR #275 は旧 combined proof を入口と signature class 単位に分割し、対象を小さくする方向を取った。分割は timeout の観測と再現性の改善には有効だったが、whole-call symbolic proof という性質自体は残った。

得られた教訓は、分割の単位が「API や入力 class」では不十分で、「証明したい数学的性質」でなければならないことである。例えば `decode_wav` 全体を入力種別で分けても、chunk boundary、scan progress、range arithmetic、PCM conversion、diagnostic formatting が同一 proof に残る。これらは必要な oracle と支配要因が異なるため、Property 単位へ再構成する必要がある。

## PR #276: fmt/allocator stub ablation

PR #276 では、diagnostic formatting と allocation error path の stub が solver 到達性へ与える影響を測定した。知見は次である。

- `std::fmt::format` は diagnostic メッセージ構築のために展開され、memchr/PadAdapter などの経路を solver state に持ち込む。
- `fmt_format_empty` stub は、Result や error reason 文字列を oracle が検査しない proof では到達性を改善する。
- ただし、この stub は formatting 自体の panic、allocation、message 長依存の挙動を証明対象から外す。したがって無条件の等価性は主張できない。
- WAV の 2 件の no-panic harness で採用されたのは、proof が diagnostic 文字列を検査しない場合に限る条件付き保証である。canonical な verdict には至らず、保証は依然として NotRun/timeout の whole-call proof に留まる。
- allocation failure stub は、メモリ確保失敗の semantics を変えてしまうため却下された。mp4 writer fmt stub も採用されなかった。

教訓は、stub は「速くするための便宜」ではなく、証明範囲を変更する仕様操作として扱うことである。採用時には、置換が oracle と無関係である理由、除外した範囲、実用的効果を明記しなければならない。

## PR #281: WAV-RANGE-008 の solver 実験

open draft PR #281 は main に統合されていないが、WAV-RANGE-008 の range arithmetic を local proof 化する最初の実験として重要である。この PR は valid proof が 300 秒 budget 内に完走しなかったため Required NotRun のまま draft とされた。

実験では、production `read_wav_range` が呼ぶ pure arithmetic helper を導入し、valid/invalid exact complement を検査した。invalid proof は数秒で Pass し、`total_frames` を壊す mutation を exit 1 / `VERIFICATION:- FAILED` で検出した。一方 valid proof は 300012 ms で timeoutした。

試されたものは次である。

- native multiplication のままの検査。
- constructive reparameterization。
- endpoint subtraction。
- shift/add linear expansion。
- branchless mask。
- assert/assume による local lemma decomposition。
- CaDiCaL / Kissat / Bitwuzla / Z3 の solver 比較。

これらでも exact u32×u16 multiplication monotonicity/containment は 300 秒 budget 内に完走しなかった。原因は製品欠陥の観測ではなく、bitvector solver における非線形乗算と広い整数領域の組合せが支配要因だったと整理されている。peak memory の systematic な証拠は得られず、timeout 中の RSS 観測を proof 完了コストとして扱わなかった。

教訓は次である。

- 「valid direction」と「invalid direction」は同じ Property でも solver cost が大きく異なる。
- invalid の mutation 検出ができても、valid exact proof が timeout なら Required assurance は成立しない。
- solver 変更や lemma 分解は戦術であり、Property 境界の設計とは別である。timeout が続く場合は、入力領域の意味的分解や独立 oracle の形を再設計する。
- timeout を長くするだけでは Required proof にならない。実用予算内で完了する形が assurance の一部である。

## main 統合済み proof slice 群の知見

PR #282 以降、WAV range、chunk scan、format signature、MP4 box/scan、AVI chunk/movi/stream/strf/wb/timing arithmetic の local proof が段階的に main へ統合された。現行 main `886828500b85452774af2be7114437ebb226976b` の registry には 40 件の canonical harness があり、baseline `c91b8ce20c90733fa21098451cb768a74425884d` 以降に追加された local proof は 21 件である。`header_reads_valid_ranges_slice_exact` と `header_reads_invalid_ranges_error` は WAV-RANGE-008 の exact replacement へ置き換えられて退役した。

主な local proof の最新の非 mutation pass evidence は、概ね次の範囲にある。

- 多くの固定長分類・header extraction proof: 約 5〜21 秒、peak RSS 約 250〜500 MiB。
- scan/collection/composition proof: 約 8〜25 秒、peak RSS 約 300〜500 MiB。
- `avi_audio_wb_concatenation_bounded_exact`: 約 40 秒、peak RSS 約 823 MiB。

これらは予算内で完了しているが、公開 API 全体 no-panic の証明ではない。各 proof は宣言した symbolic scalar、固定長 prefix、または小さい有限列の範囲で成立する local guarantee である。

### production helper と expected model

有効だったのは、production pure helper または production decision step を直接呼び、独立した expected model/oracle と比較する形である。例えば、stream kind classification、strl entry classification、strf capture ordering、movi scan collection は、production の分類・選択・停止 semantics と独立 oracle の一致を検査する。

この形には 2 つの利点がある。

- proof が production code に対して True to the Code を保つ。
- product 契約が変わったとき、harness の copy ではなく production helper と expected model の差として検出できる。

一方、expected model 自身が production と同じ誤りを写す危険がある。そのため、product contract を日本語/表形式で先に固定し、mutation による検出能力を確認する必要がある。

### small scan model と Bound

WAV/MP4 scan、AVI movi collection、strh presence、strf capture、wb concatenation では、任意長ではなく 3 要素程度の small model を使う。これは証明の弱さではなく、bounded exactness の明示的な境界である。

small modelで証明できるのは、step semantics、strict progress、first error stop、ordering、collect/skip/stop の組合せである。証明できないのは、無制限 iteration termination、4件以上の列、実 `Vec` の任意長 accumulation、深い nested walker である。この残りを未証明として残すことが、bounded proof を公開 API 全体へ誤用しない鍵である。

### AVI の partial proof は Property 全体 Pass にしない

AVI では、tag、entry decision、scan collection、stream kind、strh declaration、strh presence、video/audio strf declaration、strf capture、wb concatenation、start tick の local proof が個別に Pass している。しかし、AVI-STREAM-002、AVI-MOVI-003、AVI-TIME-004、AVI-PCM-005 の Property 全体はまだ Pass としない。

理由は、残っている未証明領域が Property の核心を含むからである。

- AVI-MOVI-003: body boundary/lifetime composition、任意長 scan、実 Vec accumulation。
- AVI-STREAM-002: 任意長 nested walker、複数 strl、完全な declaration correspondence。
- AVI-TIME-004: remainder、packet increment/accumulation、stream separation。
- AVI-PCM-005: PCM conversion semantics、任意長 copy、dynamic Vec。

個別 proof の Pass は積み上がった証拠として記録するが、Property 単位の verdict は残りの必要条件が揃うまで NotRun を維持する。

## Mutation による検出能力

新しい local proof では、意図的 mutation に対する検出 evidence が繰り返し確認された。代表的な結果は次である。

- WAV range invalid proof: `total_frames` を壊す mutationを `VERIFICATION:- FAILED` で検出。
- WAV chunk boundary/scan: padding/scan progression mutation を検出。ただし mutation の対象によっては片方の proof だけが検出する。
- MP4 box header/scan、AVI chunk: 対応する boundary mutationを検出。
- AVI movi tag/entry/scan、stream kind、strh/strf declaration/capture/presence、wb concatenation、stream start tick: 対象 mutation を検出。
- 一部 mutation では無関係な proof が Pass し、mutation の意味が限定的であることが分かる。これは失敗ではなく、mutation が「どの oracle を壊したか」を明確にする材料である。

mutation evidence が重要なのは、proof が単に何も検査していない状態で緑になることを防ぐためである。特に exact complement、first error stop、ordering、copy exactness は、mutation で oracle が反応することを確認しないと保証の検出能力を主張できない。

## spike harness と正式 proof の分離

`spike_` 12 件は runner registryの canonical 40 件に含まれない。spike は ablation、solver 実験、no-stub baseline、stub 組合せの比較に使う。正式 proof と同じ表に混ぜると、採用済み保証と実験結果の区別が失われる。

分離の運用は次である。

- canonical: assurance pack または legacy inventory として verdict/cost を管理する。
- spike: 実験条件、採用/却下、理由を記録する。
- proptest: sampled assurance の baseline として継続実行する。

## 境界設計の判断理由

AL#274 で採用された方向は、whole-call proof を直ちに捨てるのではなく、Required assurance を Property 単位の local proof へ移し、legacy proof を inventory として残すことである。

理由は次である。

- 公開 API の symbolic no-panic proof は、実用予算内に完走しないため Required assurance として機能しない。
- しかし、timeout evidence は過去の試行と支配要因を示す記録として価値がある。
- round-trip/corruption/partial ingest は proptest が適しており、Kani に置き換える必要はない。
- Kani が強いのは、純粋算術、境界、分類、短い合成の exact complement である。
- local proof が増えるほど、保証範囲・assumption・Bound・residual risk の ledger が必要になる。

## コスト比較の現状と未解決事項

旧 proof の多くは 600〜1500 秒級の timeout で verdict を返さなかった。新しい local proof は数秒〜数十秒で Pass している。これは大幅な実効コスト改善であるが、厳密な旧新比較には注意が必要である。

- 旧 timeout の wall clock は proof 完了コストではなく timeout 到達である。
- canonical 1500 秒/proof と ablation 600 秒 cap は条件が異なる。
- runner 環境、Kani/CBMC version、入力領域、proof の対象が異なる。
- 一部旧 proof の peak memory は欠測または観測値であり、systematic な同条件比較ではない。

したがって、現時点で確実に言えるのは「新しい local proof は実用予算内で verdict を返す」「旧 whole-call proof は Required assurance として機能しなかった」までである。完全な実行時間・ピークメモリ・再実行コスト比較は後続作業である。

## 今後に残る論点

- PR #281 で観測された valid proof timeout のような非線形 bitvector 算術に対し、Property 分解・lemma・solver のどれで解くか。
- MP4 sample table、MP4 PCM extraction、WAV PCM/layout/writer、AVI timing accumulation、AVI PCM decode の Required proof 実装。
- 任意長 scan/iteration、nested walker、dynamic Vec/copy を small model からどう拡張するか。
- Lean 等の強い手法の導入可否。対象は任意長・任意回数の算術不変条件に限定して評価する。
- legacy whole-call proof を registry から物理削除する時期。代替 assurance と requirement state の整理が先である。
- 旧 `docs/specs/formal-verification.md` との最終統合。

## 実務上のチェックリスト

新しい proof を追加するときは、次を先に書く。

1. Property と製品契約。
2. production helper と独立 oracle。
3. 入力領域、assume、unwind bound、stub。
4. 証明できない範囲と proptest への接続。
5. 実行予算と支配要因の仮説。
6. mutation で検出できるべき反例。
7. spike と canonical の区別。

この順序を逆にして harness 実装から始めると、証明対象が API 全体へ膨らみ、timeout と保証範囲の曖昧さが再発する。
