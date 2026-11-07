# Produção do vídeo de apresentação — como foi feito e como refazer

> Complementa [video-spec.md](video-spec.md) (o quê e por quê: roteiro, estilo, decisões). Este arquivo é o **como**:
> o processo completo, do roteiro ao MP4, e como editar cada parte depois. Última atualização: 2026-11-07.
> Estado: versão **16:9** com 7 capítulos (~181 s) pronta no gerador. Falta: render final, versão 9:16, voz oficial (Azure).

## 1. Regra de ouro: preparar → conferir → renderizar

O render completo leva ~13 min. Para não pagar isso a cada retificação, o fluxo é sempre:

1. **Preparar** (editar scripts, arte, roteiro, pronúncia).
2. **Conferir** com quadros estáticos (segundos): `bash video/scripts/revisar.sh [capítulos]`.
3. **Revisar** as imagens e pedir retificações citando o nome da imagem (ex.: "c4_2: afaste o botão").
4. **Áudio**: `python3 video/scripts/montar.py 16x9 --so-audio` e ouvir `video/audio/previa-audio.mp3`.
5. **Só então renderizar**: `bash video/scripts/refazer.sh --sem-voz`. **Nunca renderizar antes do pedido explícito.**

## 2. Estrutura de pastas

| Pasta/arquivo | O que é | Git |
|---|---|---|
| `video/roteiros/video.json` | Narração por capítulo e por fala (fonte da verdade do texto) | versionado |
| `video/roteiros/pronuncia.json` | Pronúncia **só da voz** (ex.: `git` → `guíti`); legendas seguem o texto original | versionado |
| `video/arte/` | **Vetores (SVG)** do fundo (5 colunas + sombra), **regerados** por `scripts/fundo.mjs` | ignorado (regerável) |
| `video/scripts/` | Gerador: `lib.mjs`, `ui.mjs`, `fundo.mjs`, `cap1.mjs`, `caps.mjs`, `build.mjs`, `tts.py`, `musica.py`, `montar.py`, `revisar.sh`, `refazer.sh`, `reduzir.sh` | versionado |
| `video/audio/falas/` | MP3 de cada fala + `cN.words.json` (tempo de cada palavra) | ignorado |
| `video/audio/musica/` | MP3 da música (qualquer `.mp3`; `principal.mp3` tem prioridade) | ignorado |
| `video/audio/` (`voz.wav`, `mix.wav`, `previa-audio.mp3`) | Voz, mixagem e prévia para ouvir | ignorado |
| `video/gerado/` | Projetos `.ecproj`, `.jsonl` de comandos, `capN-tempos.json` | ignorado |
| `video/gerado/revisao/` | Quadros para revisão (`full/` = 1920×1080), `INDICE.txt`, `GERADO_EM.txt` | ignorado |
| `video/out/` | MP4 dos capítulos (`out/cap/`) e o final `taiga-assist-16x9.mp4` | ignorado |
| `video/.venv/` | Python com `edge-tts` | ignorado |

A fonte da verdade é **roteiro + scripts** (no git). Tudo o que está em `arte/`, `gerado/`, `out/` e `audio/` é **gerado e fica fora do git**:
se apagar, refaz-se com `bash video/scripts/refazer.sh` (o SVG do fundo sai de `scripts/fundo.mjs`; a voz, de `tts.py`).
O que **não** se refaz sozinho e deve ser guardado à parte: o MP3 da música (`video/audio/musica/`) e os MP4 finais (`video/out/`).

## 3. Ferramentas (instaladas em 2026-11-07)

- **EffectCraft** (suíte Craft, grátis) pela CLI `effectcraft-cli` em `~/.local/bin` (ver video-spec §2.3). Sem janela.
- **edge-tts** no venv: `python3 -m venv video/.venv && video/.venv/bin/pip install edge-tts` (precisa de internet).
- `ffmpeg`/`ffprobe`, `node`, `python3`, ImageMagick (`magick`) e `rsvg-convert` do sistema.
- Fontes: **Inter** (embutida no EffectCraft, só até *Bold*; não há *Black*) e **JetBrains Mono** (do sistema).

## 4. Passo a passo

### 4.1 Roteiro e voz
1. Editar o texto em `roteiros/video.json` (capítulo → `falas[]`; cada fala é uma frase).
2. Ajustar pronúncia em `roteiros/pronuncia.json` (chave: palavra minúscula sem pontuação; valor: como falar).
   Já mapeados: Git → *guíti*, Kiro → *quíro*, CLI → *cí él ái*, deploy → *deplói*, push → *púxi*, branch → *brântchi*,
   IA → *í á*, NTI → *ene tê í*, US → *iúzer stóri* (plural *iúzer stóris*), app → *épi*, link → *línk*, Windows/Mac/Linux,
   Assist → *assíst*, "pré-requisitos" → "pré requisitos" (o hífen confundia a voz).
3. Gerar: `video/.venv/bin/python video/scripts/tts.py [capítulos]`. Voz `pt-BR-FranciscaNeural` (reserva `ThalitaNeural`).
   - Para cada fala grava `c<cap>_<n>.mp3` e, em `c<cap>.words.json`, o tempo de cada palavra (WordBoundary).
   - **Alinhamento legenda × fala:** a voz fala o texto "respelhado", mas `words.json` guarda as palavras **originais** com o
     tempo dos tokens falados (uma palavra original pode virar vários tokens, ex.: `CLI` → 3). Se o serviço devolver marcações a
     menos, o script tenta de novo (até 4 vezes) e, se persistir, aborta com o erro.
   - A duração de cada cena sai desse áudio: **mudar texto/pronúncia muda os tempos** e exige regenerar os capítulos.

### 4.2 Vetores e arte (fundo)
- **Fundo (animado):** `scripts/fundo.mjs` gera **um SVG por coluna** (`video/arte/fundo-coluna-1..5-16x9.svg`, cada um mais alto que a
  tela para o movimento nunca mostrar a ponta) e `video/arte/fundo-sombra-16x9.svg` (escurecimento + vinheta forte). Camadas, de baixo
  para cima: base lisa `#4e4e4a` → 5 colunas → sombra. **As colunas alternam sobe-e-desce** (uma sobe enquanto a vizinha desce),
  muito devagar: amplitude 26–38 px e período ≈ 21 s (`AMPLITUDE` e `VELOCIDADE` em `fundo.mjs`). O movimento continua entre os
  capítulos (o `build.mjs` soma a duração dos capítulos anteriores à fase do seno, sem salto no corte).
  Os SVGs ficam em `video/arte/` **fora do git** (são regerados por `fundo.mjs`) e podem ser abertos no VectorCraft/Inkscape, **mas o build os
  regrava** a cada execução: para mudar o desenho de forma permanente, edite `gerarSvgs` em `fundo.mjs` (que está no git).
  Prévia só do fundo: `node video/scripts/fundo-preview.mjs [segundos]` → `video/out/fundo-animado-16x9.mp4`.
- **Componentes de UI** (janela do app, caixas, botões, cartões do Kanban, pílulas, notificação) **não são arquivos SVG**:
  são formas do EffectCraft desenhadas por código em `scripts/ui.mjs` e `scripts/lib.mjs`. Para editá-los, altere o código.
  Se for necessário editá-los como vetores em editor gráfico, exportar cada componente para `video/arte/` (pendente; ainda não feito).
- **Cenas** (o que aparece em cada fala): `scripts/cap1.mjs` (capítulo 1) e `scripts/caps.mjs` (capítulos 2 a 7).
  Coordenadas absolutas em 1920×1080; tempos vêm de `T.inicio[i]` (início da fala *i*) e de `pal(i, /regex/)` (instante de uma palavra).
- **Animação** por expressões de tempo (`ease`) em opacidade, escala, posição e cor, na camada. O fundo anima as colunas (ver 4.2). **Expressões em texto (`text/sourceText`) não funcionam**; texto digitado e legendas são camadas por passo.
- **Legendas:** 2–3 palavras por vez, palavra falada em laranja, a ~80 % da altura (16:9). Geradas em `build.mjs` a partir de `words.json`.

### 4.3 Conferência (sem renderizar vídeo)
`bash video/scripts/revisar.sh [capítulos]` regenera os projetos e exporta, em `video/gerado/revisao/`:
`full/cN_i.png` (1920×1080, um quadro no meio de cada fala), `cN_i.png` (meia resolução), `capN.png` (folha do capítulo),
`INDICE.txt` (imagem → capítulo, segundo, fala) e `GERADO_EM.txt`.
Sem argumentos refaz todos; com argumentos limpa e refaz só os capítulos pedidos.

### 4.4 Revisão das imagens
O autor revisa `full/` e pede ajustes pelo **nome da imagem** (`c3_3`, `c4_2`...). Cada ajuste: editar o script do capítulo,
rodar `revisar.sh` desse capítulo e devolver as imagens atualizadas. Regras de layout aprendidas na revisão:
- Nada pode ultrapassar a borda da janela (botões, campos, retângulos); popups e notificações aparecem **sozinhos**, com o app encerrado
  antes (senão botões "vazam" por baixo).
- Textos longos em cartões quebram em duas linhas e o cartão cresce (cartão do Kanban com 150 px).
- Cartões do Kanban ficam abaixo dos títulos das colunas (`topo = cy − h/2 + 190`).
- Controles com a mesma função têm o mesmo estilo e posição em todas as cenas (ver [ui-conventions.md](ui-conventions.md)).
- As imagens de revisão só valem para o **projeto atual**; um MP4 antigo pode estar desatualizado.

### 4.5 Áudio (voz + música)
`python3 video/scripts/montar.py 16x9 --so-audio` gera `video/audio/previa-audio.mp3` para ouvir **antes** do render:
1. **Voz:** cada fala é atrasada para (início do capítulo + início da fala) e somadas em `voz.wav`.
2. **Música** (`scripts/musica.py`): usa o `.mp3` de `video/audio/musica/` (`principal.mp3` tem prioridade).
   - Se o vídeo for **maior** que a faixa: repete em **loop com transição suave** (`acrossfade` de 3 s entre repetições).
   - Se for menor: corta. **Fade-in de 1 s** no início e **fade-out de 3 s no fim** do vídeo.
   - Volume base 0,10 e **ducking** (sidechain) sob a voz.
3. **Normalização** ~ −14 LUFS (pico ≤ −1,5 dBFS).
Faixa atual: `zephiramusic-lofi-tech-588185.mp3` (91 s → 3 cópias para ~181 s). Registrar licença/origem em
`video/musica-licencas.md` (pendente; ver video-spec §2.4).

### 4.6 Render e montagem final (só quando o autor pedir)
`bash video/scripts/refazer.sh [--sem-voz]`: voz (se não `--sem-voz`) → `build.mjs` de cada capítulo → `effectcraft-cli render`
(H.264, sem áudio, ~1,5–3 min por capítulo) → `montar.py` (concatena os 7 MP4, muxa o áudio normalizado, `+faststart`).
Saída: `video/out/taiga-assist-16x9.mp4` (1920×1080, 30 fps, H.264 + AAC).
Para refazer só um capítulo: `node video/scripts/build.mjs <cap> 16x9`, `effectcraft-cli --project video/gerado/cap<N>-16x9.ecproj render --comp Cap<N> --out video/out/cap/cap<N>.mp4 --format h264 --audio off`, depois `montar.py`.

### 4.7 Versões e tamanho do arquivo
- **Versões em `video/out/`:** `taiga-assist-16x9.mp4` (v1, 149 MB) e `taiga-assist-16x9-v2.mp4` (v2, 119 MB) ficam intocadas. Para não
  sobrescrever uma versão anterior, renderize com sufixo: `SUFIXO=-v3 bash video/scripts/refazer.sh --sem-voz`.
- **Por que o render sai grande:** o codificador do EffectCraft grava H.264 a ~5 Mbps (1 quadro-chave a cada ~2 s), muito acima do necessário
  para um conteúdo vetorial plano e de movimento lento.
- **Versão reduzida:** `bash video/scripts/reduzir.sh [entrada] [saída] [crf]` (padrão: v2 → `taiga-assist-16x9-v2-reduzido.mp4`, ~13,6 MB,
  ~10× menor). Não toca no original. O que faz (a mesma ideia do YouTube, que re-codifica no upload):
  1. **Re-encode H.264 em CRF** (qualidade constante, 22) com `preset slow`: o x264 gasta mais CPU para achar a melhor compressão.
  2. **`-tune animation`**: ajustado para áreas planas e bordas nítidas (UI/vetor), com mais quadros de referência.
  3. **Teto de taxa** (`-maxrate 2500k`) e GOP de ~4 s: arquivo previsível e bom para streaming/WhatsApp/Instagram.
  4. **Áudio AAC 128 kbps** (era ~198): a voz e a música já estão normalizadas (−14 LUFS).
  5. `yuv420p` e `+faststart` (compatível com tudo e começa a tocar antes de baixar inteiro).
- **Medição** (trecho de 30 s, contra a v2): CRF 20 → 2,3 MB, VMAF 96,9 · CRF 23 → 1,9 MB, 96,3 · CRF 26 → 1,6 MB, 95,4 · CRF 28 → 1,4 MB, 94,7.
  Final (CRF 22, vídeo inteiro): **13,6 MB, VMAF 96,6**, mesmo áudio (−14,2 LUFS), 181 s, 1920×1080.
- **Não adotado:** HEVC/AV1/VP9 reduziriam um pouco mais, mas têm compatibilidade pior (Instagram, WhatsApp, TVs, navegadores antigos).

## 5. Problemas conhecidos e armadilhas
- `tts.py` pode falhar o alinhamento por variação do serviço de voz (há repetição automática).
- Alterar a narração/pronúncia muda a duração das falas e, portanto, os tempos de todas as cenas: sempre rode `revisar.sh`.
- Capítulos têm corte seco entre si (o fundo reinicia o movimento); aceito.
- `kill`/`pkill` genérico pode atingir o shell; para parar um render, usar o PID exato do `effectcraft-cli`.
- Não commitar `video/arte/`, `video/out/`, `video/gerado/`, `video/audio/`, `video/.venv/`, `*.ecproj` nem o vídeo de referência (todos no `.gitignore`).

## 6. Pendências
1. **Render final** do 16:9 (aguardando o autor).
2. **Versão 9:16** (layout vertical por componente; hoje só `16x9`).
3. **Voz oficial** (Azure AI Speech) se o autor fornecer a chave; hoje `edge-tts` (não oficial).
4. Exportar os componentes de UI como SVG em `video/arte/` (se o autor quiser editá-los fora do código).
5. `video/musica-licencas.md` com origem/licença da faixa usada.
