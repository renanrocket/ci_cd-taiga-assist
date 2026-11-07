#!/usr/bin/env bash
# Refaz o vídeo 16:9 do zero: voz → projetos → render dos capítulos → montagem. Uso: [SUFIXO=-v2] bash video/scripts/refazer.sh [--sem-voz]   (SUFIXO evita sobrescrever o MP4 final anterior)
set -euo pipefail
cd "$(dirname "$0")/../.."
[ "${1:-}" = "--sem-voz" ] || video/.venv/bin/python video/scripts/tts.py
mkdir -p video/out/cap
for i in 1 2 3 4 5 6 7; do
  node video/scripts/build.mjs $i 16x9
  effectcraft-cli --project video/gerado/cap$i-16x9.ecproj render --comp Cap$i --out video/out/cap/cap$i.mp4 --format h264 --audio off | tail -1
done
python3 video/scripts/montar.py 16x9 --sufixo="${SUFIXO:-}"
