#!/usr/bin/env bash
# Baixa vozes pt-BR do HuggingFace (rhasspy/piper-voices) para ./voices/
#
# Uso:
#   ./scripts/baixar_vozes.sh                    # baixa as vozes padrão
#   ./scripts/baixar_vozes.sh faber-low edresson-low
#   VOICES_DIR=/outro/dir ./scripts/baixar_vozes.sh cadu-medium
#
# Lista completa: https://huggingface.co/rhasspy/piper-voices/tree/main/pt/pt_BR

set -euo pipefail

VOICES_DIR="${VOICES_DIR:-voices}"
BASE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR"

# Vozes padrão (nomes no formato "<speaker>-<quality>")
DEFAULT_VOICES=(
    "faber-low"
    "faber-medium"
    "edresson-low"
    "cadu-medium"
)

# Mapeamento nome-voz → caminho no HF
# faber-low  → pt/pt_BR/faber/low/
# faber-medium → pt/pt_BR/faber/medium/
# edresson-low → pt/pt_BR/edresson/low/
# cadu-medium → pt/pt_BR/cadu/medium/
hf_path() {
    local voz="$1"
    local speaker="${voz%-*}"
    local quality="${voz##*-}"
    echo "$speaker/$quality"
}

# Baixa um arquivo (segue redirects)
download() {
    local url="$1"
    local dest="$2"
    if [[ -f "$dest" ]]; then
        echo "  ⏭️  já existe: $(basename "$dest")"
        return 0
    fi
    echo "  ⬇️  $(basename "$dest")"
    curl -fL --retry 3 --retry-delay 2 -o "$dest" "$url"
}

baixar_voz() {
    local voz="$1"
    local speaker="${voz%-*}"
    local quality="${voz##*-}"
    local subpath
    subpath=$(hf_path "$voz")

    local dir="$VOICES_DIR/$voz"
    mkdir -p "$dir"

    local prefix="pt_BR-${speaker}-${quality}"
    local onnx_url="$BASE_URL/$subpath/${prefix}.onnx"
    local json_url="$BASE_URL/$subpath/${prefix}.onnx.json"

    echo "🎙️  $voz"
    download "$onnx_url" "$dir/$voz.onnx"
    download "$json_url" "$dir/$voz.onnx.json"
}

# ─── Main ─────────────────────────────────────────────────────────────
mkdir -p "$VOICES_DIR"
touch "$VOICES_DIR/.gitkeep"

if [[ $# -gt 0 ]]; then
    VOZES=("$@")
else
    VOZES=("${DEFAULT_VOICES[@]}")
fi

echo "📁 Vozes em: $VOICES_DIR"
echo "📦 Total: ${#VOZES[@]} voz(es)"
echo

for voz in "${VOZES[@]}"; do
    baixar_voz "$voz" || echo "  ⚠️  falha em $voz (verifique se existe em rhasspy/piper-voices)"
done

echo
echo "✅ concluído"
ls -lh "$VOICES_DIR" 2>/dev/null || true
