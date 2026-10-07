//! Exemplo mínimo de síntese.
//!
//! Uso:
//!   VOICES_DIR=./voices \
//!   BCDE_TAGGER_DATA=./BCDE-tagger/data \
//!     cargo run --example sintetizar -- "Bom dia, doutor!"

use bcde_piper_vozz_rs::{
    orchestrator::render_dialogo,
    phonemizer::Phonemizer,
    piper::VoiceManager,
    dialog::Speaker,
    wav::samples_to_wav_bytes,
};
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let texto = std::env::args().nth(1)
        .unwrap_or_else(|| "Bom dia, como está se sentindo hoje?".to_string());

    let voices_dir: PathBuf = std::env::var("VOICES_DIR")
        .map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("voices"));

    let tagger_data = std::env::var("BCDE_TAGGER_DATA")
        .unwrap_or_else(|_| "BCDE-tagger/data".to_string());

    println!("Carregando vozes de {}...", voices_dir.display());
    let mut voices = VoiceManager::load_from_dir(&voices_dir)?;

    let voz_default = voices.list().first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("nenhuma voz em {}", voices_dir.display()))?;
    println!("Usando voz: {}", voz_default);

    println!("Carregando tagger de {}...", tagger_data);
    let mut ph = Phonemizer::new(&tagger_data, None)?;

    let speakers: Vec<Speaker> = voices.list().iter().take(2).map(|n| Speaker {
        role: n.clone(),
        voice: n.clone(),
        sid: None,
    }).collect();

    println!("Sintetizando: {:?}", texto);
    let r = render_dialogo(
        &mut voices,
        &mut ph,
        &texto,
        &voz_default,
        &speakers,
        400,      // pausa entre segmentos
        None,     // length_scale
        None,     // noise_scale
        None,     // noise_w
    )?;

    let wav = samples_to_wav_bytes(&r.pcm, r.sr);
    let saida = Path::new("saida.wav");
    std::fs::write(saida, wav)?;

    println!(
        "✅ {} segmentos | {:.2}s @ {} Hz | salvo em {}",
        r.segmentos,
        r.pcm.len() as f64 / r.sr as f64,
        r.sr,
        saida.display()
    );
    Ok(())
}
