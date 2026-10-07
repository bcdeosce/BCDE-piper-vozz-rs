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


#[cfg(test)]
mod testes {
    use super::*;

    fn speakers() -> Vec<Speaker> {
        vec![
            Speaker { role: "medico".into(),   voice: "voz_a".into(), sid: None },
            Speaker { role: "paciente".into(), voice: "voz_b".into(), sid: None },
        ]
    }

    #[test]
    fn sem_tags_usa_default() {
        let (segs, ign) = parse_dialogo("Bom dia.", "voz_default", &speakers());
        assert_eq!(ign, 0);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].voice, "voz_default");
        assert_eq!(segs[0].text, "Bom dia.");
        assert!(segs[0].role.is_none());
    }

    #[test]
    fn uma_tag() {
        let (segs, _) = parse_dialogo(
            "[medico] Bom dia.", "voz_default", &speakers());
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].voice, "voz_a");
        assert_eq!(segs[0].role.as_deref(), Some("medico"));
        assert_eq!(segs[0].text, "Bom dia.");
    }

    #[test]
    fn duas_tags() {
        let (segs, _) = parse_dialogo(
            "[medico] Olá. [paciente] Dói muito.",
            "voz_default", &speakers());
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].voice, "voz_a");
        assert_eq!(segs[1].voice, "voz_b");
    }

    #[test]
    fn tag_desconhecida_ignorada() {
        let (segs, ign) = parse_dialogo(
            "[xyz] texto qualquer",
            "voz_default", &speakers());
        assert_eq!(ign, 1);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].voice, "voz_default");
    }

    #[test]
    fn retorno_ao_role_anterior() {
        let (segs, _) = parse_dialogo(
            "[medico] A. [paciente] B. [medico] C.",
            "voz_default", &speakers());
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].voice, "voz_a");
        assert_eq!(segs[1].voice, "voz_b");
        assert_eq!(segs[2].voice, "voz_a");
    }

    #[test]
    fn texto_vazio() {
        let (segs, _) = parse_dialogo("", "voz_default", &speakers());
        assert!(segs.is_empty());
    }
}
