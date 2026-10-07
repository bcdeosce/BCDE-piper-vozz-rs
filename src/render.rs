//! Render de chunks em PCM — modo "Piper puro".
//!
//! Reconstrói a IPA do chunk com pontuação e faz UMA inferência.
//! Sem config de emoção, sem inserção de pausa, sem silêncio manual.
//! Length_scale = 1.0 (igual ao Piper nativo).

use anyhow::Result;

use crate::piper::Piper;
use crate::wav::f32_to_i16;

pub use vozz_g2p_rs::piper_pipeline::{Chunk, Fragmento as Fragment};

pub fn render_chunks(
    piper: &mut Piper,
    chunks: &[Chunk],
    _emotion: &str,
    sid: Option<i64>,
) -> Result<(Vec<i16>, u32)> {
    let sr = piper.sample_rate();
    let mut out: Vec<i16> = Vec::new();

    for ch in chunks {
        if ch.fragments.is_empty() { continue; }

        let mut ipa_full = String::new();
        for (i, fr) in ch.fragments.iter().enumerate() {
            if i > 0 { ipa_full.push(' '); }
            ipa_full.push_str(&fr.ipa);
            if let Some(p) = fr.punct {
                ipa_full.push(p);
            }
        }

        // ls = 1.0 → igual ao Piper nativo
        let (f32s, _) = piper.create(&ipa_full, false, sid, Some(1.0), None, None)?;
        out.extend(f32_to_i16(&f32s));
    }

    Ok((out, sr))
}
