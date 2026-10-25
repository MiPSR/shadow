pub struct Capabilities {
    pub audio_input: bool,
    pub audio_output: bool,
}

pub use cpal::traits::HostTrait;
pub fn detect() -> Capabilities {
    let host = cpal::default_host();
    Capabilities {
        audio_input: host.default_input_device().is_some(),
        audio_output: host.default_output_device().is_some(),
    }
}
