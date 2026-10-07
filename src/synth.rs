//! Fragmento IPA (já no alfabeto Piper) → amostras i16.
//! Inclui a conversão IPA → phoneme IDs.

use crate::voices::Voice;
use std::collections::HashMap;

/// IDs reservados no phoneme_id_map do Piper.
const BOS: &str = "^";
const EOS: &str = "$";
const PAD: &str = "_";

/// Converte IPA (formato Piper) em phoneme IDs.
pub fn ipa_para_ids(ipa: &str, id_map: &HashMap<String, Vec<i64>>) -> Vec<i64> {
    let mut ids: Vec<i64> = Vec::with_capacity(ipa.len() * 2);

    if let Some(v) = id_map.get(BOS) { ids.extend(v); }
    if let Some(v) = id_map.get(PAD) { ids.extend(v); }

    for ch in ipa.chars() {
        let s = ch.to_string();
        if let Some(v) = id_map.get(&s) {
            ids.extend(v);
            if ch == ' ' {
                if let Some(pad) = id_map.get(PAD) { ids.extend(pad); }
            }
        }
        // chars fora do map são ignorados silenciosamente
    }

    if let Some(v) = id_map.get(EOS) { ids.extend(v); }
    ids
}

/// Sintetiza um fragmento (sem pausa) → amostras i16.
pub fn sintetizar_fragmento(
    voice: &Voice,
    ipa: &str,
    length_scale: f32,
    sid: Option<i64>,
) -> Result<Vec<i16>, String> {
    if ipa.trim().is_empty() {
        return Ok(Vec::new());
    }
    let ids = ipa_para_ids(ipa, voice.id_map());
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let inf = &voice.cfg.inference;
    let scales = [inf.noise_scale, inf.length_scale * length_scale, inf.noise_w];

    let sid_final = if voice.num_speakers() > 1 { sid } else { None };
    let amostras_f32 = voice.model.infer(&ids, scales, sid_final)?;

    let i16s: Vec<i16> = amostras_f32.iter()
        .map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();
    Ok(i16s)
}