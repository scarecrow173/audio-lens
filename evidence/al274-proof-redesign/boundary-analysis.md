# WAV-RANGE-008 exact valid-range proof boundary analysis

対象は Issue #274 の WAV-RANGE-008 に対応する range arithmetic である。本書は実装と solver 実験に先立つ境界設計であり、PR #281 の failed evidence を read-only 入力として使う。PR #281 の valid proof `wav_range_plan_valid_ranges_exact` は 300012ms timeout で Required NotRun、invalid proof `wav_range_plan_invalid_ranges_error` は Pass、mutation detection は diagnostic payload mutation で確認された。ここでは invalid proof と mutation detection の成果を保持しつつ、exact valid-range proof が Required Pass になる境界と分解を決める。

## 1. exact input domain

検証する入力は production `WavDataLayout` と public frame index である。`data_offset`、`data_len`、frame index は u64、`block_align` は production 表現どおり u16、RIFF data length は u32 境界を持つ。

constructor-admitted layout は次の exact invariant を満たす。

- `1 <= block_align <= u16::MAX`。
- `0 <= data_len <= u32::MAX`。
- `data_len % block_align == 0`。
- `total_frames = data_len / block_align`。
- `data_offset + data_len <= u32::MAX + 8`。右辺は RIFF size field が u32 であり、file length が RIFF header 8byte を足した長さであることに由来する。
- 上記の除算・比較・加算は数学上の値として読む。production constructor は frame 境界でない data length、zero `block_align`、RIFF boundary に収まらない extent を受理しない。

public frame index `start_frame`・`end_frame` は u64 全域である。valid predicate は `start_frame < end_frame && end_frame <= total_frames`、invalid predicate は exact complement の `start_frame >= end_frame || end_frame > total_frames` とする。valid 側では `total_frames <= u32::MAX` なので、valid range を満たす `start_frame`・`end_frame` は必ず u32 に収まる。したがって valid proof で zero-extension した u32 start/end を使うのは exact reparameterization であり、入力領域の narrowing ではない。

u8 fixture、8 frame、representative channel count、small rate、supported encoding への narrowing は採用しない。channel count、sample rate、encoding は range arithmetic の入力ではなく、layout invariant へ影響しない。

## 2. 証明する性質

valid proof family は、constructor-admitted layout 全域と valid predicate を満たす全 public frame index を対象に次を証明する。

1. valid range が range plan として受理される。
2. `total_frames` が layout の frame count と一致する。
3. `frame_count = end_frame - start_frame`。
4. byte offset が frame 単位の開始位置 `data_offset + start_frame * block_align` に一致する。
5. byte length が `[start_frame, end_frame)` の byte 幅に一致する。endpoint product form を使う場合は `end_frame * block_align - start_frame * block_align` が byte length の定義であり、count product との一致を別補題または assertion で検査する。
6. exclusive byte end が byte offset + byte length に一致し、`data_offset + end_frame * block_align` に一致する。
7. slice の exclusive byte end が data extent `data_offset + data_len` に収まる。
8. overflow、underflow、panic、unwinding assertion failure がない。

invalid proof は、exact complement を満たす全 u64 start/end が必ず拒否されること、内部 error payload の `total_frames` が layout の frame count に一致することを証明する。diagnostic formatting 自体は proof に含めないが、`MediaError` へ写像する前の compact payload までは formal proof の対象に含める。

## 3. 検証しない領域

この local proof は次を証明しない。

- seek/read/short read と I/O error 系列。
- `Vec` allocation と allocation failure。
- PCM decode、sample conversion、f64 精度。
- diagnostic formatting と人間向け error message の内容。
- 公開 API 全体 no-panic。
- chunk walking、scan progress、RIFF signature、writer round-trip。

これらは未証明領域として ledger に残す。local proof Pass を公開 API 全体 no-panic や decode correctness へ転用しない。proptest `frame_count_and_range_reads_hold_the_contract` と `writer_output_round_trips_losslessly` は integration coverage として維持し、formal proof の代替にも formal proof の重複証拠にもしない。

## 4. production への接続条件

True to the Code の接続条件は次の通りである。

1. production `read_wav_range` が range validation、frame count、byte offset、byte length、exclusive byte end を同じ production pure helper から得る。
2. `wav_layout` が `WavDataLayout::new` を通し、zero `block_align`、frame 境界でない data length、u32/RIFF boundary を越える data extent を拒否する。
3. `wav_frame_count` が同じ `WavDataLayout.total_frames` を返す。
4. helper 事後条件の byte offset/byte length から、production `read_wav_range` が seek offset と read length を組み立てる。
5. helper は I/O、`Vec` allocation、PCM decode、`std::fmt` 診断文字列構築を実行しない。内部 error は compact payload に限定し、呼出側で既存 `MediaError` 文言へ写像する。

test-side model だけを証明して完了とはしない。proof は production helper を直接呼び、symbolic layout は constructor-admitted layout の exact invariant で構成する。constructor 自体の enforcing は concrete unit test と production 接続 review で確認し、proof の assume は exact constructor invariant に限定する。

## 5. unsupported/malformed format の扱い

range arithmetic が意味を持つのは constructor-admitted layout 全体である。decode 可能な WAVEFORMATEX layout へ絞らない。

理由は production の責務分割にある。`wav_frame_count` は layout だけを見て frame count を返し、unsupported encoding でも data length と block align が整っていれば count を返しうる。`read_wav_range` も range validation を decode 前に行い、invalid range は unsupported encoding より先に拒否される。この error precedence を変えない。したがって unsupported encoding の range plan は成功 slice の製品保証に直接寄与しないが、range arithmetic の入力領域から unsupported encoding を除外する理由にもならない。proof boundary は constructor-admitted layout 全体に置き、decode 可能性は上位 integration contract に残す。

## 6. assume と coverage の分類

proof family の assume は次の三種に限定する。

| 種別 | assume | coverage 根拠 |
| --- | --- | --- |
| exact constructor invariant | `block_align != 0`、`data_len <= u32::MAX`、frame alignment、data extent boundary | production constructor が enforcing する領域と一致する |
| exact branch selection | valid では `start < end && end <= total_frames`、invalid ではその否定 | 二 predicate は exact complement であり union は全 u64 start/end |
| exact reparameterization / entailed bound | valid start/end の zero-extension u32 表現、`data_len = total_frames * block_align` の constructive 表現、block bit width partition に伴う total frames の上界 | いずれも constructor invariant と valid predicate から論理的に導かれる。代表値 sampling ではない |

constructive layout 表現は、frame-aligned data length を quotient `total_frames` と `block_align` から作る。任意の constructor-admitted layout は `total_frames = data_len / block_align` を取れば同じ形で表せるため、coverage を落とさない。division inverse を solver に一度に解かせず、proof 対象を range arithmetic に局所化するための exact reparameterization である。

## 7. decomposition 候補の比較

### constructor-faithful count form + constructive range decomposition(採用)

`total_frames = data_len / block_align` という constructor invariant を保ち、production の byte length を `frame_count * block_align` で計算する。valid input は `start32`、正の `frame_count32`、`tail_frames32` で構成し、`end = start + frame_count`、`total = end + tail` とする。data length は `start`、`frame_count`、`tail` の 3 つの byte segment の和として表す。

この表現は exact である。任意の constructor-admitted layout と valid range について、`frame_count = end-start`、`tail = total-end` を取れば同じ形になり、整数の distributivity により `data_len = total * block_align` は 3 segment の和に一致する。widen した u64 では各 segment product の和が 2^64 を越えないため、有限幅の wrap に依存しない。逆に、構成した tuple は常に valid range と frame-aligned data length を与える。coverage を落とす代表値 sampling ではない。

この分解を選ぶ理由は、production correctness のうち byte length と containment を solver の nonlinear monotonicity ではなく linear segment equality `byte_end + tail_bytes = data_offset + data_len` で検査できることである。multiplication は production helper と oracle の双方に残るが、証明 query は同じ frame count・start frame・block align を使う exact equality に局所化される。

### constructor-faithful division/count form 単独

symbolic data length からの division inverse と count product containment を同じ proof に置く形である。production semanticsに近いが、PR #281 の初期形と同じく solver が nonlinear arithmetic を一度に扱う。constructive decomposition を伴わない単独案は採用しない。

### endpoint form + distributivity/containment lemma

endpoint product difference `end * block - start * block` を byte length に使う PR #281 final 形である。pure helper 構造と invalid proof の成果は再利用できるが、byte length の underflow 不存在と data containment が multiplication monotonicity に依存する。今回の b16 partition 実験でも CaDiCaL は 2分44秒時点で solver grind を続けた。wide/portion partition の総当たりより、count form と constructive decomposition へ boundary を変えるほうが保証を弱めずに小さい query へ分解できる。採用しない。

### byte-oriented validation algebra

byte offset/byte length/end を直接検査し、frame-domain validation との equivalence を別補題に分ける。public 意味論を変えずに実装だけ byte-oriented へ寄せると、受入条件と算術の対応が読みにくい。containment を runtime check でごまかす形にすると、valid range の acceptance proof が同じ multiplication monotonicity を必要とする。採用しない。

### exact finite partition

`block_align` u16 の bit width `[2^(n-1), 2^n - 1]` で 16 分割する案は union coverage を保つ。ただし endpoint monotonicity が残る限り、各 partition の nonlinear query がなお支配因になる。constructive decomposition は partitioning なしで exact coverage を保てるため、finite partition は今回の採用案ではない。将来ほかの arithmetic property で使う場合は、disjointness/union coverage を同じ形式で ledger に書く。

### function contract / verified stub decomposition

multiplication monotonicity、distributivity、containment を独立 contract にする案は、runner へ最小限の unstable flag 追加と proof-for-contract registry を要する。今回は contract なしで exact proof が実用予算に入ったため採用しない。assert 後 assume の local lemma も使わない。未証明の数学的事実を assume していない。

## 8. 採用する境界と runner 戦略

最終採用案は constructor-faithful count product form の production pure helper と、valid input の start/frame_count/tail constructive decomposition である。valid proof `wav_range_plan_valid_ranges_exact` は 1 件で constructor-admitted layout 全域と valid public u64 frame index をカバーする。invalid proof `wav_range_plan_invalid_ranges_error` は exact complement を full u64 start/end で検査する。二つの range predicate は exact complement である。

proof は symbolic scalar と production helper だけを対象にし、writer fixture、`Cursor`、`Read+Seek`、`Vec` decode、fmt/alloc stub を入れない。runner には wav-range assurance group を追加し、この 2 proof を漏れなく選択できるようにする。per-harness elapsed ms、raw log、pass/fail/not_run 判定、`/usr/bin/time -v` が利用できる環境での max RSS を残す。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論は変えない。

legacy whole-call proof `header_reads_valid_ranges_slice_exact` / `header_reads_invalid_ranges_error` は、exact valid replacement を含む wav-range Required proof 全体が Pass した後にだけ retire する。全件 Kani lane はこの slice の必須検証にしない。

bounded input への縮小、lazy diagnostic、fmt/alloc stub、`cfg(kani)` による製品実装置換は採用しない。solver 属性は使わず、Kani/CBMC の default CaDiCaL を使う。solver の総当たりは目的にしない。

## 9. 設計予算と失敗時の扱い

目標は wav-range assurance group の選択実行合計 10〜20 分、単一 proof timeout 300000ms 以内、各 proof の peak memory 4GiB 未満である。peak memory は `/usr/bin/time -v` が利用できる環境で systematic に記録する。ただしこの値は runner が起動したコマンドに対する GNU time の max RSS であり、個別 solver process だけを分離した cgroup 計測ではない。その限界を ledger に書く。

Required valid proof が timeout または Fail なら、proof を弱めず legacy proof を retire せず Required NotRun/Fail を正直に報告する。decode 可能 layout への縮小、bounds による unbounded/exact proof 読み替え、timeout の Pass 化はしない。
