use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use opus::{Application, Channels, Decoder, Encoder};
use socket2::{Domain, Socket, Type};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct VoiceSession {
    pub running: Arc<Mutex<bool>>,
    pub error: Arc<Mutex<Option<String>>>,
    pub handle: Option<thread::JoinHandle<()>>,
}

impl VoiceSession {
    pub fn start(peer: SocketAddr, key: [u8; 32]) -> Result<Self, String> {
        let running = Arc::new(Mutex::new(true));
        let error = Arc::new(Mutex::new(None));
        let running_clone = running.clone();
        let error_clone = error.clone();
        let handle = thread::spawn(move || {
            if let Err(e) = run_voice(peer, key, running_clone.clone(), error_clone.clone()) {
                eprintln!("voice error: {}", e);
                *error_clone.lock().unwrap() = Some(e);
            }
            *running_clone.lock().unwrap() = false;
        });
        Ok(Self { running, error, handle: Some(handle) })
    }

    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        if let Some(h) = self.handle.take() { let _ = h.join(); }
    }
}

fn run_voice(peer: SocketAddr, key: [u8; 32], running: Arc<Mutex<bool>>, error: Arc<Mutex<Option<String>>>) -> Result<(), String> {
    let host = cpal::default_host();
    let input_dev = host.default_input_device().ok_or("no input device")?;
    let output_dev = host.default_output_device().ok_or("no output device")?;

    let in_config = input_dev.default_input_config().map_err(|e| e.to_string())?;
    let out_config = output_dev.default_output_config().map_err(|e| e.to_string())?;

    let sample_rate = 48000u32;
    let channels = 1usize;
    let frame_samples = 960usize; // 20ms

    // UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_nonblocking(true).ok();

    // playback buffer
    let playback: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let playback_cb = playback.clone();

    // output stream
    let out_stream = output_dev.build_output_stream(
        &out_config.config(),
        move |data: &mut [f32], _| {
            let mut buf = playback_cb.lock().unwrap();
            for sample in data.iter_mut() {
                if !buf.is_empty() {
                    *sample = buf.remove(0);
                } else {
                    *sample = 0.0;
                }
            }
        },
        |err| eprintln!("output stream err: {}", err),
        None,
    ).map_err(|e| e.to_string())?;
    out_stream.play().map_err(|e| e.to_string())?;

    // encoder/decoder
    let mut encoder = Encoder::new(sample_rate, Channels::Mono, Application::Voip).map_err(|e| e.to_string())?;
    let mut decoder = Decoder::new(sample_rate, Channels::Mono).map_err(|e| e.to_string())?;

    // input stream
    let socket_send = socket.try_clone().map_err(|e| e.to_string())?;
    let running_in = running.clone();
    let mut pcm_buf: Vec<f32> = Vec::with_capacity(frame_samples);
    let in_stream = input_dev.build_input_stream(
        &in_config.config(),
        move |data: &[f32], _| {
            if !*running_in.lock().unwrap() { return; }
            for &s in data {
                pcm_buf.push(s);
                if pcm_buf.len() >= frame_samples {
                    // encode
                    let mut opus_buf = vec![0u8; 4000];
                    // convert f32 to i16
                    let mut i16buf = vec![0i16; frame_samples];
                    for (i, &s) in pcm_buf.iter().enumerate() {
                        let v = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        i16buf[i] = v;
                    }
                    if let Ok(n) = encoder.encode(&i16buf, &mut opus_buf) {
                        let ct = opus_buf[..n].to_vec();
                        // encrypt
                        if let Ok((nonce, ciphertext)) = shared::crypto::xchacha::encrypt(&key, &ct, &[]) {
                            let mut pkt = Vec::with_capacity(24 + ciphertext.len());
                            pkt.extend_from_slice(&nonce);
                            pkt.extend_from_slice(&ciphertext);
                            let _ = socket_send.send_to(&pkt, peer);
                        }
                    }
                    pcm_buf.clear();
                }
            }
        },
        |err| eprintln!("input stream err: {}", err),
        None,
    ).map_err(|e| e.to_string())?;
    in_stream.play().map_err(|e| e.to_string())?;

    // receive loop
    let mut recv_buf = vec![0u8; 4096];
    let mut last_recv = std::time::Instant::now();
    while *running.lock().unwrap() {
        if last_recv.elapsed().as_secs() > 5 {
            *error.lock().unwrap() = Some("no packets received".into());
            *running.lock().unwrap() = false;
            break;
        }
        if let Ok((n, _)) = socket.recv_from(&mut recv_buf) {
            last_recv = std::time::Instant::now();
            let pkt = &recv_buf[..n];
            if pkt.len() >= 24 {
                let (nonce, ct) = pkt.split_at(24);
                if let Ok(pt) = shared::crypto::xchacha::decrypt(&key, nonce, ct, &[]) {
                    // decode opus
                    let mut pcm = vec![0i16; frame_samples];
                    if let Ok(decoded) = decoder.decode(&pt, &mut pcm, false) {
                        let mut buf = playback.lock().unwrap();
                        for i in 0..decoded {
                            buf.push(pcm[i] as f32 / 32767.0);
                        }
                    }
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}
