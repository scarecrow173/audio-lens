//! ストーリー駆動のゲームプレイ動画 fixture 生成:
//! `cargo run -p al-cli --example gen_scene -- samples/`
//!
//! 2 人のキャラクターの会話シーンから敵の出現を経て戦闘シーンに
//! 遷移する 10 秒の AVI(BGR24 非圧縮映像 + PCM s16 ステレオ音声)。
//! 音声レイヤは環境音・BGM・声(パン付き)・SE(足音/斬撃/魔法/爆発)
//! で構成し、全て決定的(シード固定)に合成する。

use std::path::PathBuf;

use al_testkit::media::{AviSpec, pcm_s16le, write_avi};
use al_testkit::signal::{Biquad, Rng, embed, interleave, noise};

const RATE: u32 = 48_000;
const FPS: u32 = 10;
const W: u32 = 128;
const H: u32 = 96;
const DUR_S: f64 = 10.0;
const N: usize = (RATE as f64 * DUR_S) as usize;

// ---------- audio helpers ----------

/// `t` 秒の時刻をサンプル index にする。
fn at(t: f64) -> usize {
    (t * f64::from(RATE)) as usize
}

/// `dur` 秒のスムースな矩形エンベロープ(立ち上がり/下がり 8 ms)。
fn envelope(dur: f64, edge: f64) -> Vec<f64> {
    let len = at(dur);
    let e = at(edge.min(dur / 2.0)).max(1);
    (0..len)
        .map(|i| {
            if i < e {
                i as f64 / e as f64
            } else if i >= len - e {
                (len - i) as f64 / e as f64
            } else {
                1.0
            }
        })
        .collect()
}

/// 減衰する正弦波バースト(打撃・足音・ブーム)。
fn decay_sine(freq: f64, dur: f64, amp: f64, decay_hz: f64) -> Vec<f64> {
    (0..at(dur))
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            (2.0 * std::f64::consts::PI * freq * t).sin() * amp * (-decay_hz * t).exp()
        })
        .collect()
}

/// ノイズバースト + エンベロープ + ローパス(爆発・whoosh 系)。
fn noise_hit(dur: f64, amp: f64, seed: u64, lpf: f64) -> Vec<f64> {
    let n = noise(at(dur), seed, amp);
    let env = envelope(dur, dur.min(0.08));
    let shaped: Vec<f64> = n.iter().zip(&env).map(|(a, b)| a * b).collect();
    Biquad::lowpass(RATE, lpf, 0.8).apply(&shaped)
}

/// 上昇掃引する正弦波(魔法の詠唱・発動)。位相は瞬時周波数を
/// 積分して求める(`sin(2πft)` だと掃引が倍速で overshoot する)。
fn sweep(f0: f64, f1: f64, dur: f64, amp: f64) -> Vec<f64> {
    let len = at(dur);
    let env = envelope(dur, 0.02);
    let mut phase = 0.0;
    (0..len)
        .map(|i| {
            let f = f0 + (f1 - f0) * i as f64 / len as f64;
            phase += 2.0 * std::f64::consts::PI * f / f64::from(RATE);
            phase.sin() * amp * env[i]
        })
        .collect()
}

/// 母音っぽい1音節: 高調波を重ねた短い発声音 + 緩いビブラート。
/// `base` でキャラの声高を分ける(A=150 Hz 男性寄り / B=240 Hz)。
fn syllable(base: f64, dur: f64, amp: f64, seed: u64) -> Vec<f64> {
    let len = at(dur);
    let env = envelope(dur, dur.min(0.03));
    let mut rng = Rng::new(seed);
    (0..len)
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            let vib = 1.0 + 0.04 * (2.0 * std::f64::consts::PI * 5.5 * t).sin();
            let f = base * vib;
            let voice = (2.0 * std::f64::consts::PI * f * t).sin()
                + 0.5 * (2.0 * std::f64::consts::PI * f * 2.0 * t).sin()
                + 0.25 * (2.0 * std::f64::consts::PI * f * 3.0 * t).sin();
            voice * env[i] * amp + rng.symmetric() * amp * 0.04
        })
        .collect()
}

/// `syllable` の連続で1フレーズ分の「発話」を作る。
/// 音節列のリズムとピッチはシードから決定的に揺らす。
fn phrase(base: f64, syllables: usize, amp: f64, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for _ in 0..syllables {
        let dur = 0.09 + rng.next_f64() * 0.07;
        let gap = 0.02 + rng.next_f64() * 0.05;
        let pitch = base * (0.9 + rng.next_f64() * 0.25);
        let syl = syllable(pitch, dur, amp, rng.next_u64());
        out.extend_from_slice(&syl);
        out.resize(out.len() + at(gap), 0.0);
    }
    out
}

/// ステレオ `dst` に `pan`(-1.0=L 全振り … +1.0=R 全振り)で
/// モノラル `src` を `t` 秒位置に重畳する。
fn place(dst: &mut [Vec<f64>], src: &[f64], t: f64, pan: f64, gain: f64) {
    let lg = gain * (1.0 - pan.max(0.0));
    let rg = gain * (1.0 + pan.min(0.0));
    embed(&mut dst[0], src, at(t), lg);
    embed(&mut dst[1], src, at(t), rg);
}

/// 環境音(風): 低域に寄せたゆるく揺れるノイズ床。
fn ambience(seed: u64) -> Vec<f64> {
    let n = noise(N, seed, 700.0);
    let lp = Biquad::lowpass(RATE, 500.0, 0.7).apply(&n);
    (0..N)
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            lp[i] * (0.7 + 0.3 * (2.0 * std::f64::consts::PI * 0.3 * t).sin())
        })
        .collect()
}

/// BGM: 会話パートは長三和音の分散和音、戦闘パートは低音の刻み +
/// 短三和音。`start..end` の区間だけ鳴る。
fn bgm(start: f64, end: f64, battle: bool, amp: f64) -> Vec<f64> {
    let len = at(end) - at(start);
    let notes = if battle {
        // 刻み: 低音 + 短三和音(緊張感)
        vec![[110.0, 130.8, 164.8], [98.0, 116.5, 146.8]]
    } else {
        // 分散和音: 長三和音を分散させる
        vec![[196.0, 246.9, 293.7], [220.0, 261.6, 329.6]]
    };
    let beat = if battle { 0.25 } else { 1.0 };
    (0..len)
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            let b = (t / beat) as usize;
            let chord = notes[b % notes.len()];
            let v = if battle && (t % beat) < 0.12 {
                // 低音パルス
                (2.0 * std::f64::consts::PI * 55.0 * t).sin() * 1.6
            } else {
                chord
                    .iter()
                    .map(|f| (2.0 * std::f64::consts::PI * f * t).sin())
                    .sum::<f64>()
                    / chord.len() as f64
            };
            v * amp
        })
        .collect::<Vec<f64>>()
        .iter()
        .enumerate()
        .map(|(i, v)| {
            // フェードイン/アウト 0.4s
            let e = at(0.4);
            let g = if i < e {
                i as f64 / e as f64
            } else if i >= len - e {
                (len - i) as f64 / e as f64
            } else {
                1.0
            };
            v * g
        })
        .collect()
}

// ---------- video helpers ----------

fn put(px: &mut [u8], x: i32, y: i32, b: u8, g: u8, r: u8) {
    let (w, h) = (W as i32, H as i32);
    if x < 0 || x >= w || y < 0 || y >= h {
        return;
    }
    // ボトムアップ: ファイル先頭が最下行
    let i = ((h - 1 - y) * w + x) as usize * 3;
    px[i] = b;
    px[i + 1] = g;
    px[i + 2] = r;
}

fn rect(px: &mut [u8], x: i32, y: i32, w: i32, h: i32, c: (u8, u8, u8)) {
    for dy in 0..h {
        for dx in 0..w {
            put(px, x + dx, y + dy, c.0, c.1, c.2);
        }
    }
}

fn circle(px: &mut [u8], cx: i32, cy: i32, r: i32, c: (u8, u8, u8)) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                put(px, cx + dx, cy + dy, c.0, c.1, c.2);
            }
        }
    }
}

/// キャラクター1体: 身体 + 頭 + 簡易腕。`crouch` で姿勢を低くする。
/// `y` は胴体の上端(画面座標: 下ほど y が大きい)。
fn sprite(px: &mut [u8], x: i32, y: i32, c: (u8, u8, u8), crouch: bool) {
    let hgt = if crouch { 14 } else { 20 };
    rect(px, x - 4, y, 8, hgt, c); // body
    circle(px, x, y - 4, 5, (120, 150, 190)); // head(肌色 BGR、胴体の上)
    rect(px, x - 7, y + 4, 3, 10, c); // left arm
    rect(px, x + 4, y + 4, 3, 10, c); // right arm
}

/// 敵(赤いスライム状): 丸みのある塊 + 目。
fn enemy(px: &mut [u8], x: i32, y: i32, flash: bool) {
    let body = if flash {
        (240, 240, 240)
    } else {
        (30, 30, 200)
    };
    circle(px, x, y + 8, 12, body);
    circle(px, x - 4, y + 12, 2, (255, 255, 255));
    circle(px, x + 4, y + 12, 2, (255, 255, 255));
}

/// 吹き出し: 発話中のキャラの頭上に白い矩形 + 尾。
fn speech_bubble(px: &mut [u8], x: i32, y: i32, ticks: usize) {
    rect(px, x - 10, y, 20, 10, (240, 240, 240));
    rect(px, x - 2, y + 10, 4, 4, (240, 240, 240)); // 尾(下向き)
    // 話している間だけ内部に点が出る
    for i in 0..3 {
        if (ticks + i * 2) % 6 < 3 {
            put(px, x - 5 + i as i32 * 5, y + 5, 40, 40, 40);
        }
    }
}

/// 1 フレームを描画する。t = 秒。`speech` は (開始, 長さ, 話者 0=A/1=B)。
fn scene_frame(t: f64, speech: &[(f64, f64, i32)]) -> Vec<u8> {
    let mut px = vec![0u8; (W * H * 3) as usize];
    // 空と地面
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let (b, g, r) = if y > 60 {
                (30, 90, 30) // 地面(緑)
            } else {
                let v = 60 + (60 - y) * 2; // 空の縦グラデーション(青)
                (v as u8, (v as u8) / 2 + 20, (v as u8) / 4)
            };
            put(&mut px, x, y, b, g, r);
        }
    }

    // 戦闘シーン(4.3s 以降): ヒット直後に全体を数 px 揺らす
    let battle = t >= 4.3;
    let hit_moment = [4.3, 4.5, 5.7, 6.5, 7.3, 8.1]
        .iter()
        .any(|&h| (t - h).abs() < 0.15);
    let shake: i32 = if hit_moment { 2 } else { 0 };

    // キャラクター位置(会話中はじっと、戦闘で動く)
    let mut ax = 30.0;
    let mut bx = 90.0;
    let mut ay = 40.0;
    let by = 40.0;
    let mut crouch_a = false;
    let mut crouch_b = false;
    if battle {
        let bt = t - 4.3;
        ax = 30.0 + (bt * 30.0).min(48.0); // A が敵に向かって走る(5.9s で x=78)
        bx = 90.0 - (bt * 15.0).min(20.0); // B が少し下がる(5.6s で x=70)
        crouch_a = bt > 1.6 && (bt * 2.0) % 2.0 < 1.0; // 回避モーション
        crouch_b = bt > 1.6 && (bt * 2.0) % 2.0 >= 1.0;
        ay += (bt * 6.0).sin() * 2.0; // 上下動
    }
    // 敵は右の画面外から滑り込み、x=95 で停止して最後まで残る
    let enemy_x = if battle {
        ((W as i32 + 20) - ((t - 4.3) * 40.0) as i32).max(95)
    } else {
        W as i32 + 20
    };
    let enemy_flash = battle && [5.7, 6.5, 7.3, 8.1].iter().any(|&h| (t - h).abs() < 0.15);

    // 描画(奥→手前、ヒット時は shake だけずらす)。
    // 敵は画面外座標のまま渡し、put() の範囲チェックで自然にクリップする。
    if enemy_x <= W as i32 + 14 {
        enemy(&mut px, enemy_x, 46 + shake, enemy_flash);
    }
    sprite(
        &mut px,
        ax as i32,
        ay as i32 + shake,
        (200, 80, 40),
        crouch_a,
    ); // A: 青
    sprite(
        &mut px,
        bx as i32,
        by as i32 + shake,
        (40, 160, 60),
        crouch_b,
    ); // B: 緑

    // 斬撃エフェクト: A の腕から敵方向への白い弧(5.45-5.65s)
    if battle && t > 5.45 && t < 5.65 {
        for i in 0..10 {
            let dx = i * 2;
            let arc = (i as f64 * 0.35).sin() * 6.0;
            put(
                &mut px,
                ax as i32 + 8 + dx,
                44 - arc as i32 + shake,
                255,
                255,
                255,
            );
        }
    }

    // 会話パートの吹き出し(実際の発話区間と同期)
    if !battle {
        for &(start, len, who) in speech {
            if t >= start && t < start + len {
                let (sx, sy) = if who == 0 {
                    (ax as i32, ay as i32 - 24)
                } else {
                    (bx as i32, by as i32 - 24)
                };
                speech_bubble(&mut px, sx, sy, (t * 10.0) as usize);
            }
        }
    }

    // VFX フラッシュ(爆発・ヒット)
    if [4.5, 6.5].iter().any(|&h| (t - h).abs() < 0.1) || (t - 5.7).abs() < 0.08 {
        for p in px.iter_mut() {
            *p = p.saturating_add(140);
        }
    }
    // 魔法エフェクト: B から敵への光の筋。着弾(6.5s)と爆発が一致。
    if battle && t > 6.15 && t < 6.5 {
        let p = (t - 6.15) / 0.35;
        let sx = bx as i32 + ((enemy_x - bx as i32) as f64 * p) as i32;
        circle(&mut px, sx, 50 + shake, 3, (255, 255, 120));
        circle(&mut px, sx, 50 + shake, 6, (200, 200, 60));
    }
    // フェードイン/アウト(末尾は最終フレーム t=9.9 で完全に黒)
    let fade = if t < 0.4 {
        1.0 - t / 0.4
    } else if t > 9.4 {
        ((t - 9.4) / 0.5).min(1.0)
    } else {
        0.0
    };
    if fade > 0.0 {
        for p in px.iter_mut() {
            *p = (*p as f64 * (1.0 - fade)) as u8;
        }
    }
    px
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples"));
    std::fs::create_dir_all(dir.join("video")).unwrap();

    // ============ audio ============
    // ステレオのミキシングバッファ(L=0, R=1)
    let mut mix = vec![vec![0.0; N], vec![0.0; N]];

    // 環境音(風) — L/R を別シードで少し非相関に
    place(&mut mix, &ambience(101), 0.0, -0.3, 1.0);
    place(&mut mix, &ambience(102), 0.0, 0.3, 1.0);

    // BGM — 会話 0.0-4.3 / 戦闘 4.3-10.0 で別フレーズ
    place(&mut mix, &bgm(0.0, 4.3, false, 4_000.0), 0.0, 0.0, 1.0);
    place(&mut mix, &bgm(4.3, 10.0, true, 4_500.0), 4.3, 0.0, 1.0);

    // 声 — A(150Hz・左寄り) / B(240Hz・右寄り)。実際のフレーズ長を
    // 吹き出し表示区間にも使うため生成物を保持する。
    let voice_a1 = phrase(150.0, 7, 9_000.0, 201);
    let voice_b = phrase(240.0, 9, 9_000.0, 202);
    let voice_a2 = phrase(150.0, 4, 11_000.0, 203);
    let speech = [
        (0.5, voice_a1.len() as f64 / f64::from(RATE), 0), // A
        (1.9, voice_b.len() as f64 / f64::from(RATE), 1),  // B
        (3.55, voice_a2.len() as f64 / f64::from(RATE), 0), // A(B の末尾と重ならない開始)
    ];
    place(&mut mix, &voice_a1, 0.5, -0.5, 1.0);
    place(&mut mix, &voice_b, 1.9, 0.5, 1.0);
    place(&mut mix, &voice_a2, 3.55, -0.5, 1.0);
    // 戦闘の掛け声(画面位置に合わせてパン)
    place(&mut mix, &phrase(180.0, 3, 12_000.0, 204), 5.5, 0.2, 1.0);
    place(&mut mix, &phrase(260.0, 4, 12_000.0, 205), 6.1, 0.3, 1.0);

    // SE: 敵出現のブーム + 画面フラッシュの爆発。
    // 低域に寄せたノイズは LPF で大幅減衰するため大きめの振幅が必要。
    place(
        &mut mix,
        &noise_hit(0.8, 60_000.0, 301, 400.0),
        4.3,
        0.4,
        1.0,
    );
    place(
        &mut mix,
        &noise_hit(0.6, 80_000.0, 302, 800.0),
        4.5,
        0.0,
        1.0,
    );

    // SE: 足音 — A が駆け寄る区間(4.6-5.9s)のみ。
    for i in 0..5 {
        place(
            &mut mix,
            &decay_sine(90.0, 0.12, 7_000.0, 18.0),
            4.6 + i as f64 * 0.3,
            0.1,
            1.0,
        );
    }
    // B の後ずさり(4.5-5.6s)
    for i in 0..3 {
        place(
            &mut mix,
            &decay_sine(90.0, 0.12, 6_000.0, 18.0),
            4.8 + i as f64 * 0.35,
            0.3,
            1.0,
        );
    }

    // SE: 剣の斬撃 whoosh + ヒット(敵は画面右にいるので右寄りパン)
    place(
        &mut mix,
        &noise_hit(0.25, 30_000.0, 303, 6_000.0),
        5.5,
        0.3,
        1.0,
    );
    place(
        &mut mix,
        &decay_sine(70.0, 0.3, 16_000.0, 10.0),
        5.7,
        0.4,
        1.0,
    );

    // SE: 魔法詠唱(B)→ 着弾した瞬間に爆発
    place(
        &mut mix,
        &sweep(400.0, 2_400.0, 0.35, 8_000.0),
        6.15,
        0.3,
        1.0,
    );
    place(
        &mut mix,
        &noise_hit(0.7, 90_000.0, 304, 500.0),
        6.5,
        0.4,
        1.0,
    );

    // SE: 追撃の打撃×2(敵の位置=右寄りに合わせる)
    place(
        &mut mix,
        &decay_sine(80.0, 0.25, 14_000.0, 12.0),
        7.3,
        0.4,
        1.0,
    );
    place(
        &mut mix,
        &decay_sine(80.0, 0.25, 14_000.0, 12.0),
        8.1,
        0.4,
        1.0,
    );

    // マスターフェード(先頭0.4s / 末尾0.5s)で端部のクリックを防ぐ
    let fade_in = at(0.4);
    let fade_out = at(0.5);
    for c in &mut mix {
        for (i, v) in c.iter_mut().enumerate() {
            if i < fade_in {
                *v *= i as f64 / fade_in as f64;
            } else if i >= N - fade_out {
                *v *= (N - i) as f64 / fade_out as f64;
            }
        }
    }

    // ピーク正規化(決定的)してクリップを防ぐ
    let peak = mix
        .iter()
        .flat_map(|c| c.iter())
        .fold(0.0f64, |m, &v| m.max(v.abs()))
        .max(1.0);
    let scale = 28_000.0 / peak; // i16 の ~85% に頭を揃える
    for c in &mut mix {
        for v in c.iter_mut() {
            *v *= scale;
        }
    }
    let stereo = interleave(&mix);

    // ============ video ============
    let n_frames = (N as u64 * u64::from(FPS) / u64::from(RATE)) as usize;
    let frames: Vec<Vec<u8>> = (0..n_frames)
        .map(|f| scene_frame(f as f64 / f64::from(FPS), &speech))
        .collect();

    let bytes = write_avi(&AviSpec::cfr(
        W,
        H,
        FPS,
        frames,
        pcm_s16le(&stereo),
        2,
        RATE,
    ));
    let path = dir.join("video/scene-dialogue-battle.avi");
    std::fs::write(&path, bytes).unwrap();
    println!(
        "wrote {} ({} frames, {:.1}s)",
        path.display(),
        n_frames,
        DUR_S
    );
}
