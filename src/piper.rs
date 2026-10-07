use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::onnx::OnnxModel;

#[derive(Debug, Clone, Deserialize)]
pub struct PiperModelConfig {
    pub audio: AudioCfg,
    pub inference: InferenceCfg,
    pub phoneme_id_map: HashMap<String, Vec<i64>>,
    #[serde(default)] pub num_speakers: u32,
    #[serde(default)] pub speaker_id_map: HashMap<String, u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioCfg { pub sample_rate: u32 }

#[derive(Debug, Clone, Deserialize)]
pub struct InferenceCfg {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

fn eh_ignoravel(c: char) -> bool {
    c == '\u{200d}' || c == '\u{0361}'
}

pub fn ipa_to_ids(ipa: &str, id_map: &HashMap<String, Vec<i64>>) -> Vec<i64> {
    let mut ids: Vec<i64> = Vec::with_capacity(ipa.len() * 3);
    if let Some(v) = id_map.get("^") { ids.extend(v); }
    for ch in ipa.chars() {
        if eh_ignoravel(ch) { continue; }
        let s = ch.to_string();
        if let Some(v) = id_map.get(&s) {
            ids.extend(v);
            if let Some(p) = id_map.get("_") { ids.extend(p); }
            if matches!(ch, ',' | ';' | ':' | '.' | '!' | '?' | '…') {
                if let Some(p) = id_map.get("_") { ids.extend(p); }
            }
        }
    }
    if let Some(v) = id_map.get("$") { ids.extend(v); }
    ids
}

pub struct Piper {
    pub name: String,
    pub dir: PathBuf,
    pub onnx_path: PathBuf,
    pub model_cfg_path: PathBuf,
    pub model_cfg: PiperModelConfig,
    model: OnnxModel,
}

impl Piper {
    pub fn new(onnx_path: &Path, config_path: &Path) -> Result<Self> {
        let dir = onnx_path.parent()
            .ok_or_else(|| anyhow!("onnx sem parent"))?.to_path_buf();
        let name = onnx_path.file_stem().and_then(|s| s.to_str())
            .unwrap_or("voice").to_string();

        let cfg_text = std::fs::read_to_string(config_path)
            .with_context(|| format!("ler {}", config_path.display()))?;
        let model_cfg: PiperModelConfig = serde_json::from_str(&cfg_text)
            .with_context(|| format!("parse {}", config_path.display()))?;

        let model = OnnxModel::load(onnx_path, model_cfg.num_speakers)?;

        Ok(Self { name, dir, onnx_path: onnx_path.into(),
            model_cfg_path: config_path.into(), model_cfg, model })
    }

    pub fn sample_rate(&self) -> u32 { self.model_cfg.audio.sample_rate }
    pub fn num_speakers(&self) -> u32 { self.model_cfg.num_speakers }
    pub fn id_map(&self) -> &HashMap<String, Vec<i64>> { &self.model_cfg.phoneme_id_map }

    pub fn create(&mut self, ipa: &str, _use_espeak: bool, speaker_id: Option<i64>,
        length_scale: Option<f32>, noise_scale: Option<f32>, noise_w: Option<f32>)
        -> Result<(Vec<f32>, u32)>
    {
        let ids = ipa_to_ids(ipa, self.id_map());
        if ids.is_empty() { return Ok((Vec::new(), self.sample_rate())); }

        let inf = &self.model_cfg.inference;
        let scales = [
            noise_scale.unwrap_or(inf.noise_scale),
            length_scale.unwrap_or(1.0),
            noise_w.unwrap_or(inf.noise_w),
        ];
        let amostras = self.model.infer(&ids, scales, speaker_id)?;
        Ok((amostras, self.sample_rate()))
    }
}

pub struct VoiceManager {
    pub voices: HashMap<String, Piper>,
    pub root: PathBuf,
}

impl VoiceManager {
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        if !dir.is_dir() { return Err(anyhow!("dir inválido: {}", dir.display())); }
        let mut voices = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            let e = entry?;
            let path = e.path();
            if !path.is_dir() { continue; }
            let name = match e.file_name().to_str() {
                Some(s) => s.to_string(), None => continue,
            };
            match carregar_par(&path, &name) {
                Ok((onnx, cfg)) => match Piper::new(&onnx, &cfg) {
                    Ok(p) => {
                        tracing::info!("[voices] {} ({} Hz, {} speakers)",
                            name, p.sample_rate(), p.num_speakers());
                        voices.insert(name, p);
                    }
                    Err(err) => tracing::warn!("[voices] {}: {}", name, err),
                },
                Err(err) => tracing::warn!("[voices] {}: {}", name, err),
            }
        }
        Ok(Self { voices, root: dir.to_path_buf() })
    }
    pub fn get(&self, n: &str) -> Option<&Piper> { self.voices.get(n) }
    pub fn get_mut(&mut self, n: &str) -> Option<&mut Piper> { self.voices.get_mut(n) }
    pub fn list(&self) -> Vec<String> {
        let mut v: Vec<_> = self.voices.keys().cloned().collect();
        v.sort(); v
    }
    pub fn len(&self) -> usize { self.voices.len() }
}

fn carregar_par(dir: &Path, name: &str) -> Result<(PathBuf, PathBuf)> {
    let onnx_pref = dir.join(format!("{}.onnx", name));
    let cfg_pref = dir.join(format!("{}.onnx.json", name));
    if onnx_pref.is_file() && cfg_pref.is_file() { return Ok((onnx_pref, cfg_pref)); }
    let mut onnx = None; let mut cfg = None;
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        let s = p.to_string_lossy();
        if s.ends_with(".onnx.json") { cfg = Some(p); }
        else if s.ends_with(".onnx") { onnx = Some(p); }
    }
    match (onnx, cfg) {
        (Some(o), Some(c)) => Ok((o, c)),
        _ => Err(anyhow!("sem .onnx/.onnx.json em {}", dir.display())),
    }
}
