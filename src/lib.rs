pub mod dialog;
pub mod onnx;
pub mod orchestrator;
pub mod phonemizer;
pub mod piper;
pub mod wav;

pub use dialog::{parse_dialogo, Segment, Speaker};
pub use orchestrator::{render_dialogo, DialogoResultado};
pub use phonemizer::Phonemizer;
pub use piper::{Piper, VoiceManager};
pub use wav::{f32_to_i16, samples_to_wav_bytes};
