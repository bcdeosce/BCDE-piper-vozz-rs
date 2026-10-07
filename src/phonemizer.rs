//! Wrapper sobre a lib do vozz-g2p-rs. Tudo in-process.

use anyhow::{anyhow, Result};
use std::path::Path;

use vozz_g2p_rs::lexicon_homografos::LexiconHomografos;
use vozz_g2p_rs::piper_pipeline::Chunk;
use vozz_g2p_rs::pipeline::texto_para_chunks;
use vozz_g2p_rs::tagger::{self, Tagger};

pub struct Phonemizer {
    tagger: Tagger,
    homografos: Option<LexiconHomografos>,
}

impl Phonemizer {
    pub fn new(tagger_data: &str, lexicon_path: Option<&Path>) -> Result<Self> {
        let tagger = tagger::carregar_tagger(tagger_data)
            .map_err(|e| anyhow!("carregar tagger: {}", e))?;

        let homografos = match lexicon_path {
            Some(p) if p.is_file() => {
                let h = LexiconHomografos::from_path(p)
                    .map_err(|e| anyhow!("carregar lexicon: {}", e))?;
                Some(h)
            }
            _ => None,
        };

        Ok(Self { tagger, homografos })
    }

    pub fn process_chunks(&mut self, texto: &str) -> Result<Vec<Chunk>> {
        Ok(texto_para_chunks(
            texto,
            Some(&self.tagger),
            self.homografos.as_ref(),
            None,
            None,
        ))
    }
}
