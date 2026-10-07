use anyhow::{anyhow, Result};

use crate::dialog::{parse_dialogo, Speaker};
use crate::phonemizer::Phonemizer;
use crate::piper::VoiceManager;
use crate::wav::f32_to_i16;

pub struct DialogoResultado {
    pub pcm: Vec<i16>,
    pub sr: u32,
    pub segmentos: usize,
}

fn resamplear(pcm: &[i16], sr_in: u32, sr_out: u32) -> Vec<i16> {
    if sr_in == sr_out || pcm.is_empty() { return pcm.to_vec(); }
    let ratio = sr_out as f64 / sr_in as f64;
    let n_out = (pcm.len() as f64 * ratio) as usize;
    let mut out = Vec::with_capacity(n_out);
    for i in 0..n_out {
        let src = i as f64 / ratio;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(pcm.len() - 1);
        let t = (src - i0 as f64) as f32;
        let s = pcm[i0] as f32 * (1.0 - t) + pcm[i1] as f32 * t;
        out.push(s as i16);
    }
    out
}

pub fn render_dialogo(
    voices: &mut VoiceManager,
    ph: &mut Phonemizer,
    texto: &str,
    voice_default: &str,
    speakers: &[Speaker],
    pausa_entre_ms: u32,
    length_scale: Option<f32>,
    noise_scale: Option<f32>,
    noise_w: Option<f32>,
) -> Result<DialogoResultado> {
    let (segmentos, ignoradas) = parse_dialogo(texto, voice_default, speakers);
    if ignoradas > 0 {
        tracing::warn!("[orchestrator] {} tag(s) desconhecida(s)", ignoradas);
    }
    if segmentos.is_empty() { return Err(anyhow!("nenhum segmento")); }

    let mut pcm: Vec<i16> = Vec::new();
    let mut sr_final: u32 = 0;

    for (i, seg) in segmentos.iter().enumerate() {
        let chunks = ph.process_chunks(&seg.text)?;

        let piper = voices.get_mut(&seg.voice)
            .ok_or_else(|| anyhow!("voz desconhecida: {}", seg.voice))?;

        let mut local: Vec<i16> = Vec::new();
        for ch in &chunks {
            if ch.fragments.is_empty() { continue; }
            let mut ipa = String::new();
            for (j, fr) in ch.fragments.iter().enumerate() {
                if j > 0 { ipa.push(' '); }
                ipa.push_str(&fr.ipa);
                if let Some(p) = fr.punct { ipa.push(p); }
            }
            let (f32s, _) = piper.create(
                &ipa, false, seg.sid,
                length_scale, noise_scale, noise_w,
            )?;
            local.extend(f32_to_i16(&f32s));
        }
        let sr = piper.sample_rate();

        let amostras = if sr_final != 0 && sr != sr_final {
            resamplear(&local, sr, sr_final)
        } else {
            if sr_final == 0 { sr_final = sr; }
            local
        };
        pcm.extend(amostras);

        if i + 1 < segmentos.len() && pausa_entre_ms > 0 {
            let n = ((pausa_entre_ms as u64 * sr_final as u64) / 1000) as usize;
            pcm.extend(std::iter::repeat(0i16).take(n));
        }
    }
    Ok(DialogoResultado { pcm, sr: sr_final, segmentos: segmentos.len() })
}


#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn resample_mesmo_sr_e_identidade() {
        let pcm = vec![100i16, 200, 300, 400];
        let out = resamplear(&pcm, 22050, 22050);
        assert_eq!(out, pcm);
    }

    #[test]
    fn resample_downsample() {
        let pcm: Vec<i16> = (0..1000).map(|i| i as i16).collect();
        let out = resamplear(&pcm, 22050, 16000);
        // 1000 * 16000/22050 ≈ 725 amostras
        assert!((out.len() as i64 - 725).abs() < 5);
    }

    #[test]
    fn resample_upsample() {
        let pcm: Vec<i16> = (0..1000).map(|i| i as i16).collect();
        let out = resamplear(&pcm, 16000, 22050);
        // 1000 * 22050/16000 ≈ 1378 amostras
        assert!((out.len() as i64 - 1378).abs() < 5);
    }

    #[test]
    fn resample_vazio() {
        let out = resamplear(&[], 22050, 16000);
        assert!(out.is_empty());
    }
}
