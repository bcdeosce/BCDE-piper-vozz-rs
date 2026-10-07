use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bcde_piper_vozz_rs::dialog::Speaker;
use bcde_piper_vozz_rs::orchestrator::render_dialogo;
use bcde_piper_vozz_rs::phonemizer::Phonemizer;
use bcde_piper_vozz_rs::piper::VoiceManager;
use bcde_piper_vozz_rs::wav::{f32_to_i16, samples_to_wav_bytes};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const BUILD_ID: &str = "2024-11-bcde-piper-vozz-v7";

fn voices_dir() -> PathBuf {
    std::env::var("VOICES_DIR").map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("voices"))
}

fn empacotar(pcm: &[i16], sr: u32, extra: Value) -> Value {
    let wav = samples_to_wav_bytes(pcm, sr);
    let dir = std::env::var("AUDIO_CACHE_DIR")
        .unwrap_or_else(|_| "/tmp/bcde-piper".to_string());
    let _ = std::fs::create_dir_all(&dir);
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos()).unwrap_or(0);
    let path = format!("{}/{}.wav", dir, id);
    let mut obj = if std::fs::write(&path, &wav).is_ok() {
        json!({
            "sr": sr, "samples": pcm.len(),
            "duration_ms": (pcm.len() as u64 * 1000) / sr as u64,
            "audio_path": path, "audio_bytes": wav.len(),
        })
    } else {
        json!({
            "sr": sr, "samples": pcm.len(),
            "duration_ms": (pcm.len() as u64 * 1000) / sr as u64,
            "audio_wav_base64": STANDARD.encode(&wav),
        })
    };
    if let (Value::Object(em), Value::Object(om)) = (extra, &mut obj) {
        for (k, v) in em { om.insert(k, v); }
    }
    obj
}

/// Lê parâmetros opcionais. `false` ou ausente = ignora.
/// `len_scale` e `speed` são sinônimos.
fn params(req: &Value) -> (Option<f32>, Option<f32>, Option<f32>) {
    let get = |k: &str| -> Option<f32> {
        match req.get(k) {
            Some(Value::Number(n)) => n.as_f64().map(|x| x as f32),
            _ => None,
        }
    };
    let ls = get("len_scale").or_else(|| get("speed"));
    (ls, get("noise_scale"), get("noise_w"))
}

struct Estado { voices: VoiceManager, phonemizer: Option<Phonemizer> }

impl Estado {
    fn novo(root: PathBuf) -> Result<Self> {
        let voices = VoiceManager::load_from_dir(&root)?;
        let phonemizer = match std::env::var("BCDE_TAGGER_DATA") {
            Ok(data) if !data.is_empty() => {
                let lexico = std::env::var("LEXICON_HOMOGRAFOS").ok()
                    .map(PathBuf::from);
                match Phonemizer::new(&data, lexico.as_deref()) {
                    Ok(p) => { eprintln!("[worker] phonemizer ok"); Some(p) }
                    Err(e) => { eprintln!("[worker] phonemizer: {}", e); None }
                }
            }
            _ => { eprintln!("[worker] BCDE_TAGGER_DATA ausente"); None }
        };
        Ok(Self { voices, phonemizer })
    }
}

fn h_synthesize(estado: &mut Estado, req: &Value) -> Result<Value> {
    let name = req.get("voice").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("voice"))?;
    let ipa = req.get("ipa").and_then(|v| v.as_str()).unwrap_or("");
    let sid = req.get("sid").and_then(|v| v.as_i64());
    let (ls, ns, nw) = params(req);

    let piper = estado.voices.get_mut(name)
        .ok_or_else(|| anyhow!("voz: {}", name))?;
    let (f32s, sr) = piper.create(ipa, false, sid, ls, ns, nw)?;
    Ok(empacotar(&f32_to_i16(&f32s), sr, json!({"voice": name})))
}

fn h_synthesize_dialog(estado: &mut Estado, req: &Value) -> Result<Value> {
    let ph = estado.phonemizer.as_mut()
        .ok_or_else(|| anyhow!("BCDE_TAGGER_DATA não configurado"))?;
    let texto = req.get("text").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("text"))?;
    let voice_default = req.get("voice").and_then(|v| v.as_str())
        .unwrap_or("faber-medium");
    let pausa_entre_ms = req.get("pausa_entre_ms")
        .and_then(|v| v.as_u64()).unwrap_or(400) as u32;
    let speakers: Vec<Speaker> = req.get("speakers").cloned()
        .map(serde_json::from_value).transpose()?.unwrap_or_default();
    let (ls, ns, nw) = params(req);

    let r = render_dialogo(
        &mut estado.voices, ph, texto, voice_default,
        &speakers, pausa_entre_ms, ls, ns, nw,
    )?;
    Ok(empacotar(&r.pcm, r.sr,
        json!({"mode":"dialog","segments": r.segmentos})))
}

fn h_list_voices(voices: &VoiceManager) -> Value {
    let lista: Vec<Value> = voices.list().iter().map(|n| {
        let v = voices.get(n).unwrap();
        json!({
            "name": n,
            "sr": v.sample_rate(),
            "num_speakers": v.num_speakers(),
        })
    }).collect();
    json!({"voices": lista})
}

fn main() -> Result<()> {
    let root = voices_dir();
    eprintln!("[worker] vozes: {}", root.display());
    let mut estado = match Estado::novo(root.clone()) {
        Ok(e) => e,
        Err(e) => { eprintln!("[worker] {}", e); std::process::exit(1); }
    };
    eprintln!("[worker] {} vozes", estado.voices.len());

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        if line.trim().is_empty() { continue; }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(out, "{}", json!({"error": e.to_string()}));
                let _ = out.flush(); continue;
            }
        };
        let action = req.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let resp: Result<Value> = match action {
            "version" => Ok(json!({
                "build": BUILD_ID,
                "features": ["list_voices","reload_voices","synthesize","synthesize_dialog"],
                "has_phonemizer": estado.phonemizer.is_some(),
            })),
            "list_voices" => Ok(h_list_voices(&estado.voices)),
            "reload_voices" => match VoiceManager::load_from_dir(&root) {
                Ok(v) => { let n = v.list(); estado.voices = v;
                    Ok(json!({"ok":true,"voices":n})) }
                Err(e) => Err(e),
            },
            "synthesize"        => h_synthesize(&mut estado, &req),
            "synthesize_dialog" => h_synthesize_dialog(&mut estado, &req),
            other => Err(anyhow!("ação desconhecida: {}", other)),
        };
        let texto = match resp {
            Ok(v) => v.to_string(),
            Err(e) => json!({"error": e.to_string()}).to_string(),
        };
        let _ = writeln!(out, "{}", texto);
        let _ = out.flush();
    }
    Ok(())
}
