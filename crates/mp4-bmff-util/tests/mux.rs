//!

const SAMPLE_MP4: &[u8] = include_bytes!("samples/sample.mp4");

#[test]
fn mux_roundtrip_video() {
    let _ = SAMPLE_MP4;
    todo!("Implement muxing test for sample.mp4");
}
