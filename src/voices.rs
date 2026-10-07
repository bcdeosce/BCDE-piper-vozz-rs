//! Scanner + carregador de vozes.
//!
//! Estrutura esperada:
//!   voices/<voz>/<voz>.onnx
//!   voices/<voz>/<voz>.onnx.json
//!   voices/<voz>/<voz>.config.json   (criado automaticamente se faltar)

use crate::config::VoiceConfig;
use crate::onnx::OnnxModel;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Deserialize, Debug, Clone)]
pub struct PiperModelConfig {
    pub audio: AudioCfg,
    pub inference: InferenceCfg,
    pub phoneme_id_map: HashMap<String, Vec<i64>>,
    #[serde(default)]
    pub num_speakers: u32,
    #[serde(default)]
    pub speaker_id_map: HashMap<String, u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AudioCfg {
    pub sample_rate: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InferenceCfg {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

pub struct Voice {
    pub name: String,
    pub dir: PathBuf,
    pub onnx_path: PathBuf,
    pub model_cfg_path: PathBuf,
    pub config_path: PathBuf,
    pub model: OnnxModel,
    pub cfg: PiperModelConfig,
    pub voice_config: VoiceConfig,
}

impl Voice {
    /// Carrega uma voz de `voices/<nome>/`.
    /// Se `<nome>.config.json` não existir, cria com os defaults.
    pub fn load(name: &str, dir: &Path) -> Result<Self, String> {
        let onnx_path = dir.join(format!("{}.onnx", name));
        let model_cfg_path = dir.join(format!("{}.onnx.json", name));
        let config_path = dir.join(format!("{}.config.json", name));

        if !onnx_path.is_file() {
            return Err(format!("faltando {}", onnx_path.display()));
        }
        if !model_cfg_path.is_file() {
            return Err(format!("faltando {}", model_cfg_path.display()));
        }

        let cfg_text = std::fs::read_to_string(&model_cfg_path)
            .map_err(|e| format!("ler {}: {}", model_cfg_path.display(), e))?;
        let cfg: PiperModelConfig = serde_json::from_str(&cfg_text)
            .map_err(|e| format!("parse {}: {}", model_cfg_path.display(), e))?;

        let model = OnnxModel::load(&onnx_path)?;

        let voice_config = if config_path.is_file() {
            VoiceConfig::load(&config_path)?
        } else {
            let c = VoiceConfig::default_para(name, cfg.audio.sample_rate);
            c.save(&config_path)?;
            c
        };

        Ok(Self {
            name: name.to_string(),
            dir: dir.to_path_buf(),
            onnx_path,
            model_cfg_path,
            config_path,
            model,
            cfg,
            voice_config,
        })
    }

    pub fn sample_rate(&self) -> u32 { self.cfg.audio.sample_rate }

    pub fn id_map(&self) -> &HashMap<String, Vec<i64>> { &self.cfg.phoneme_id_map }

    pub fn num_speakers(&self) -> u32 { self.cfg.num_speakers }
}

pub struct VoiceSet {
    pub root: PathBuf,
    pub voices: HashMap<String, Arc<Voice>>,
}

impl VoiceSet {
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut voices: HashMap<String, Arc<Voice>> = HashMap::new();
        if !root.is_dir() {
            return Err(format!("diretório de vozes inválido: {}", root.display()));
        }
        for entry in std::fs::read_dir(root)
            .map_err(|e| format!("ler {}: {}", root.display(), e))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_dir() { continue; }
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            match Voice::load(&name, &path) {
                Ok(v) => {
                    eprintln!("[voices] {} ({}, {} Hz, {} speakers)",
                        name, v.onnx_path.file_name().unwrap().to_string_lossy(),
                        v.sample_rate(), v.num_speakers());
                    voices.insert(name, Arc::new(v));
                }
                Err(e) => eprintln!("[voices] pulando {}: {}", name, e),
            }
        }
        Ok(Self { root: root.to_path_buf(), voices })
    }

    pub fn get(&self, name: &str) -> Option<Arc<Voice>> {
        self.voices.get(name).cloned()
    }

    pub fn list(&self) -> Vec<String> {
        let mut v: Vec<String> = self.voices.keys().cloned().collect();
        v.sort();
        v
    }
}