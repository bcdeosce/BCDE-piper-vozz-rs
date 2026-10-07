//! Parser de `[role]` inline.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Speaker {
    pub role: String,
    pub voice: String,
    #[serde(default)] pub sid: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub role: Option<String>,
    pub text: String,
    pub voice: String,
    pub sid: Option<i64>,
}

pub fn parse_dialogo(texto: &str, voice_default: &str, speakers: &[Speaker])
    -> (Vec<Segment>, usize)
{
    let map_role: HashMap<&str, &Speaker> = speakers.iter()
        .map(|s| (s.role.as_str(), s)).collect();

    let mut segmentos = Vec::new();
    let mut role_atual: Option<String> = None;
    let mut buffer = String::new();
    let mut ignoradas = 0usize;

    let chars: Vec<char> = texto.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(rel) = chars[i..].iter().position(|&c| c == ']') {
                let tag = chars[i+1..i+rel].iter().collect::<String>()
                    .trim().to_lowercase();
                let buf_trim = buffer.trim();
                if !buf_trim.is_empty() {
                    segmentos.push(montar(role_atual.as_deref(),
                        buf_trim, voice_default, &map_role));
                }
                buffer.clear();
                if map_role.contains_key(tag.as_str()) {
                    role_atual = Some(tag);
                } else {
                    ignoradas += 1;
                }
                i += rel + 1;
                continue;
            }
        }
        buffer.push(chars[i]);
        i += 1;
    }
    let buf_trim = buffer.trim();
    if !buf_trim.is_empty() {
        segmentos.push(montar(role_atual.as_deref(),
            buf_trim, voice_default, &map_role));
    }
    (segmentos, ignoradas)
}

fn montar(role: Option<&str>, texto: &str,
    voice_default: &str, map: &HashMap<&str, &Speaker>) -> Segment
{
    let spk = role.and_then(|r| map.get(r).copied());
    Segment {
        role: role.map(String::from),
        text: texto.to_string(),
        voice: spk.map(|s| s.voice.clone()).unwrap_or_else(|| voice_default.to_string()),
        sid: spk.and_then(|s| s.sid),
    }
}
