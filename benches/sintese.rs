//! Benchmarks — funções puras e síntese completa.
//!
//! Uso:
//!   cargo bench
//!   VOICES_DIR=./voices cargo bench   # inclui síntese ONNX

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::path::PathBuf;

use bcde_piper_vozz_rs::piper::{ipa_to_ids, VoiceManager};
use bcde_piper_vozz_rs::wav::{f32_to_i16, samples_to_wav_bytes};

fn bench_ipa_to_ids(c: &mut Criterion) {
    let mut id_map: HashMap<String, Vec<i64>> = HashMap::new();
    id_map.insert("^".to_string(), vec![1]);
    id_map.insert("$".to_string(), vec![2]);
    id_map.insert("_".to_string(), vec![0]);
    for (i, ch) in "ʊ xˈatʊ xoˈew a xˈowpɐ dʊ xˈeɪ dʒi xˈomɐ.,!?".chars().enumerate() {
        id_map.insert(ch.to_string(), vec![(i + 10) as i64]);
    }

    let ipa = "ʊ xˈatʊ xoˈew a xˈowpɐ dʊ xˈeɪ dʒi xˈomɐ.";

    c.bench_function("ipa_to_ids", |b| {
        b.iter(|| ipa_to_ids(black_box(ipa), &id_map))
    });
}

fn bench_wav_encoder(c: &mut Criterion) {
    let pcm: Vec<i16> = (0..44100).map(|i| ((i as f32 * 0.1).sin() * 16000.0) as i16).collect();

    c.bench_function("samples_to_wav_bytes (1s @ 22050)", |b| {
        b.iter(|| samples_to_wav_bytes(black_box(&pcm), 22050))
    });
}

fn bench_f32_to_i16(c: &mut Criterion) {
    let f32s: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.001).sin()).collect();

    c.bench_function("f32_to_i16 (44100 amostras)", |b| {
        b.iter(|| f32_to_i16(black_box(&f32s)))
    });
}

fn bench_sintese(c: &mut Criterion) {
    let voices_dir: PathBuf = std::env::var("VOICES_DIR")
        .map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("voices"));

    if !voices_dir.is_dir() {
        eprintln!("⚠️  {} não existe — pulando bench de síntese", voices_dir.display());
        return;
    }

    let mut mgr = match VoiceManager::load_from_dir(&voices_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("⚠️  não carregou vozes: {}", e);
            return;
        }
    };

    let nome = match mgr.list().first().cloned() {
        Some(n) => n,
        None => {
            eprintln!("⚠️  nenhuma voz em {}", voices_dir.display());
            return;
        }
    };

    let ipa = "ʊ xˈatʊ xoˈew a xˈowpɐ dʊ xˈeɪ dʒi xˈomɐ.";

    c.bench_function(&format!("sintese ONNX ({})", nome), |b| {
        b.iter(|| {
            let piper = mgr.get_mut(&nome).unwrap();
            let _ = piper.create(black_box(ipa), false, None, None, None, None);
        })
    });
}

criterion_group!(
    benches,
    bench_ipa_to_ids,
    bench_wav_encoder,
    bench_f32_to_i16,
    bench_sintese,
);
criterion_main!(benches);
