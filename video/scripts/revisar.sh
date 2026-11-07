#!/usr/bin/env bash
# Conferência SEM renderizar vídeo: regenera os projetos e tira quadros de cada fala + uma folha por capítulo.
# Uso: bash video/scripts/revisar.sh [capítulos...]   (sem argumentos = todos)  →  video/gerado/revisao/
set -euo pipefail
cd "$(dirname "$0")/../.."
caps=("$@"); [ ${#caps[@]} -gt 0 ] || caps=(1 2 3 4 5 6 7)
mkdir -p video/gerado/revisao/full
touch video/gerado/revisao/INDICE.txt
for c in "${caps[@]}"; do
  rm -f video/gerado/revisao/c${c}_*.png video/gerado/revisao/full/c${c}_*.png
  grep -v "^c${c}_" video/gerado/revisao/INDICE.txt > video/gerado/revisao/INDICE.tmp || true; mv video/gerado/revisao/INDICE.tmp video/gerado/revisao/INDICE.txt
  node video/scripts/build.mjs "$c" 16x9
  python3 - "$c" <<'PY'
import json, subprocess, sys
cap = sys.argv[1]; d = json.load(open(f'video/gerado/cap{cap}-tempos.json')); ini = d['inicio']; n = len(ini)
falas = json.load(open(f'video/audio/falas/c{cap}.words.json'))
for i in range(n):
    t = (ini[i] + (ini[i+1] if i+1 < n else d['fim'])) / 2 + 0.9
    open('video/gerado/revisao/INDICE.txt', 'a').write(f"c{cap}_{i}.png  cap {cap}, {t:.1f}s dentro do capítulo  | fala: {falas[i]['texto']}\n")
    subprocess.run(['effectcraft-cli', '--project', f'video/gerado/cap{cap}-16x9.ecproj', 'render-frame', '--comp', f'Cap{cap}',
                    '--time', f'{t:.2f}', '--scale', '0.5', '--out', f'video/gerado/revisao/c{cap}_{i}.png'], capture_output=True)
    subprocess.run(['effectcraft-cli', '--project', f'video/gerado/cap{cap}-16x9.ecproj', 'render-frame', '--comp', f'Cap{cap}',
                    '--time', f'{t:.2f}', '--out', f'video/gerado/revisao/full/c{cap}_{i}.png'], capture_output=True)
PY
  magick montage video/gerado/revisao/c${c}_*.png -tile 3x -geometry +4+4 -background '#222' video/gerado/revisao/cap$c.png
done
echo "quadros em video/gerado/revisao/ (meia resolução) e revisao/full/ (1920x1080) (nenhum vídeo foi renderizado)"
