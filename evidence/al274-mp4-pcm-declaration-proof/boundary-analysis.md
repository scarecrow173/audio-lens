# MP4 PCM declaration/capability selective exact proof の検証境界

対象は Issue #274 の MP4-PCM-DECL-006 のうち、固定 scalar sound-entry declaration domain に対する PCM declaration/capability classification と sample-width support である。本書は実装前に確定した境界である。MP4 sample table、PCM byte extraction、endian conversion、payload decode semantics、parser integration 全体、AVI、WAV conversion/writer はこの slice の保証に含めない。

## 1. Property と製品リスク

MP4 の `soun` sample entry は、codec fourcc と declaration field から PCM byte layout を決める。分類を誤ると、compressed/unknown codec を PCM と誤認する、ISO `ipcm`/`fpcm` の endianness や sample size を誤る、`lpcm` の packed/non-interleaved flag を見落とす、zero channels/sample rate を再生能力ありと誤認する、または未対応 bit width を対応済みと誤認する。結果として、partial ingest の capability observation、metadata の `pcm_tracks`、最初に復号する track の選択が変わる。

この slice が所有する Property は次の 2 つである。

1. **MP4-PCM-DECL-006a declaration classification。** fixed scalar `SoundEntry` declaration domain 全体で、production `pcm_layout` が legacy PCM fourcc、ISO `ipcm`/`fpcm` + `pcmC`、`lpcm` flags/defaults、unknown/compressed complement を exact に分類する。
2. **MP4-PCM-DECL-006b capability/sample width。** declaration から得た `PcmLayout` について、production capability helper が zero channels/sample rate と supported sample width を exact に合成し、`pcm_sample_width_value` が supported/unsupported width matrix を exact に返す。

proptest は引き続き executable production oracle/regression lane であり、local proof の代替にも local proof の重複証明にもしない。proof は parser integration、sample table、extraction、conversion、diagnostics、I/O、allocation、payload semantics を証明済みとみなさない。

## 2. exact input domain

declaration proof の入力は production `SoundEntry` の fixed scalar declaration field だけである。

- `format: [u8; 4]` は全 2^32 bit pattern。
- `channels: u16`、`bits: u16`、`sample_rate: u32` は全数値領域。
- `lpcm_flags: Option<u32>` は presence bit と full u32 value の exact reparameterization。
- `pcmc: Option<(u8, u8)>` は presence bit、full u8 flags、full u8 sample size の exact reparameterization。

`Option` の bool 表現は input narrowing ではない。`None`/`Some` の 2 Variant を漏れなく構成し、`Some` の payload は全数値領域を取る。`lpcm` 以外の fourcc で `lpcm_flags` を symbolic に渡すのは、production 到達外または unobserved の total extensionであり、flag 非依存も同じ proof で検査する。`ipcm`/`fpcm` 以外で `pcmc` を symbolic に渡すことも同じである。`kani::assume` は置かない。

capability/width proof は同じ `SoundEntry` domain に加えて、独立した scalar `float: bool` と `bits: u16` で `pcm_sample_width_value` の全 matrix を検査する。`bool` は 2 値の全領域であり、非対応 width を代表値へ狭めない。

proof family は finite-class decomposition ではなく、単一 symbolic scalar proof 2 件を最初に採用する。分岐は fourcc equality、`pcmC` presence、flag bit、width matrix の finite bitvector 条件だけで、loop、`Vec`、I/O、allocation、fmt を含まない。timeout した場合は入力域・assertion・oracle を弱めず、fourcc class の exact decomposition または Required NotRun を evidence に残して判断する。

## 3. 証明する性質

### declaration classification

`pcm_layout` は次を満たすことを証明する。

- exact `sowt` は signed integer little-endian、declaration `bits` を保持する。
- exact `twos` は signed integer big-endian、declaration `bits` を保持する。
- exact `in24` / `in32` は signed integer big-endian、width declarationをそれぞれ 24 / 32 へ固定する。
- exact `fl32` / `fl64` は float signed big-endian、width を 32 / 64 へ固定する。
- exact `raw ` は unsigned integer endian-neutral 8 bit へ固定する。width-1 integer read では endianness は観測不能だが、現行 contract の layout bit は little-endian側に固定する。
- exact `ipcm` は `pcmC` があるときだけ signed integer、`format_flags` bit0 が 1 なら little-endian、0 なら big-endian、`PCM_sample_size` を width へ使う。`pcmC` 不在は unsupported/unknown であり、supported PCM と同一視しない。
- exact `fpcm` は同じ presence/endianness/sample-size規則で float signed とする。
- exact `lpcm` は v2 flags があるときそれを使い、ないとき big-endian signed packed integer default を使う。`PACKED` 不在または `NON_INTERLEAVED` 指定は decodable PCM layout ではない。受理時は `FLOAT`/`SIGNED`/`BIG_ENDIAN` bit と declaration `bits` を保持する。
- 上記以外の compressed/unknown/future fourcc は PCM declaration ではない。valid/unsupported/unknown を混同しない。
- 受理した layout では declaration の channels/sample_rate を変更しない。
- overflow、underflow、panic がない。

独立 oracle は specification level の期待関係を raw symbolic scalar から計算する。production truth を迂回する proof-only implementation ではなく、production `pcm_layout` の結果と独立 oracleを比較する。

### capability と sample width

production `pcm_supported_width` と `pcm_sample_width_value` は次を満たすことを証明する。

- integer width 8/16/24/32 はそれぞれ 1/2/3/4 byte、float width 32/64 はそれぞれ 4/8 byteである。
- 上記以外の integer/float width は unsupported である。`signed` と `big_endian` は width support に影響しない。
- channels または sample_rate が 0 の layout は capability を持たない。
- channels/sample_rate が非 0 でも、unsupported width は capability を持たない。
- capability成立は PCM declaration、非 zero channels/sample rate、supported width の conjunction と一致し、成立時の width は width matrix と一致する。

## 4. production への接続条件

True to the Code の接続条件は次の通りである。

1. production `pcm_layout` が唯一の codec declaration classification seam であり、harness はこれを直接呼ぶ。fourcc/table/classification logic を proof 内だけへ複製して実物判定を迂回しない。
2. production `pcm_sample_width` の error 文言を作る shell は既存の公開挙動を保ち、width matrix 自体は allocation/fmt を持たない `pcm_sample_width_value` に置く。`decode_pcm_bytes` は従来どおり error shell を通る。
3. `pcm_tracks` と first PCM extraction selection に共通する first PCM declaration 規則は、production `first_pcm_declaration` に集約する。これは最初に PCM layout を持つ entry で走査を止める現行規則を変えない。capability成立には production `pcm_supported_width` を使う。
4. extraction shell は no sample table、zero channels/rate、sample table、payload byte extraction、mid-sample/mid-frame、decode conversion の既存 error precedence を変えない。本 proof はその precedence を証明しない。
5. `cfg(kani)` による製品置換、fmt/alloc/IO stub、proof専用の不自然な抽象化、任意の `kani::assume` input narrowing は使わない。unwinding assertions は有効のままとし、loop bound を外さない。

## 5. 検証しない領域

この slice の local proof は次を証明しない。

- sound-entry box parser completeness、`stsd` entry ordering、child box scan、byte offset extraction。
- MP4 sample-table accumulation（`stts`/`ctts`/`stsc`/`stsz`/`stco`）、chunk map、sample extraction。
- PCM byte endian conversion、integer/float decode semantics、payload conversion、mid-sample/mid-frame data validation。
- compressed codec の内部 semantics、partial ingest の integrity observation precedence、diagnostic 文言。
- I/O、allocation、`Vec`、公開 API 全体 no-panic、writer round-trip、mutation/truncated parser integration。
- AVI、WAV conversion/writer、MP4-TABLE-005、MP4-PCM-EXTRACT-007。

これらは residual NotRun scope として ledger と PR に残す。

## 6. runtime/proptest 接続

既存 MP4/container proptest は弱めない。`sowt`/`twos`/`raw `/`lpcm`/`ipcm`、compressed fourcc、`pcmC` 不在、unpacked `lpcm`、unsupported width、zero channels/rate、mid-frame を既存 generator が覆う。proof は exact scalar classification/capabilityを担い、proptest は production integration を担う。production seam の接続が不足する最小例だけを `crates/al-media/src/mp4.rs` の deterministic unit test に追加し、lease 外 test file は変更しない。

## 7. runner と予算

runner に `mp4-pcm-declaration` group を追加し、`mp4_pcm_declaration_classification_exact` と `mp4_pcm_capability_width_exact` を漏れなく選択する。既存 env-detect、WSL mirror、per-harness sequential execution、timeout semantics、elapsed ms、raw log、`/usr/bin/time -v` max RSS、verdict schema、fail-closed、group/subset 排他、既定全件挙動は変えない。

`AL_KANI_TIMEOUT_MS=600000` を canonical budget とし、1 proof 10 分以内・ピーク RSS 4 GiB 未満を設計予算とする。timeout/Fail の場合は、solver 設定だけで帳尻を合わせず、exact decomposition または Required NotRun/Fail の事実を記録する。legacy timeout proof を含む全件 lane はこの slice の必須検証にしない。

## 8. mutation plan

Able to Fail 証拠として、少なくとも 1 つの semantic mutation を入れる。第一候補は ISO `ipcm` endianness bit の反転、または integer 24-bit widthを 2 byte へ変える width matrix mutationである。期待は `mp4-pcm-declaration` lane が exit 1 / verdict `fail` / 対象 harness `VERIFICATION:- FAILED` になることである。diagnostic-only mutation、runner error、harness compile error、別 proofだけの失敗は検出証拠にしない。確認後は保存した byte-for-byte copy で復元し、hash が一致することを確認して post-revert canonical lane を再実行する。

## 9. RF 還元判定

PCM declaration/capability contract は汎用化候補である。ただし現行契約には AudioLens の partial ingest、first PCM declaration、capability observation precedence、pre-normalization PCM contract が結び付く。本 slice では Ruviform 側ファイルや Issue を変更せず、候補と理由を PR 本文へ記録する。ruviform#1015 の重複起票はしない。
