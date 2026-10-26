use image::ImageFormat;
use std::io::Cursor;

pub fn image_to_webp(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    if img.width() > 1920 || img.height() > 1080 {
        img = img.resize(1920, 1080, image::imageops::FilterType::Lanczos3);
    }
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::WebP).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

pub fn verify_webp(bytes: &[u8]) -> bool {
    image::load_from_memory_with_format(bytes, ImageFormat::WebP).is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn wav_to_opus_ogg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use hound::WavReader;
    use ogg::PacketWriter;
    use opus::{Application, Channels, Encoder};
    use std::io::Cursor;

    let reader = WavReader::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.sample_rate != 48000 {
        return Err("only 48kHz WAV supported".into());
    }
    let channels = if spec.channels == 1 { Channels::Mono } else { Channels::Stereo };
    let mut encoder = Encoder::new(48000, channels, Application::Audio).map_err(|e| e.to_string())?;

    // Read all samples as i16
    let samples: Vec<i16> = reader.into_samples::<i16>().filter_map(|s| s.ok()).collect();
    let frame_samples = 960 * spec.channels as usize;

    let mut out = Cursor::new(Vec::new());
    let mut pw = PacketWriter::new(&mut out);

    // OpusHead
    let mut head = Vec::new();
    head.extend_from_slice(b"OpusHead");
    head.push(1); // version
    head.push(spec.channels as u8);
    head.extend_from_slice(&312i16.to_le_bytes()); // pre-skip
    head.extend_from_slice(&48000u32.to_le_bytes());
    head.extend_from_slice(&0i16.to_le_bytes()); // gain
    head.push(0); // channel mapping family
    pw.write_packet(&head, 1, ogg::PacketWriteEndInfo::NormalPacket, 0).map_err(|e| e.to_string())?;

    // OpusTags
    let mut tags = Vec::new();
    tags.extend_from_slice(b"OpusTags");
    let vendor = b"shadow";
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0u32.to_le_bytes()); // no comments
    pw.write_packet(&tags, 1, ogg::PacketWriteEndInfo::NormalPacket, 0).map_err(|e| e.to_string())?;

    // Audio frames
    let mut granule: u64 = 0;
    let mut i = 0;
    while i + frame_samples <= samples.len() {
        let frame = &samples[i..i + frame_samples];
        let mut buf = vec![0u8; 4000];
        let n = encoder.encode(frame, &mut buf).map_err(|e| e.to_string())?;
        pw.write_packet(buf[..n].to_vec(), 1, ogg::PacketWriteEndInfo::NormalPacket, granule).map_err(|e| e.to_string())?;
        i += frame_samples;
        granule += 960;
    }
    // final page
    let _ = pw.write_packet(&[], 1, ogg::PacketWriteEndInfo::EndStream, granule);
    Ok(out.into_inner())
}

pub fn video_supported() -> bool { false }
pub fn audio_supported() -> bool { true }
