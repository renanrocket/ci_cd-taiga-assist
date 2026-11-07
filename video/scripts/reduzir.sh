#!/usr/bin/env bash
# Gera uma versão reduzida (re-encode H.264) de um MP4 já renderizado, SEM tocar no original.
# Uso: bash video/scripts/reduzir.sh [entrada.mp4] [saida.mp4] [crf=22]
#   padrão: out/taiga-assist-16x9-v2.mp4 → out/taiga-assist-16x9-v2-reduzido.mp4
set -euo pipefail
cd "$(dirname "$0")/.."
ENT="${1:-out/taiga-assist-16x9-v2.mp4}"; SAI="${2:-out/taiga-assist-16x9-v2-reduzido.mp4}"; CRF="${3:-22}"
[ "$ENT" != "$SAI" ] || { echo "entrada e saída iguais: abortando para não sobrescrever o original"; exit 1; }
# x264 lento (melhor compressão), tune animation (bom para vetor/UI plana), CRF (qualidade constante), teto de taxa para
# streaming, GOP de ~4 s, áudio AAC 128k; o áudio já está normalizado (-14 LUFS), então só muda a taxa.
ffmpeg -y -loglevel error -i "$ENT" -c:v libx264 -preset slow -crf "$CRF" -tune animation -pix_fmt yuv420p \
  -maxrate 2500k -bufsize 5000k -g 120 -bf 3 -refs 4 -c:a aac -b:a 128k -ar 48000 -movflags +faststart "$SAI"
ls -l "$SAI" | awk '{printf "%.1f MB\n", $5/1048576}'
