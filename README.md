```markdown
# BCDE-piper-vozz-rs

Motor de síntese **Piper (ONNX)** para pt-BR, com fonemização pelo [`vozz-g2p-rs`](https://github.com/bcdeosce/vozz-g2p-rs) — biblioteca de G2P com desambiguação de homógrafos via [`BCDE-tagger`](https://github.com/bcdeosce/BCDE-tagger).

Parte do **ecossistema BCDE** para processamento de fala em português brasileiro, distribuído sob **MIT**.

---

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![ONNX Runtime](https://img.shields.io/badge/onnxruntime-1.20%2B-blueviolet.svg)]()

## Índice

- [O que é](#o-que-é)
- [Motivação](#motivação)
- [Ecossistema BCDE](#ecossistema-bcde)
- [Como funciona](#como-funciona)
- [Instalação](#instalação)
- [Estrutura do projeto](#estrutura-do-projeto)
- [Vozes](#vozes)
- [Uso como biblioteca Rust](#uso-como-biblioteca-rust)
- [Uso como worker binário](#uso-como-worker-binário)
- [Payloads](#payloads)
- [Exemplos](#exemplos)
- [Benchmarks](#benchmarks)
- [Testes](#testes)
- [API reference](#api-reference)
- [Design decisions](#design-decisions)
- [Licença](#licença)
- [Créditos](#créditos)

---

## O que é

O `BCDE-piper-vozz-rs` é um **motor de síntese de fala** que:

- Carrega modelos **Piper ONNX** (vozes pt-BR)
- Fonemiza via **`vozz-g2p-rs`** (com desambiguação de homógrafos via BCDE-tagger)
- Reconstrói a IPA **com pontuação** e envia ao modelo ONNX
- Suporta **múltiplas vozes** com sample rates distintos (resample automático em diálogo)
- Funciona **in-process** (biblioteca Rust) ou como **worker persistente** (JSON stdin/stdout)
- É **single-thread por worker** — escala via multiprocessamento, não threading

Distribuído como **biblioteca** (`bcde_piper_vozz_rs`) e **binário** (`bcde-piper-worker`).

**Licença:** MIT. Sem dependências copyleft. Sem runtime externo. Sem dependência de Python, Node ou eSpeak em produção.

---

## Motivação

O objetivo deste projeto é fornecer um **motor de síntese pt-BR com licenciamento permissivo**, livre de restrições que dificultam uso comercial, redistribuição em produtos proprietários ou integração em infraestruturas fechadas.

Três frentes:

1. **Licenciamento permissivo.** Todo o ecossistema BCDE é distribuído sob **MIT** ou **Apache 2.0** — as duas licenças mais permissivas em uso corrente. Qualquer pessoa pode usar, modificar, redistribuir, vender, embutir em produtos comerciais, sem obrigação de abrir código-fonte, sem copyleft, sem cláusulas restritivas. O motor é MIT puro, e as dependências diretas são MIT ou Apache 2.0.

2. **Independência de runtime externo.** A pilha não depende de Python, Node, eSpeak ou qualquer binário externo em runtime. É Rust puro + ONNX Runtime. Um único binário estático, sem `pip install`, sem container pesado, sem dependências transitivas em infraestrutura de produção.

3. **Fonemização de qualidade.** O G2P pt-BR tem regras complexas (nasalização, sândi, ditongos, trema, desambiguação de homógrafos). O `vozz-g2p-rs` resolve isso com qualidade de nível espeak-ng — melhor em casos como `sede` (seat/thirst), `cinquenta` (trema), `os animais` (sândi). E é MIT/Apache 2.0 puro.

O resultado é um motor que **roda em qualquer lugar, com qualquer licença, em qualquer produto**, sem contaminação por licenças restritivas.

---

## Ecossistema BCDE

```
┌──────────────────────────────────────────────────────────────────┐
│  Aplicação cliente (API, CLI, app)                               │
└────────────────────────────┬─────────────────────────────────────┘
                             │ JSON
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│  bcde-piper-worker (este projeto)                                │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Parser de [role]                                          │  │
│  └────────────────────┬───────────────────────────────────────┘  │
│                       │                                          │
│  ┌────────────────────▼───────────────────────────────────────┐  │
│  │  vozz-g2p-rs (lib)                                         │  │
│  │  ├─ normalize      (datas, números, moedas, siglas)        │  │
│  │  ├─ splitter       (sentenças)                             │  │
│  │  ├─ BCDE-tagger    (POS tagging + desambiguação)           │  │
│  │  └─ g2p            (regras fonológicas)                    │  │
│  └────────────────────┬───────────────────────────────────────┘  │
│                       │ IPA Piper                                │
│  ┌────────────────────▼───────────────────────────────────────┐  │
│  │  ONNX Runtime (1 thread, 1 CPU)                            │  │
│  │  síntese por chunk (uma inferência por chunk)              │  │
│  └────────────────────┬───────────────────────────────────────┘  │
│                       │ PCM i16                                  │
│  ┌────────────────────▼───────────────────────────────────────┐  │
│  │  Orchestrator                                              │  │
│  │  (resample + concat + pausa entre segmentos)               │  │
│  └────────────────────┬───────────────────────────────────────┘  │
│                       │ WAV                                       │
└───────────────────────┼──────────────────────────────────────────┘
                        ▼
                     Cliente
```

**Projetos do ecossistema:**

| Projeto | Papel | Licença |
|---------|-------|:-:|
| [`BCDE-tagger`](https://github.com/bcdeosce/BCDE-tagger) | POS tagging + desambiguação de homógrafos | MIT |
| [`vozz-g2p-rs`](https://github.com/bcdeosce/vozz-g2p-rs) | G2P pt-BR (grafema → fonema) | Apache 2.0 |
| **`BCDE-piper-vozz-rs`** | Motor de síntese (este projeto) | MIT |

---

## Como funciona

### Pipeline interno

Para cada chamada `synthesize_dialog`:

1. **Parser** quebra o texto em segmentos por `[role]`
2. Para cada segmento, o **vozz-g2p-rs** (lib) retorna os chunks fonemizados:
   - Normaliza (datas, números, moedas)
   - Divide em sentenças
   - Aplica BCDE-tagger para desambiguação (ex.: `sede` seat vs thirst)
   - Roda o G2P (nasalização, sândi, ditongos, trema)
   - Converte para o alfabeto Piper
   - Fragmenta em chunks
3. Cada chunk é sintetizado com **uma única inferência ONNX**
4. Se há múltiplas vozes com sample rates diferentes, **resample linear** para o sr do primeiro
5. Segmentos são concatenados com pausa fixa entre eles (`pausa_entre_ms`)
6. Devolve WAV 16-bit mono

### Sobre a IPA

Enviar o texto direto ao modelo não funciona bem porque a fonemização interna do runtime não trata corretamente as regras específicas do pt-BR (trema, sândi, homógrafos). Pré-fonemizamos com o vozz e entregamos **IPA com pontuação** ao ONNX.

O modelo Piper foi treinado com `PAD` após cada phoneme e `PAD PAD` após pontuação. Reproduzimos esse padrão em `ipa_to_ids`. Resultado: o modelo gera pausas naturalmente — sem inserir silêncio manual.

### Sobre escalabilidade

Cada worker roda com `intra_threads=1` e `inter_threads=1`. Isso é intencional:

- Um worker por CPU física
- N workers paralelos = N CPUs trabalhando em paralelo
- Latência p95 muito menor que threading ONNX
- Escalabilidade linear até o número de CPUs

---

## Instalação

### Como biblioteca (Cargo.toml)

```toml
[dependencies]
bcde-piper-vozz-rs = { git = "https://github.com/bcdeosce/BCDE-piper-vozz-rs" }
```

### Como binário

```bash
git clone https://github.com/bcdeosce/BCDE-piper-vozz-rs
cd BCDE-piper-vozz-rs

# Baixa as vozes pt-BR (faber-low, faber-medium, edresson-low, cadu-medium)
./scripts/baixar_vozes.sh

# Compila
cargo build --release
```

O binário fica em `target/release/bcde-piper-worker`.

### Requisitos

- Rust 1.75+ (versão pinada em `rust-toolchain.toml`)
- ONNX Runtime 1.20+ (baixado automaticamente pelo `ort-sys`)

---

## Estrutura do projeto

```
BCDE-piper-vozz-rs/
├── Cargo.toml
├── Cargo.lock
├── LICENSE                    (MIT)
├── README.md
├── rust-toolchain.toml        (pina stable + rustfmt + clippy)
├── rustfmt.toml
├── .gitignore
├── src/
│   ├── lib.rs                 (reexports)
│   ├── piper.rs               (Piper + VoiceManager + ipa_to_ids)
│   ├── onnx.rs                (wrapper ort)
│   ├── wav.rs                 (encoder WAV, f32→i16)
│   ├── phonemizer.rs          (wrapper do vozz-g2p-rs)
│   ├── dialog.rs              (parser de [role])
│   ├── orchestrator.rs        (render_dialogo + resample)
│   └── bin/
│       └── piper-worker.rs    (worker JSON stdin/stdout)
├── examples/
│   └── sintetizar.rs          (exemplo mínimo)
├── benches/
│   └── sintese.rs             (Criterion benchmarks)
├── scripts/
│   └── baixar_vozes.sh        (download de vozes do HuggingFace)
└── voices/
    └── .gitkeep               (vazia; modelos não versionados)
```

---

## Vozes

O worker procura as vozes em `$VOICES_DIR` (default: `./voices`). Cada voz é uma subpasta com dois arquivos:

```
voices/
├── faber-medium/
│   ├── faber-medium.onnx              ← modelo ONNX
│   └── faber-medium.onnx.json         ← config do modelo (sr, phoneme_id_map)
├── edresson-low/
│   ├── edresson-low.onnx
│   └── edresson-low.onnx.json
└── cadu-medium/
    ├── cadu-medium.onnx
    └── cadu-medium.onnx.json
```

Só isso. Sem arquivos de config extra, sem JSON de metadados.

### Download automático

O script `scripts/baixar_vozes.sh` baixa as vozes pt-BR oficiais do repositório [`rhasspy/piper-voices`](https://huggingface.co/rhasspy/piper-voices) no HuggingFace.

```bash
# Baixa as vozes padrão
./scripts/baixar_vozes.sh

# Baixa vozes específicas
./scripts/baixar_vozes.sh faber-medium edresson-low

# Diretório customizado
VOICES_DIR=/outro/dir ./scripts/baixar_vozes.sh
```

Vozes padrão que o script baixa:

| Voz | sr | Tamanho | Qualidade |
|-----|:-:|:-:|-----------|
| `faber-low` | 16000 | ~20 MB | Baixa |
| `faber-medium` | 22050 | ~63 MB | Média |
| `edresson-low` | 16000 | ~20 MB | Baixa |
| `cadu-medium` | 22050 | ~63 MB | Média |

Qualquer modelo Piper no formato `ONNX + .onnx.json` é aceito — basta colocar a pasta em `voices/`.

---

## Uso como biblioteca Rust

### Exemplo — diálogo completo

```rust
use bcde_piper_vozz_rs::{
    orchestrator::render_dialogo,
    phonemizer::Phonemizer,
    piper::VoiceManager,
    dialog::Speaker,
    wav::samples_to_wav_bytes,
};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Carrega todas as vozes de `voices/`
    let mut voices = VoiceManager::load_from_dir(Path::new("voices"))?;

    // Carrega o tagger + homógrafos do vozz (uma vez)
    let mut ph = Phonemizer::new(
        "BCDE-tagger/data",
        Some(Path::new("lexicon_homografos.json")),
    )?;

    let texto = "[medico] Bom dia, como está se sentindo? [paciente] Doutor, estou com dor de cabeça.";

    let speakers = vec![
        Speaker { role: "medico".into(),   voice: "faber-medium".into(), sid: None },
        Speaker { role: "paciente".into(), voice: "edresson-low".into(), sid: None },
    ];

    let r = render_dialogo(
        &mut voices,
        &mut ph,
        texto,
        "faber-medium",   // voz default
        &speakers,
        400,              // pausa entre segmentos, ms
        None,             // length_scale
        None,             // noise_scale
        None,             // noise_w
    )?;

    let wav = samples_to_wav_bytes(&r.pcm, r.sr);
    std::fs::write("saida.wav", wav)?;

    println!("{} segmentos | {} ms", r.segmentos, r.pcm.len() * 1000 / r.sr as usize);
    Ok(())
}
```

### Exemplo — síntese direta (IPA já pronta)

```rust
use bcde_piper_vozz_rs::piper::VoiceManager;
use bcde_piper_vozz_rs::wav::{f32_to_i16, samples_to_wav_bytes};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let mut mgr = VoiceManager::load_from_dir(Path::new("voices"))?;
    let piper = mgr.get_mut("faber-medium").unwrap();

    let ipa = "ʊ xˈatʊ xoˈew a xˈowpɐ";
    let (f32s, sr) = piper.create(
        ipa,
        false,       // use_espeak — sempre ignorado (sempre IPA direta)
        None,        // speaker_id
        None,        // length_scale (default 1.0)
        None,        // noise_scale (default do modelo)
        None,        // noise_w (default do modelo)
    )?;

    std::fs::write("out.wav", samples_to_wav_bytes(&f32_to_i16(&f32s), sr))?;
    Ok(())
}
```

### Exemplo — carregar uma voz específica por path

```rust
use bcde_piper_vozz_rs::piper::Piper;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let mut piper = Piper::new(
        Path::new("/caminho/custom/voz.onnx"),
        Path::new("/caminho/custom/voz.onnx.json"),
    )?;

    let (amostras, sr) = piper.create("bˈõ dʒiɐ", false, None, None, None, None)?;
    println!("{} amostras @ {} Hz", amostras.len(), sr);
    Ok(())
}
```

---

## Uso como worker binário

O `bcde-piper-worker` é um **processo persistente**: lê um JSON por linha do `stdin`, escreve um JSON por linha no `stdout`. Desenhado para rodar 1 worker por CPU física.

### Variáveis de ambiente

| Variável | Obrigatória | Descrição |
|----------|:-:|-----------|
| `VOICES_DIR` | sim | Pasta com as vozes |
| `BCDE_TAGGER_DATA` | para `synthesize_dialog` | Pasta `data/` do BCDE-tagger |
| `LEXICON_HOMOGRAFOS` | opcional | Caminho para `lexicon_homografos.json` |
| `AUDIO_CACHE_DIR` | opcional | Onde salvar WAVs (default: `/tmp/bcde-piper`) |

### Iniciar

```bash
export VOICES_DIR=/caminho/para/voices
export BCDE_TAGGER_DATA=/caminho/para/BCDE-tagger/data

./target/release/bcde-piper-worker
```

### Multiprocessamento

Padrão recomendado: N workers paralelos, um por CPU física.

```bash
# 8 CPUs físicas
for i in $(seq 1 8); do
  VOICES_DIR=/caminho/voices \
  BCDE_TAGGER_DATA=/caminho/data \
  ./bcde-piper-worker &
done
```

Cada worker carrega todas as vozes em memória. Se você tem 5 vozes de ~60MB, isso é ~300MB por worker. Com 8 workers, ~2.4GB total.

---

## Payloads

### `version`

```json
{"action":"version"}
```

```json
{
  "build": "2024-11-bcde-piper-vozz-v7",
  "features": ["list_voices","reload_voices","synthesize","synthesize_dialog"],
  "has_phonemizer": true
}
```

`has_phonemizer` indica se o `BCDE_TAGGER_DATA` foi encontrado e o tagger carregado. Se `false`, apenas `synthesize` funciona.

### `list_voices`

```json
{"action":"list_voices"}
```

```json
{
  "voices": [
    {"name":"faber-medium","sr":22050,"num_speakers":1},
    {"name":"edresson-low","sr":16000,"num_speakers":1}
  ]
}
```

### `reload_voices`

Recarrega as vozes do disco. Útil depois de adicionar/remover vozes.

```json
{"action":"reload_voices"}
```

```json
{"ok": true, "voices": ["faber-medium", "edresson-low"]}
```

### `synthesize` — IPA direta

Sintetiza uma string IPA. Sem parser, sem phonemizer.

```json
{
  "action": "synthesize",
  "voice": "faber-medium",
  "ipa": "ʊ xˈatʊ xoˈew a xˈowpɐ",
  "sid": null,
  "len_scale": 1.2,
  "noise_scale": 0.667,
  "noise_w": 0.8
}
```

Todos os campos numéricos são opcionais. Aceita `len_scale` ou `speed` como sinônimos.

Resposta:

```json
{
  "sr": 22050,
  "samples": 46280,
  "duration_ms": 2099,
  "audio_path": "/tmp/bcde-piper/1730000000000000000.wav",
  "audio_bytes": 92560,
  "voice": "faber-medium"
}
```

**Nota:** o áudio é devolvido como **caminho de arquivo** (`audio_path`), não base64. Isso evita trafegar ~180KB de base64 por chamada. Se o disco falhar, cai automaticamente para `audio_wav_base64`.

### `synthesize_dialog` — texto com tags

A ação principal. Aceita texto com tags `[role]`, fonemiza via vozz-g2p-rs e devolve um WAV único.

```json
{
  "action": "synthesize_dialog",
  "voice": "faber-medium",
  "text": "[medico] Bom dia, como está se sentindo? [paciente] Doutor, estou com dor de cabeça.",
  "speakers": [
    {"role": "medico",   "voice": "faber-medium"},
    {"role": "paciente", "voice": "edresson-low"}
  ],
  "pausa_entre_ms": 400,
  "len_scale": 1.0,
  "noise_scale": 0.667,
  "noise_w": 0.8
}
```

Resposta:

```json
{
  "sr": 22050,
  "samples": 98420,
  "duration_ms": 4463,
  "audio_path": "/tmp/bcde-piper/1730000000000000001.wav",
  "audio_bytes": 196840,
  "mode": "dialog",
  "segments": 2
}
```

### Parser de tags

O parser reconhece uma tag inline:

- **`[role]`** — define o speaker atual. A `voice` do role é resolvida via `speakers[]`.

**Regras:**
- Texto antes de qualquer tag usa `voice` do payload
- Tag desconhecida é ignorada (não quebra o parser)
- Trocar de role muda a voz do segmento seguinte

**Exemplo:**

```
[medico] Bom dia. [paciente] Dói muito! [medico] Vou examinar.
```

Vira 3 segmentos:

| # | role | text | voice |
|---|------|------|-------|
| 0 | medico | "Bom dia." | faber-medium |
| 1 | paciente | "Dói muito!" | edresson-low |
| 2 | medico | "Vou examinar." | faber-medium |

### Resample automático

Vozes diferentes têm sample rates diferentes:

| Voz | sr |
|-----|:-:|
| `faber-medium` | 22050 |
| `edresson-low` | 16000 |

Em diálogo com vozes mistas, o orquestrador **resampleia linearmente** para o sr do primeiro segmento. O WAV final é uniforme.

### Cliente Python

```python
import subprocess, json, base64
from pathlib import Path
from IPython.display import Audio

class BcdePiper:
    def __init__(self, bin_path, voices_dir, tagger_data):
        import os
        env = {**os.environ,
               "VOICES_DIR": voices_dir,
               "BCDE_TAGGER_DATA": tagger_data}
        self.proc = subprocess.Popen(
            [bin_path],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1, env=env)
        self._req({"action": "version"})

    def _req(self, obj):
        self.proc.stdin.write(json.dumps(obj) + "\n")
        self.proc.stdin.flush()
        return json.loads(self.proc.stdout.readline())

    def sintetizar(self, texto, voice="faber-medium",
                   speakers=None, pausa_ms=400,
                   len_scale=None, noise_scale=None, noise_w=None):
        payload = {
            "action": "synthesize_dialog",
            "voice": voice,
            "text": texto,
            "speakers": speakers or [],
            "pausa_entre_ms": pausa_ms,
        }
        if len_scale is not None: payload["len_scale"] = len_scale
        if noise_scale is not None: payload["noise_scale"] = noise_scale
        if noise_w is not None: payload["noise_w"] = noise_w

        r = self._req(payload)
        if "error" in r:
            raise RuntimeError(r["error"])
        if "audio_path" in r:
            return Path(r["audio_path"]).read_bytes()
        return base64.b64decode(r["audio_wav_base64"])

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=3)

# Uso
piper = BcdePiper(
    bin_path="./target/release/bcde-piper-worker",
    voices_dir="./voices",
    tagger_data="./BCDE-tagger/data",
)

wav = piper.sintetizar(
    "[medico] Bom dia, como está se sentindo? [paciente] Estou com dor.",
    voice="faber-medium",
    speakers=[
        {"role": "medico",   "voice": "faber-medium"},
        {"role": "paciente", "voice": "edresson-low"},
    ],
    pausa_ms=400,
    len_scale=1.0,
)

with open("saida.wav", "wb") as f:
    f.write(wav)

Audio(wav)
```

---

## Exemplos

O repositório vem com um exemplo mínimo em `examples/sintetizar.rs`:

```bash
# Baixa as vozes primeiro
./scripts/baixar_vozes.sh

# Roda o exemplo
VOICES_DIR=./voices \
BCDE_TAGGER_DATA=/caminho/para/BCDE-tagger/data \
  cargo run --release --example sintetizar -- "Bom dia, como está se sentindo?"
```

O exemplo:

- Carrega as vozes de `VOICES_DIR`
- Carrega o tagger de `BCDE_TAGGER_DATA`
- Sintetiza o texto passado como argumento
- Salva `saida.wav` no diretório atual

---

## Benchmarks

O repositório vem com benchmarks Criterion em `benches/sintese.rs`.

```bash
# Benchmarks puros (funções internas, sem modelo)
cargo bench

# Benchmarks completos (inclui síntese ONNX)
VOICES_DIR=./voices cargo bench
```

O que é medido:

| Benchmark | O que testa |
|-----------|-------------|
| `ipa_to_ids` | Conversão IPA → phoneme IDs |
| `samples_to_wav_bytes` | Encoder WAV (1s de áudio) |
| `f32_to_i16` | Conversão f32 → i16 (44100 amostras) |
| `sintese ONNX` | Inferência completa com modelo carregado |

Relatório HTML gerado em `target/criterion/report/index.html`.

---

## Testes

```bash
cargo test              # roda todos os testes
cargo test --release    # idem, mais rápido
```

Cobertura:

| Módulo | Testes |
|--------|--------|
| `wav.rs` | Header WAV válido, saturação f32→i16, WAV vazio |
| `piper.rs` | BOS/EOS, PAD duplo após pontuação, chars desconhecidos, ZWJ/tie bar |
| `dialog.rs` | Sem tags, uma tag, duas tags, tag desconhecida, retorno ao role, texto vazio |
| `orchestrator.rs` | Resample mesmo sr, downsample, upsample, entrada vazia |

---

## API reference

### `bcde_piper_vozz_rs::piper`

```rust
pub struct Piper { /* ... */ }

impl Piper {
    pub fn new(onnx_path: &Path, config_path: &Path) -> Result<Self>;
    pub fn sample_rate(&self) -> u32;
    pub fn num_speakers(&self) -> u32;
    pub fn id_map(&self) -> &HashMap<String, Vec<i64>>;
    pub fn create(
        &mut self,
        ipa: &str,
        use_espeak: bool,
        speaker_id: Option<i64>,
        length_scale: Option<f32>,
        noise_scale: Option<f32>,
        noise_w: Option<f32>,
    ) -> Result<(Vec<f32>, u32)>;
}

pub struct VoiceManager { /* ... */ }

impl VoiceManager {
    pub fn load_from_dir(dir: &Path) -> Result<Self>;
    pub fn get(&self, name: &str) -> Option<&Piper>;
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Piper>;
    pub fn list(&self) -> Vec<String>;
    pub fn len(&self) -> usize;
}
```

### `bcde_piper_vozz_rs::phonemizer`

```rust
pub struct Phonemizer { /* ... */ }

impl Phonemizer {
    pub fn new(tagger_data: &str, lexicon_path: Option<&Path>) -> Result<Self>;
    pub fn process_chunks(&mut self, texto: &str) -> Result<Vec<Chunk>>;
}
```

### `bcde_piper_vozz_rs::dialog`

```rust
pub struct Speaker {
    pub role: String,
    pub voice: String,
    pub sid: Option<i64>,
}

pub struct Segment { /* ... */ }

pub fn parse_dialogo(
    texto: &str,
    voice_default: &str,
    speakers: &[Speaker],
) -> (Vec<Segment>, usize);
```

### `bcde_piper_vozz_rs::orchestrator`

```rust
pub struct DialogoResultado {
    pub pcm: Vec<i16>,
    pub sr: u32,
    pub segmentos: usize,
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
) -> Result<DialogoResultado>;
```

### `bcde_piper_vozz_rs::wav`

```rust
pub fn samples_to_wav_bytes(samples: &[i16], sr: u32) -> Vec<u8>;
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16>;
```

---

## Design decisions

### Por que worker single-thread?

ONNX Runtime com múltiplas threads compete por CPU e degrada latência p95. Um worker = uma thread = uma CPU física. N workers = N CPUs em paralelo, sem contenção.

Medimos: `intra_threads=4` + `inter_threads=4` num worker reduz p95 em até 30% quando há carga, mas a média é pior. Single-thread tem p95 mais estável.

### Por que IPA com pontuação?

O modelo Piper foi treinado com `PAD` após cada phoneme e `PAD PAD` após pontuação. Se enviarmos IPA sem pontuação, o modelo não sabe onde pausar. Se enviarmos IPA com pontuação, ele gera as pausas naturalmente — sem inserir silêncio manual.

### Por que caminho de arquivo no áudio?

Um WAV de 4s em 22050Hz tem ~176KB. Em base64, ~235KB. Enviar isso dentro de JSON custa ~20ms por chamada (encode + serialize + write + read + parse + decode). Usando `audio_path` (caminho de arquivo temporário), o custo cai para ~1ms. O cliente lê o arquivo e apaga.

### Por que G2P separado?

O runtime interno do modelo trata pt-BR com qualidade limitada. Regras específicas do português brasileiro — trema, sândi, desambiguação de homógrafos (`sede` seat/thirst), nasalização contextual — são tratadas corretamente pelo `vozz-g2p-rs`. Enviar texto bruto ao modelo reduz a qualidade.

### `use_espeak` é ignorado

A assinatura `create(ipa, use_espeak, ...)` é mantida por compatibilidade de API, mas o argumento é ignorado. Nós sempre tratamos a entrada como IPA pré-fonemizada.

### Por que não há config de voz?

O objetivo é replicar o comportamento nativo dos modelos Piper com o mínimo de superfície: um modelo, seus defaults, os parâmetros opcionais que o chamador passar. Sem config persistente, sem valores mágicos, sem arquivos extras. O `<voz>.onnx.json` já contém tudo que o modelo precisa (`sample_rate`, `phoneme_id_map`, `inference` scales).

### Por que Cargo.lock é versionado?

O projeto distribui um binário (`bcde-piper-worker`). Aplicações binárias devem versionar o `Cargo.lock` para builds reprodutíveis. Bibliotecas normalmente não versionam — como este projeto é ambos, escolhemos versionar.

---

## Licença

**MIT License**. Veja [LICENSE](LICENSE).

Este projeto é parte do **ecossistema BCDE** e usa como dependência:

- [`vozz-g2p-rs`](https://github.com/bcdeosce/vozz-g2p-rs) — Apache 2.0
- [`BCDE-tagger`](https://github.com/bcdeosce/BCDE-tagger) — MIT
- [`ort`](https://github.com/pykeio/ort) — Apache 2.0 / MIT

Nenhuma dependência copyleft. Nenhum runtime externo. Nenhuma cláusula restritiva.

---

## Créditos

- **[rhasspy/piper](https://github.com/rhasspy/piper)** — modelo de TTS neural, arquitetura ONNX, especificação de vozes
- **[pykeio/ort](https://github.com/pykeio/ort)** — bindings Rust para ONNX Runtime
- **[vozz-g2p-rs](https://github.com/bcdeosce/vozz-g2p-rs)** — fonemizador pt-BR com desambiguação de homógrafos
- **[BCDE-tagger](https://github.com/bcdeosce/BCDE-tagger)** — POS tagging para pt-BR
- **[rhasspy/piper-voices](https://huggingface.co/rhasspy/piper-voices)** — vozes pt-BR oficiais

---

## Contribuições

Pull requests são bem-vindos. Para mudanças no pipeline de síntese:

1. Rode `cargo test` — testes unitários de parser, resample, WAV e IPA.
2. Rode `cargo bench` — confirme que não houve regressão de performance.
3. Descreva no PR o impacto em latência e duração.

Para bug reports, inclua:

- Voz usada (`faber-medium`, `edresson-low`, etc.)
- Texto (com tags se aplicável)
- Duração esperada vs obtida
- Amostra do áudio, se possível
```
