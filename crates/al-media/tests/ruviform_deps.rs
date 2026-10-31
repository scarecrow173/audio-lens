//! Ruviform Git 依存(AL#100 で消費するメディア I/O 境界)の import
//! smoke-test。基礎型の構築とリンクのみを検証する
//! (docs/designs/ruviform-dependency-ci.md §1)。
use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
use ruviform_media_core::{MediaTime, TimeBase};
use ruviform_media_io::{CodecDescriptor, CodecFormat, CodecId};
use ruviform_video_core::{PixelFormat, VideoFrame};

#[test]
fn media_core_timebase_types() {
    assert_ne!(ruviform_media_core::version(), "");
    let time = MediaTime::new(90_000, TimeBase::COMMON_90K);
    assert_eq!(time, MediaTime::new(1, TimeBase::new(1, 1).unwrap()));
}

#[test]
fn audio_core_buffer_types() {
    assert_ne!(ruviform_audio_core::version(), "");
    let spec = BufferSpec {
        layout: ChannelLayout::Stereo,
        frames: 4,
        sample_rate: SampleRate::new(48_000).unwrap(),
    };
    let buffer =
        AudioBuffer::from_interleaved(&[0.0_f32, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0], spec).unwrap();
    let view = buffer.view();
    assert_eq!(view.channels(), 2);
    assert_eq!(view.frames(), 4);
}

#[test]
fn video_core_frame_types() {
    assert_ne!(ruviform_video_core::version(), "");
    let pts = MediaTime::new(0, TimeBase::COMMON_90K);
    let frame = VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, pts).unwrap();
    assert_eq!(frame.plane(0).unwrap().width(), 4);
}

#[test]
fn media_io_boundary_types() {
    assert_ne!(ruviform_media_io::version(), "");
    let descriptor = CodecDescriptor {
        codec: CodecId::Aac,
        codec_private: Vec::new(),
        format: CodecFormat::Data,
    };
    assert_eq!(descriptor.codec.name(), "aac");
    assert!(descriptor.audio_format().is_none());
}
