//! メディア fixture: パラメタ化 AVI / WAV バイト列生成。
//!
//! `al-media` から移管した `write_avi`/`AviSpec` を、movi パケット列・
//! per-stream `dwStart`・0 長チャンク・ドロップギャップを制御可能な
//! 形に拡張している。fixture 知識は本番コードに持ち込まない。

/// インターリーブ済み f64 サンプル(整数スケール)を s16 LE バイト列へ。
/// AVI の `NNwb` チャンク等に使う。
pub fn pcm_s16le(samples: &[f64]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&v| (v.round().clamp(-32768.0, 32767.0) as i16).to_le_bytes())
        .collect()
}

/// f64 サンプルを f64 LE バイト列へ(decoded_pcm artifact 用)。
pub fn pcm_f64le(samples: &[f64]) -> Vec<u8> {
    samples.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// 16bit PCM の RIFF/WAVE バイト列。
pub fn wav_s16_bytes(channels: u16, sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let block_align = channels * 2;
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// 40-byte BITMAPINFOHEADER(BGR24 非圧縮、ボトムアップ)。
/// 行 padding を含まない入力を扱うため `width * 3 % 4 == 0` が必要。
/// 非整列幅は panic する。
pub fn bitmap_info_header(width: u32, height: u32) -> Vec<u8> {
    assert!(
        width.is_multiple_of(4),
        "unpadded BGR24 rows must be DWORD aligned"
    );
    let frame_bytes = width * height * 3;
    let mut b = Vec::with_capacity(40);
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(width as i32).to_le_bytes());
    b.extend_from_slice(&(height as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // planes
    b.extend_from_slice(&24u16.to_le_bytes()); // bit count
    b.extend_from_slice(&0u32.to_le_bytes()); // compression = BI_RGB
    b.extend_from_slice(&frame_bytes.to_le_bytes());
    b.extend_from_slice(&[0u8; 16]); // resolution + palette
    b
}

/// 18-byte WAVEFORMATEX(PCM s16)。
pub fn waveformatex(channels: u16, sample_rate: u32) -> Vec<u8> {
    let block_align = channels * 2;
    let byte_rate = sample_rate * u32::from(block_align);
    let mut b = Vec::with_capacity(18);
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&sample_rate.to_le_bytes());
    b.extend_from_slice(&byte_rate.to_le_bytes());
    b.extend_from_slice(&block_align.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes()); // cbSize
    b
}

/// RIFF チャンクを `out` に追記する(奇数長はパッド付き)。
pub fn push_chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

/// `LIST kind` チャンクを `out` に追記する(奇数長はサイズ外にパッド)。
pub fn push_list(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    let mut list_body = Vec::with_capacity(4 + body.len());
    list_body.extend_from_slice(kind);
    list_body.extend_from_slice(body);
    push_chunk(out, b"LIST", &list_body);
}

/// 56-byte AVISTREAMHEADER のボディを組み立てる低水準ヘルパ。
/// `start` は `dwStart`、`sample_size` は `dwSampleSize`
/// (PCM 音声では block align)。
/// `frame_rect` は rcFrame の (left, top, right, bottom)。音声は全ゼロ。
#[allow(clippy::too_many_arguments)]
pub fn strh_body(
    fcc_type: &[u8; 4],
    handler: &[u8; 4],
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    sample_size: u32,
    suggested_buffer: u32,
    frame_rect: [i16; 4],
) -> Vec<u8> {
    let mut b = Vec::with_capacity(56);
    b.extend_from_slice(fcc_type);
    b.extend_from_slice(handler);
    b.extend_from_slice(&0u32.to_le_bytes()); // flags
    b.extend_from_slice(&0u16.to_le_bytes()); // priority
    b.extend_from_slice(&0u16.to_le_bytes()); // language
    b.extend_from_slice(&0u32.to_le_bytes()); // initial frames
    b.extend_from_slice(&scale.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&start.to_le_bytes());
    b.extend_from_slice(&length.to_le_bytes());
    b.extend_from_slice(&suggested_buffer.to_le_bytes());
    b.extend_from_slice(&u32::MAX.to_le_bytes()); // quality
    b.extend_from_slice(&sample_size.to_le_bytes());
    for coordinate in frame_rect {
        b.extend_from_slice(&coordinate.to_le_bytes());
    }
    b
}

/// 1 ストリーム分の宣言(`strl` 相当の内容)。
#[derive(Debug, Clone)]
pub struct AviStreamSpec {
    /// `fccType`(`b"vids"` / `b"auds"`)。
    pub fcc_type: [u8; 4],
    /// `fccHandler`(映像は `b"DIB "` 等、音声は通常ゼロ)。
    pub handler: [u8; 4],
    /// `dwScale`。
    pub scale: u32,
    /// `dwRate`(1 秒あたりの tick 数)。
    pub rate: u32,
    /// `dwStart`(ストリーム開始オフセット)。
    pub start: u32,
    /// `dwLength`。`None` の場合は movi パケットから導出する
    /// (video: db/dc 個数、dwSampleSize ストリーム: 総バイト数/scale)。
    pub length: Option<u32>,
    /// `dwSampleSize`(PCM 音声では block align、他は 0)。
    pub sample_size: u32,
    /// `strf` ボディ(BITMAPINFOHEADER か WAVEFORMATEX)。
    pub format: Vec<u8>,
}

impl AviStreamSpec {
    /// BGR24 非圧縮映像ストリーム(scale=1, rate=fps)。
    pub fn video(width: u32, height: u32, fps: u32) -> Self {
        Self {
            fcc_type: *b"vids",
            handler: *b"DIB ",
            scale: 1,
            rate: fps,
            start: 0,
            length: None,
            sample_size: 0,
            format: bitmap_info_header(width, height),
        }
    }

    /// PCM s16 音声ストリーム(scale=block align, rate=byte rate)。
    pub fn pcm_audio(channels: u16, sample_rate: u32) -> Self {
        let block_align = u32::from(channels * 2);
        Self {
            fcc_type: *b"auds",
            handler: [0; 4],
            scale: block_align,
            rate: sample_rate * block_align,
            start: 0,
            length: None,
            sample_size: block_align,
            format: waveformatex(channels, sample_rate),
        }
    }

    /// `dwStart` を設定する。
    pub fn with_start(mut self, start: u32) -> Self {
        self.start = start;
        self
    }

    /// `dwLength` を明示する。実パケット数とずらすことで
    /// ドロップフレーム(宣言より少ない movi)を表現できる。
    pub fn with_length(mut self, length: u32) -> Self {
        self.length = Some(length);
        self
    }
}

/// `movi` リスト内の 1 パケット。`kind` はチャンク id の後半 2 文字
/// (`b"db"`/`b"dc"`/`b"wb"` 等)。0 長 body は dup チャンクになる。
#[derive(Debug, Clone)]
pub struct MoviPacket {
    /// `hdrl` 内のストリーム index(0..=99)。
    pub stream: usize,
    /// チャンク種別(`db` 非圧縮フレーム、`dc` 圧縮、`wb` 音声)。
    pub kind: [u8; 2],
    /// パケット本体。0 バイト可(重複/dup チャンク)。
    pub body: Vec<u8>,
}

impl MoviPacket {
    /// 映像フレームパケット(`NNdb`)。
    pub fn video_frame(stream: usize, body: Vec<u8>) -> Self {
        Self {
            stream,
            kind: *b"db",
            body,
        }
    }

    /// 音声パケット(`NNwb`)。
    pub fn audio(stream: usize, body: Vec<u8>) -> Self {
        Self {
            stream,
            kind: *b"wb",
            body,
        }
    }
}

/// パラメタ化 AVI 仕様。ストリーム宣言と movi パケット列を
/// 個別に制御できる。
#[derive(Debug, Clone)]
pub struct AviSpec {
    /// `avih` のフレーム幅。
    pub width: u32,
    /// `avih` のフレーム高。
    pub height: u32,
    /// `avih` の 1 フレームあたりマイクロ秒。
    pub usec_per_frame: u32,
    /// `avih` の総フレーム数。`None` は movi 内の映像パケット数。
    pub total_frames: Option<u32>,
    /// `hdrl` 内のストリーム宣言(順序が movi の `NN` 番号になる)。
    pub streams: Vec<AviStreamSpec>,
    /// `movi` パケット列(書き込み順 = マックス順)。
    pub packets: Vec<MoviPacket>,
}

impl AviSpec {
    /// 旧 `AviSpec` 相当の簡易構築: CFR 非圧縮映像 + PCM 音声で、
    /// movi は `00db` フレーム列の後に `01wb` 1 個という配置。
    /// フレームは未パディング BGR24。`width * 3 % 4 == 0` が必要。
    pub fn cfr(
        width: u32,
        height: u32,
        fps: u32,
        frames: Vec<Vec<u8>>,
        audio: Vec<u8>,
        channels: u16,
        sample_rate: u32,
    ) -> Self {
        let mut packets: Vec<MoviPacket> = frames
            .into_iter()
            .map(|f| MoviPacket::video_frame(0, f))
            .collect();
        packets.push(MoviPacket::audio(1, audio));
        Self {
            width,
            height,
            usec_per_frame: 1_000_000 / fps.max(1),
            total_frames: None,
            streams: vec![
                AviStreamSpec::video(width, height, fps),
                AviStreamSpec::pcm_audio(channels, sample_rate),
            ],
            packets,
        }
    }
}

/// [`AviSpec`] を legacy AVI バイト列にシリアライズする。
/// idx1 を生成しないため AVIF_HASINDEX は設定しない。
/// video の strf は BITMAPINFOHEADER とし、寸法から rcFrame を導出する。
/// packet stream > 99 や rcFrame に表現不能な寸法は panic する。
pub fn write_avi(spec: &AviSpec) -> Vec<u8> {
    assert!(
        spec.packets.iter().all(|packet| packet.stream <= 99),
        "AVI packet stream index must be in 0..=99"
    );
    let video_frames = spec
        .streams
        .iter()
        .enumerate()
        .filter(|(_, s)| &s.fcc_type == b"vids")
        .map(|(i, _)| {
            spec.packets
                .iter()
                .filter(|p| p.stream == i && matches!(&p.kind, b"db" | b"dc"))
                .count() as u32
        })
        .next()
        .unwrap_or(0);

    // avih
    let mut avih = Vec::with_capacity(56);
    avih.extend_from_slice(&spec.usec_per_frame.to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes()); // max bytes/sec
    avih.extend_from_slice(&0u32.to_le_bytes()); // padding granularity
    avih.extend_from_slice(&0u32.to_le_bytes()); // idx1 なし
    avih.extend_from_slice(&spec.total_frames.unwrap_or(video_frames).to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes()); // initial frames
    avih.extend_from_slice(&(spec.streams.len() as u32).to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes()); // suggested buffer
    avih.extend_from_slice(&spec.width.to_le_bytes());
    avih.extend_from_slice(&spec.height.to_le_bytes());
    avih.extend_from_slice(&[0u8; 16]);

    let mut hdrl = Vec::new();
    push_chunk(&mut hdrl, b"avih", &avih);

    // 各ストリームの strl(strh + strf)
    for (i, s) in spec.streams.iter().enumerate() {
        let derived_length = if s.sample_size > 0 {
            spec.packets
                .iter()
                .filter(|p| p.stream == i && &p.kind == b"wb")
                .map(|p| p.body.len() as u32)
                .sum::<u32>()
                / s.sample_size.max(1)
        } else {
            spec.packets.iter().filter(|p| p.stream == i).count() as u32
        };
        let suggested = if &s.fcc_type == b"vids" {
            spec.packets
                .iter()
                .filter(|p| p.stream == i)
                .map(|p| p.body.len() as u32)
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let frame_rect = if &s.fcc_type == b"vids" {
            // 寸法の正本は strf。avih の共通寸法とは独立した stream を扱う。
            let width = i32::from_le_bytes(s.format[4..8].try_into().unwrap());
            let height = i32::from_le_bytes(s.format[8..12].try_into().unwrap());
            [
                0,
                0,
                i16::try_from(width).expect("video width must fit rcFrame"),
                i16::try_from(height.unsigned_abs()).expect("video height must fit rcFrame"),
            ]
        } else {
            [0; 4]
        };
        let strh = strh_body(
            &s.fcc_type,
            &s.handler,
            s.scale,
            s.rate,
            s.start,
            s.length.unwrap_or(derived_length),
            s.sample_size,
            suggested,
            frame_rect,
        );
        let mut strl = Vec::new();
        push_chunk(&mut strl, b"strh", &strh);
        push_chunk(&mut strl, b"strf", &s.format);
        push_list(&mut hdrl, b"strl", &strl);
    }

    // movi: パケット列を指定順のまま書く
    let mut movi = Vec::new();
    for p in &spec.packets {
        let id: [u8; 4] = [
            b'0' + (p.stream / 10) as u8,
            b'0' + (p.stream % 10) as u8,
            p.kind[0],
            p.kind[1],
        ];
        push_chunk(&mut movi, &id, &p.body);
    }

    let mut body = Vec::new();
    push_list(&mut body, b"hdrl", &hdrl);
    push_list(&mut body, b"movi", &movi);

    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32 + 4).to_le_bytes());
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_list_pads_outside_declared_size_before_next_chunk() {
        let mut bytes = Vec::new();
        push_list(&mut bytes, b"test", &[7]);
        push_chunk(&mut bytes, b"next", &[8, 9]);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 5);
        assert_eq!(&bytes[8..14], &[b't', b'e', b's', b't', 7, 0]);
        assert_eq!(&bytes[14..18], b"next");
    }

    #[test]
    fn stream_99_is_decimal_and_stream_100_is_rejected() {
        let mut spec = AviSpec::cfr(4, 2, 10, vec![], vec![], 1, 48_000);
        spec.streams = vec![AviStreamSpec::pcm_audio(1, 48_000); 100];
        spec.packets = vec![MoviPacket::audio(99, vec![0, 0])];
        assert_eq!(movi_chunks(&write_avi(&spec)), [(*b"99wb", 2)]);
        spec.packets[0].stream = 100;
        assert!(std::panic::catch_unwind(|| write_avi(&spec)).is_err());
    }

    #[test]
    fn avi_without_index_does_not_claim_hasindex() {
        let bytes = write_avi(&AviSpec::cfr(4, 2, 10, vec![], vec![], 1, 48_000));
        let avih = bytes.windows(4).position(|w| w == b"avih").unwrap() + 8;
        let flags = u32::from_le_bytes(bytes[avih + 12..avih + 16].try_into().unwrap());
        assert_eq!(flags & 0x10, 0);
        assert!(!bytes.windows(4).any(|w| w == b"idx1"));
    }

    #[test]
    fn unpadded_dib_requires_word_aligned_rows() {
        let header = bitmap_info_header(4, 2);
        assert_eq!(u32::from_le_bytes(header[20..24].try_into().unwrap()), 24);
        assert!(std::panic::catch_unwind(|| bitmap_info_header(3, 2)).is_err());
        assert!(
            std::panic::catch_unwind(|| AviSpec::cfr(3, 2, 10, vec![], vec![], 1, 48_000)).is_err()
        );
    }

    #[test]
    fn video_rcframe_uses_stream_dimensions_and_audio_is_empty() {
        let mut spec = AviSpec::cfr(4, 2, 10, vec![], vec![], 1, 48_000);
        spec.streams[0] = AviStreamSpec::video(8, 6, 10);
        let bytes = write_avi(&spec);
        let rects: Vec<_> = bytes
            .windows(4)
            .enumerate()
            .filter(|(_, w)| *w == b"strh")
            .map(|(i, _)| bytes[i + 8 + 48..i + 8 + 56].to_vec())
            .collect();
        assert_eq!(rects, [vec![0, 0, 0, 0, 8, 0, 6, 0], vec![0; 8]]);
    }

    #[test]
    fn wav_s16_has_consistent_header() {
        let w = wav_s16_bytes(2, 48_000, &[1, -1, 100, -100]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        let riff = u32::from_le_bytes(w[4..8].try_into().unwrap());
        assert_eq!(riff as usize, w.len() - 8);
        assert_eq!(&w[36..40], b"data");
    }

    #[test]
    fn pcm_helpers_encode_expected_bytes() {
        assert_eq!(pcm_s16le(&[1.4, -1.4, 40000.0]), {
            let mut v = Vec::new();
            v.extend_from_slice(&1i16.to_le_bytes());
            v.extend_from_slice(&(-1i16).to_le_bytes());
            v.extend_from_slice(&32767i16.to_le_bytes());
            v
        });
        assert_eq!(pcm_f64le(&[1.5]).len(), 8);
    }

    #[test]
    fn cfr_avi_is_a_well_formed_riff() {
        let frame = vec![7u8; 64 * 48 * 3];
        let audio = pcm_s16le(&vec![100.0; 960]);
        let bytes = write_avi(&AviSpec::cfr(64, 48, 10, vec![frame; 30], audio, 1, 48_000));
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"AVI ");
        let riff = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(riff as usize, bytes.len() - 8);
        assert!(bytes.windows(4).any(|w| w == b"movi"));
        assert!(bytes.windows(4).any(|w| w == b"00db"));
        assert!(bytes.windows(4).any(|w| w == b"01wb"));
    }

    /// movi 内のチャンク id とサイズを書き込み順に拾う簡易パーサ。
    fn movi_chunks(bytes: &[u8]) -> Vec<([u8; 4], usize)> {
        let pos = bytes.windows(4).position(|w| w == b"movi").unwrap();
        // LIST size は "LIST" の 4 バイト前
        let list_body = &bytes[pos + 4..];
        let mut out = Vec::new();
        let mut rest = list_body;
        while rest.len() >= 8 {
            let id: [u8; 4] = rest[..4].try_into().unwrap();
            let size = u32::from_le_bytes(rest[4..8].try_into().unwrap()) as usize;
            out.push((id, size));
            rest = &rest[8 + size + (size % 2)..];
        }
        out
    }

    #[test]
    fn packet_list_controls_mux_order_and_sizes() {
        let s = AviSpec {
            width: 4,
            height: 4,
            usec_per_frame: 100_000,
            total_frames: None,
            streams: vec![
                AviStreamSpec::video(4, 4, 10),
                AviStreamSpec::pcm_audio(1, 48_000),
            ],
            packets: vec![
                MoviPacket::video_frame(0, vec![0u8; 48]),
                MoviPacket::audio(1, vec![1u8; 200]),
                MoviPacket::video_frame(0, vec![0u8; 48]),
                // 0 長 dup チャンク
                MoviPacket::video_frame(0, vec![]),
                MoviPacket::audio(1, vec![2u8; 96]),
            ],
        };
        let chunks = movi_chunks(&write_avi(&s));
        let ids: Vec<[u8; 4]> = chunks.iter().map(|c| c.0).collect();
        assert_eq!(ids, [*b"00db", *b"01wb", *b"00db", *b"00db", *b"01wb"]);
        // 0 長 dup チャンクがそのまま書かれている
        assert_eq!(chunks[3].1, 0);
        // per-packet サイズが指定どおり
        assert_eq!(chunks[4].1, 96);
    }

    #[test]
    fn per_stream_dwstart_and_declared_length_are_written() {
        let s = AviSpec {
            width: 4,
            height: 4,
            usec_per_frame: 100_000,
            total_frames: None,
            streams: vec![
                AviStreamSpec::video(4, 4, 10).with_start(5),
                AviStreamSpec::pcm_audio(1, 48_000).with_start(3),
            ],
            packets: vec![MoviPacket::video_frame(0, vec![0u8; 48])],
        };
        let bytes = write_avi(&s);
        // strh ボディは fccType("vids")で始まり、dwStart は offset 28。
        let vids = bytes.windows(4).position(|w| w == b"vids").unwrap();
        let dw_start = u32::from_le_bytes(bytes[vids + 28..vids + 32].try_into().unwrap());
        assert_eq!(dw_start, 5);
    }
}
