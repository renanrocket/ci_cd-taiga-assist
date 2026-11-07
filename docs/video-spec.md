# Vídeo de apresentação do Taiga Assist — spec

> Especificação do vídeo (o quê e por quê). Tudo o que foi decidido em 2026-11-07 está aqui. **O vídeo 16:9 já foi
> produzido**: o processo, os scripts e como refazer estão em [video-producao.md](video-producao.md); o estado final está
> na seção 11 deste arquivo. A versão 9:16 ainda não foi feita.

## 1. Objetivo e público

- **Objetivo:** ensinar colegas de trabalho (desenvolvedores do NTI Sistemas) a instalar e usar o Taiga Assist,
  de forma didática e bonita, no lugar de um README.
- **Problema contado no vídeo:** o fluxo era *programar → deploy → relatório no Taiga*. Com IA, programar ficou
  rápido; o deploy já está sendo automatizado; **sobrou o relatório** (em semana cheia, até ~2 h). O Taiga Assist
  resolve isso.
- **Onde será visto:** principalmente no **Instagram** (celular), e também compartilhado entre colegas.
- **Formato:** **um único vídeo** (~3 min) com capítulos, em duas versões: **vertical 9:16** (Instagram) e **horizontal 16:9**.

## 2. Decisões tomadas

| Tema | Decisão |
|---|---|
| **Formato** | **UM vídeo único de ~2:58 (178 s)**, dividido em 7 capítulos (não é série). *Conferir o limite atual de duração dos Reels; se for menor, publicar a versão 16:9 em outro lugar.* |
| **Orientação** | **Mestre vertical 9:16 (1080×1920)** + **versão horizontal 16:9 (1920×1080)** gerada dos mesmos dados (aprovada pelo autor). 30 fps, H.264 + AAC (MP4). Ver 2.1 |
| **Estilo visual** | *Motion design* de **objetos vetoriais planos**, **sem personagens** (como a referência) |
| **Fundo** | **Quadro Kanban visto de cima** (colunas e cartões), escurecido, com vinheta forte |
| **Legendas** | Frases curtas (2–3 palavras por vez), brancas, **palavra falada destacada em laranja**, sincronizadas palavra a palavra |
| **Telas do app** | **Mocks vetoriais simplificados** com dados fictícios. **Taiga fictício inspirado no Taiga real** (colunas, cartões com número e selos de pontos), sem copiar a marca (ver 5.0) |
| **Voz** | **Feminina, sintética, pt-BR: `pt-BR-FranciscaNeural`** (reserva: `pt-BR-ThalitaNeural`). Protótipo e tempos com `edge-tts`; **versão final com Azure AI Speech** (mesma voz, serviço oficial) se o autor fornecer a chave; sem chave, publicar com a mesma voz via `edge-tts` (risco de termos assumido pelo autor). Ver 6.1 |
| **Marca** | Apenas o **texto "NTI Sistemas" em branco com borda preta** (sem logo), na frase de abertura e no cartão final |
| **Música** | **Pixabay**: faixa principal + 2 reservas (seção 2.4), baixa sob a voz |
| **Motor** | **EffectCraft** (suíte Craft, grátis, roda **sem janela** por CLI/scripts, exporta MP4 H.264 vertical e horizontal; piloto 1 passou, seção 2.3). **SoundCraft** para áudio; **FilmCraft** só se precisar. Plano B: Revideo/Motion Canvas (MIT). **Remotion descartado** (2.2) |

### 2.1 Orientação: vertical como mestre, horizontal derivado
- O Instagram prioriza **Reels em 9:16**; o 16:9 aparece pequeno no celular. Por isso o **mestre é o vertical**.
- A versão **16:9** (aprovada) atende quem assiste no computador ao lado do app (por exemplo, instalando enquanto vê).
  Os dois saem do **mesmo roteiro, da mesma arte e dos mesmos scripts**: cada componente define **dois layouts**
  (vertical e horizontal) e o gerador emite uma composição para cada formato.
- **Ressalva do vertical:** o app é **desktop**; em 1080 px de largura o texto da janela fica pequeno. Mitigação:
  mocks **simplificados** (poucos elementos, letras grandes), **zoom** em cada parte da tela e a janela ocupando a
  largura toda, como na referência. No 16:9 as janelas ficam maiores e mais legíveis.
- **Zona segura (vertical):** o feed corta para 4:5 e o Reels cobre a base com a interface. Texto importante na faixa
  central; legendas por volta de 60–65 % da altura. *Conferir com um teste de upload.*
- **Zona segura (16:9):** margens de 6 % nas bordas; legendas a ~80 % da altura, centralizadas.

### 2.2 Licença das ferramentas
> **Status:** o Remotion foi **descartado** (a suíte Craft é gratuita e resolveu o motor). A análise abaixo fica
> registrada caso a decisão seja revista.

- **Remotion (FAQ oficial lido em 2026-11-07):** **não existe "criar" licença gratuita**. A licença gratuita vale
  **automaticamente**, sem cadastro nem chave ("You don't have to contact us" / "sign up on remotion.pro"), **se
  você for elegível**: pessoa física (uso pessoal ou comercial), organização ou equipe de **até 3 pessoas**,
  organização sem fins lucrativos, ou avaliação sem uso comercial. Não há marca d'água nem limite de renders no
  plano gratuito; a funcionalidade é igual à paga.
  - **Ponto crítico:** o FAQ diz que, numa empresa de 20 pessoas em que só 1 usa o Remotion, é preciso Company
    License "unless a Free License exemption applies", e que a elegibilidade depende do **total de pessoas da
    empresa**. Não há regra escrita sobre funcionário fazendo vídeo para a própria empresa, mas **a leitura mais
    provável é que o NTI Sistemas (mais de 3 pessoas), com a marca no vídeo, não se enquadra na licença
    gratuita**. *Os Termos (<https://www.remotion.dev/docs/terms>) não foram lidos; confirmar com a Remotion.*
  - **Custo, se precisar:** plano "Creators", **1 seat (US$ 25/mês)** para quem escreve código Remotion
    **inclusive usando agentes de IA** (é o caso aqui). O FAQ afirma que os vídeos produzidos continuam podendo ser
    usados após cancelar. Dá para pagar **1 mês**, produzir o vídeo e cancelar.
  - A licença do Remotion **não cobre patentes de codecs** (H.264/AAC), um assunto à parte e comum a qualquer
    software de vídeo.
- **Plano B:** **Revideo** (fork do Motion Canvas com renderização sem interface) ou **Motion Canvas**, ambos MIT.
  Mais trabalho manual nos elementos de interface (janelas, cartões), pois desenham em canvas.
- **Fontes:** usar OFL (Inter ou Poppins Black para títulos; JetBrains Mono para terminal).
- **Música:** ver 2.4.
### 2.3 Suíte ArtCraft/Craft: motor escolhido (piloto 1 passou; piloto 2 na produção)
- **O que é:** suíte gratuita e open source (Apache-2.0/MIT; getartcraft.com), apps nativos em Rust, **cada app com
  CLI e servidor MCP** (`<app>-cli mcp`). O *launcher* não tem MCP; os apps instalados por ele têm. Não confundir
  com o "ArtCraft Studio" (geração de imagem/vídeo com IA, repositório storytold/artcraft).
- **Instalado na máquina do autor (2026-11-07):** VectorCraft 0.22.0, PhotoCraft 0.21.0 e CADCraft 0.18.0 (**Flatpak**);
  EffectCraft 0.22.0, FilmCraft 0.18.0, SoundCraft 0.18.0 e SolidCraft 0.30.1 (**AppImage** em `~/Applications/`).
- **Ambiente preparado (2026-11-07), pronto para uso:**
  - AppImages **extraídos** em `~/.local/opt/craft/{effectcraft,filmcraft,soundcraft}/squashfs-root/` e **wrappers**
    em `~/.local/bin/{effectcraft,filmcraft,soundcraft,vectorcraft}-cli` (já ajustam o `LD_LIBRARY_PATH`; o do
    VectorCraft chama o Flatpak). Testados: `effectcraft-cli info`, `soundcraft-cli --version`, etc.
  - Pastas de trabalho do MCP: `~/craft/in` (leitura) e `~/craft/out` (escrita); o Flatpak do VectorCraft recebeu
    acesso a `~/craft` (`flatpak override --user --filesystem=$HOME/craft ai.storyteller.vectorcraft`).
  - **MCPs registrados** (escopo *local* do projeto `~/Projetos/ci_cd-taiga-assist`, em `~/.claude.json`; **todos ✔
    Connected**): `effectcraft`, `vectorcraft`, `soundcraft`, cada um com `--automation-read-root ~/craft/in
    --automation-write-root ~/craft/out`. Aparecem ao abrir uma **nova sessão** do Claude Code nessa pasta. O FilmCraft
    **não** foi registrado (sem confinamento de pastas); usar a CLI.
  - **Desfazer:** `claude mcp remove <app> -s local`; `rm -rf ~/.local/opt/craft ~/.local/bin/*-cli`;
    `flatpak override --user --reset ai.storyteller.vectorcraft`.

**Piloto 1 realizado (2026-11-07, sem janela, em pasta temporária) — passou:**
```
effectcraft-cli --empty --save-as piloto.ecproj run \
  comp.new '{"name":"Teste","width":1080,"height":1920,"frameRate":30,"duration":3}' \
  layer.newSolid '{"name":"Fundo","color":"#0b111f"}' layer.newText '{"text":"CHEGA DE RELATÓRIO MANUAL"}'
effectcraft-cli --project piloto.ecproj render-frame --comp Teste --time 1 --out frame.png   # 1080x1920 em ~16 ms
effectcraft-cli --project piloto.ecproj render --comp Teste --out piloto.mp4 --format h264   # 90 quadros em 2,5 s
```
Resultado: **MP4 H.264, 1080×1920, 30 fps, 3,0 s**; acentos corretos no texto. (O texto saiu grande demais e sem
quebra de linha: é só ajuste de parâmetros.)

**O que as CLIs oferecem (lido do `--help`):**
- **EffectCraft:** 760 comandos e 308 efeitos; `comp.new` (largura/altura/fps/duração); `layer.newSolid`, `layer.newText`;
  **presets de animação de texto** (`typewriter`, `fadeUpCharacters`, `bounce`…) e animadores/seletores de texto;
  `props`/`get`/`set` (keyframes e expressões); **`script FILE.jsx`** (JavaScript com modelo de objetos no estilo
  After Effects); `render` (h264, hevc, av1, prores, webm, png, gif, wav… com `--fps`, `--bitrate`, `--audio`);
  **modelos `.ectemplate` renderizados com valores** (`render --template T --values V.json`);
  `render-frame` (PNG, para conferir); `run --commands FILE` (JSON/JSONL, até 100 mil passos);
  `mcp` com `--automation-read-root/--automation-write-root` (confina pastas).
- **FilmCraft:** motor sem janela; `import`, `inspect`, `exec`, `run script.jsonl`, `export <saída> [--preset]
  [--format h264|hevc|prores|…] [--scale] [--quality] [--no-audio]`, `render --seconds S --out f.png`, `probe`, `mcp`.
- **SoundCraft:** `info`, `convert` (reamostrar, profundidade de bits, **normalizar**), `run`/`script`, `commands`, `mcp`.

**Maturidade e riscos:** todos declarados **alfa/em desenvolvimento**, com aviso de comportamento imprevisível;
`command_batch` do MCP **não é transacional**; o MCP "não é uma fronteira de autorização" e o FilmCraft não confina
pastas (usar pastas dedicadas e trabalhar em cópia). *Fontes secundárias diziam não haver MP4/9:16; o piloto mostrou
que há.*

**Ainda NÃO validado (escopo do piloto 2, passo 2 da produção, seção 9):**
1. **Destaque palavra a palavra** (seletor/animador de texto por palavra) e legendas de 2–3 palavras por vez.
2. **Importar SVG** (arte do VectorCraft) e PNG como camadas.
3. **Importar áudio** (voz) e exportar MP4 com o áudio sincronizado.
4. **Fontes** (Poppins/Inter Black, JetBrains Mono): como carregar.
5. **Formas** (janelas, cartões com cantos arredondados, pílulas), máscaras e **vinheta**.
6. **Easing/entradas** (escala + fade, *ease-out*, leve *overshoot*) e movimento de câmera (zoom/pan).
7. Qualidade e tempo de render para ~35 s a 30 fps.

**Decisão:** **EffectCraft é o motor principal.** A fonte da verdade são **scripts versionados** (JSONL/JSX) em
`video/`; os projetos `.ecproj` são **gerados** a partir deles (fora do git). Para produção usar a **CLI diretamente**
(previsível e versionável); o **MCP serve para exploração** com o Claude. Se o piloto 2 bloquear em algum item,
usar o plano B (Revideo/Motion Canvas, MIT) mantendo VectorCraft (arte) e SoundCraft (áudio).
O registro dos MCPs já foi feito com autorização do autor (ver acima).

### 2.5 Resultado do piloto 2 e da produção 16:9 (2026-11-07)
- **Funciona no EffectCraft (CLI):** formas arredondadas, SVG e áudio importados, fontes (Inter Bold; JetBrains Mono do sistema; a Inter não tem Black),
  vinheta (SVG), easing/entrada por **expressões de tempo** (`ease`) em opacidade, escala, posição e cor, e render MP4 H.264 (~2 min por capítulo).
- **Não funciona:** expressões em `text/sourceText` (erro "not a callable function"). Legendas e texto digitado são camadas por passo.
- **Pipeline:** `video/scripts/` (`build.mjs <cap> <16x9>` gera e monta o projeto; `tts.py` voz; `montar.py` concatena, mixa e normaliza).
  Arte vetorial em `video/arte/` (regerada por `fundo.mjs`; fora do git). Saída: `video/out/taiga-assist-16x9.mp4` (180 s, −14 LUFS).
- **Pendente:** música (`video/audio/musica/principal.mp3`), versão 9:16 (layout vertical por componente) e a voz oficial (Azure).

### 2.4 Música (Pixabay): escolhidas
Escolha por **título, gênero, tags e duração** das listagens do Pixabay (lidas em 2026-11-07). **Ninguém ouviu as
faixas e o Pixabay não mostra BPM**: a avaliação final é no vídeo montado. Critério: instrumental, tech/explainer,
limpa, e **duração perto dos ~3 min** do vídeo.

| Ordem | Faixa | Artista | Duração | Link |
|---|---|---|---|---|
| **1 (principal)** | Corporate Explainer Video | alex-morgan | 3:29 | <https://pixabay.com/music/corporate-corporate-explainer-video-568180/> |
| 2 (reserva) | Tutorial - Tutorial Music | PaulYudin | 2:45 | <https://pixabay.com/music/corporate-tutorial-tutorial-music-574011/> |
| 3 (reserva) | Product Tutorial Background (Clean Tech) | (artista não capturado) | 2:39 | <https://pixabay.com/music/corporate-product-tutorial-background-clean-tech-459161/> |

- A **principal** cobre o vídeo inteiro (fade-out no fim). As reservas são mais curtas: **estender** com repetição do
  trecho final e `acrossfade` (ffmpeg). Usar a reserva só se a principal não funcionar no vídeo montado.
- **Licença (Pixabay, lida em 2026-11-07):** uso comercial permitido; **atribuição não é obrigatória** (agradecida);
  **proibido** redistribuir a faixa isolada ("standalone"), usar como marca e usos políticos/enganosos. Os termos
  **não tratam de Content ID** (YouTube/Instagram): se a plataforma sinalizar a faixa, contestar com a prova da
  licença. *Não é parecer jurídico.*
- **Regra de produção:** guardar em `video/musica-licencas.md` o nome, o link, a data e um print da página de cada
  faixa usada. Mixar **baixa sob a voz** (≈ −18 a −22 dB), com fade-in de 1 s e fade-out de 3 s.
- **Alternativa:** no Instagram, a música pode ser adicionada pelo próprio app ao publicar (biblioteca licenciada
  pela plataforma; contas comerciais têm biblioteca limitada). *Não verificado.*
- **Ação do autor:** baixar o **MP3 da faixa 1** (o download do Pixabay é por clique no navegador) para
  `video/audio/musica/principal.mp3` (e, se quiser, as reservas como `reserva1.mp3`, `reserva2.mp3`).

## 3. Análise da referência (vídeo "2 Instagram.mp4", 33 s)

> O arquivo está em `~/Downloads/2 Instagram.mp4` (fora do repositório; **não commitar**). Tema: "Build your
> own AirTag under $15, parte 2". Dados medidos com `ffprobe`/`ffmpeg`.

- **Técnico:** 1080×1920, 30 fps, VP9 + AAC estéreo 48 kHz, 33,2 s, ~0,78 Mbps. **Nenhum corte brusco**
  (detecção de cena > 0,25 vazia): é uma única composição animada com fades e entradas. Áudio **sem pausas**
  (≥ 0,5 s) e volume médio ≈ −17 dB; **não foi possível ouvir** (voz sintética? música?): *perguntar ao autor*.
- **Ritmo:** ~**1 legenda por segundo** (~33 em 33 s); um elemento visual novo a cada 1–1,5 s.
- **Linha do tempo observada:**
  1. 0–4 s: rótulo "PART 2", título em 2 linhas ("BUILD YOUR OWN" / "AIRTAG" em dourado), objeto principal (placa)
     no centro, depois "UNDER $15". Legenda: *Here's how to / build your own / AirTag for under / 15 dollars.*
  2. 4–6 s: fundo clareia, celulares com brilho laranja; "MILLIONS OF iPHONES" → muitos ícones ("HELP FIND IT").
  3. 6–9 s: "ONE BOARD", objeto com rótulo ("PRO MICRO nRF52840") e um dedo ao lado para dar a **escala**.
  4. 9–17 s: aparece o **indicador de passos** (1 FLASH · 2 KEY · 3 WEB APP); janela de terminal com comando
     digitado e **barra de progresso laranja** (21 % → 99 %); ao fim, a pílula 1 vira verde com ✓.
  5. 17–22 s: passo 2 (chave): comando em uma linha, ícone de chave; "ONE COMMAND"; "Now it's shouting" com
     celulares ao redor do objeto.
  6. 22–27 s: passo 3: janela de navegador (creme) com mapa, modal "Sign in to Apple", pino no mapa
     ("YOUR TAG, ON A MAP").
  7. 27–30 s: cartão **"FAIR WARNING"** com borda âmbar; itens entram um a um (✕ "No ready-made download yet",
     ▸ "Build it from source").
  8. 30–33 s: "YOUR STUFF ONLY" com chips ✓ (KEYS, BAG, BIKE) e **chamada final**: dois botões
     ("I'D BUILD ONE" / "NOT FOR ME") + "COMMENT BELOW".
- **Visual:** fundo estático (cidade vista de cima) **escurecido com vinheta forte** nas bordas; objetos planos
  com contorno escuro e detalhes dourados; janelas com os 3 botões coloridos; títulos em **caixa alta, fonte
  pesada, dourado com contorno/sombra escura**; subtítulo na base em branco com palavra atual em laranja.
- **Cores (aproximadas):** azul-marinho `#0b111f`, dourado `#f1d390`/`#d4ae6b`, creme `#f6efe2`/`#e8dfce`,
  cinza-oliva do fundo `#4e4e4a`–`#828073`, laranja `#f07a1a` e verde `#2fa364` *(estes dois, por olho)*.

## 4. Design system do vídeo (tokens e componentes)

**Tokens (proposta; ajustar com a marca do Taiga Assist):**
`--navy #0b111f`, `--navy-2 #1c2332`, `--gold #f1d390`, `--cream #f6efe2`, `--laranja #f07a1a`,
`--verde #2fa364`, `--vermelho #d9503f`, `--azul-app #2f6fed` (azul do app, para a janela do Taiga Assist).
Tipografia: títulos Poppins/Inter **Black** em caixa alta; corpo Inter; terminal JetBrains Mono.

**Componentes (cada um vira uma pré-composição/modelo `.ectemplate` no EffectCraft, parametrizado por texto/cor; fallback: componente no plano B):**

| Componente | Comportamento |
|---|---|
| `Fundo` | Kanban visto de cima: colunas e cartões desenhados em SVG, escurecido; vinheta forte; variações (zoom/pan lento, clarear por cena) |
| `Titulo` | Rótulo pequeno ("PARTE N") + título grande em 1–2 linhas; palavra-chave em dourado; contorno/sombra escura; entrada com escala+fade |
| `Pilulas` | Indicador de passos; estados **pendente** (escuro) / **atual** (laranja) / **feito** (verde com ✓); transição suave |
| `Janela` | Moldura com 3 botões; variantes **terminal** (escuro, mono), **navegador** (creme, barra de URL), **app** (Taiga Assist) e **Taiga** (Kanban fictício) |
| `Terminal` | Texto digitado caractere a caractere; barra de progresso laranja; linha de resultado em azul-claro |
| `Cartao` | Cartão com borda âmbar (ex.: aviso); itens entram um a um com ✓/✕ |
| `Chips` / `Botao` | Chips com ✓; par de botões de chamada final |
| `Legenda` | Lê `words.json`; mostra 2–3 palavras por vez; destaca a palavra falada em laranja; posição ~62 % da altura |
| `Cena` | Orquestra a entrada/saída (fade ~10 quadros) dos elementos por linha do tempo |

**Regras de movimento:** entrada = escala 0,92→1 + fade em ~8 quadros com *ease-out*; transição entre cenas =
fade cruzado ~10 quadros; texto digitado ~25–35 caracteres/s; nunca mais de ~2 elementos novos ao mesmo tempo.

**Marca NTI Sistemas:** texto **"NTI Sistemas"** em **branco com borda preta** (contorno ~4 px), fonte pesada, sem logo.
Aparece **no capítulo 1** (junto da frase que o cita) e **no cartão final**; não há marca d'água permanente.

**Dois layouts por componente (V = vertical 1080×1920, H = horizontal 1920×1080):** título no topo (V) ou
canto superior esquerdo (H); janela ocupando a largura (V) ou ~60 % da largura, à direita (H); pílulas no topo (V)
ou na lateral esquerda (H); legendas a ~62 % (V) ou ~80 % da altura (H). Os dois formatos usam a mesma arte e os
mesmos tempos.

## 5. O vídeo único

### 5.0 Taiga fictício (recomendação adotada)
Kanban **inspirado no Taiga real e simplificado**, com marca própria: colunas **Novo · Em andamento · Pronto · Fechado**;
cartões com número (`#312`), título curto, selos de pontos (UX/Front/Back) e avatar com iniciais; projeto
**"Portal de Eventos"**, usuária **"Ana Souza"**; US da reunião: **"Reunião semanal · 12/11 · Ana Souza"**. Sem o
logo nem as cores exatas do Taiga. Todos os dados são fictícios (nenhum dado real aparece no vídeo).

### 5.1 Capítulos e tempos (alvo: 178 s)

| # | Capítulo | Início–fim | Duração |
|---|---|---|---|
| 1 | O problema e o Taiga Assist | 0–30 s | 30 s |
| 2 | Instalar | 30–46 s | 16 s |
| 3 | Configurar (5 passos) | 46–80 s | 34 s |
| 4 | Commit e push → relatório → US no Taiga | 80–108 s | 28 s |
| 5 | Revisar e fechar | 108–130 s | 22 s |
| 6 | Prestar conta na reunião | 130–164 s | 34 s |
| 7 | Atualizações e fechamento | 164–178 s | 14 s |

> Os tempos são **alvo**: o áudio gerado (`words.json`) define a duração real de cada fala e as cenas se ajustam a ele.

### 5.2 Roteiro completo (visual + narração em pt-BR)

**Capítulo 1: o problema (0–30 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 0–3 | Rótulo e título **CHEGA DE RELATÓRIO MANUAL**; cartão do Taiga com relógio "2 h" | "Quanto tempo você perde escrevendo relatório?" |
| 3–10 | Texto **NTI Sistemas** (branco, borda preta) no topo; pílulas **1 PROGRAMAR · 2 DEPLOY · 3 RELATÓRIO** com barras de tempo (programar é a mais longa) | "No NTI Sistemas, o fluxo era: programar, fazer o deploy e gerar o relatório no Taiga." |
| 10–13 | Barra "programar" destacada | "Programar era o que mais demorava." |
| 13–17 | Terminal com código digitado rápido; a barra de programar encolhe | "Mas com IA, programar ficou muito mais rápido." |
| 17–21 | Pílula DEPLOY vira verde com ✓ (engrenagem) | "E o deploy, a gente já está automatizando." |
| 21–26 | Sobra a pílula RELATÓRIO em vermelho; pilha de cartões; "ATÉ 2 H POR SEMANA" | "Sobrou o relatório: em semana cheia, até duas horas." |
| 26–30 | Ícone e nome **Taiga Assist**; a janela do app aparece | "Por isso criei o Taiga Assist." |

**Capítulo 2: instalar (30–46 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 30–35 | Três ícones (Windows, Mac, Linux) e botão **Baixar** | "Baixe o instalador para o seu sistema: Windows, Mac ou Linux." |
| 35–41 | Aviso genérico do sistema ("app não assinado") → botão **Abrir mesmo assim** | "Na primeira abertura, o sistema pode avisar que o app não é assinado. É só confirmar." |
| 41–46 | Pílulas ✓ **GIT** · ✓ **KIRO CLI** | "Você só precisa ter o Git e o Kiro CLI instalados." |

**Capítulo 3: configurar (46–80 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 46–49 | Pílulas dos 5 passos; janela do app | "A configuração tem cinco passos." |
| 49–52 | Passo 1: Git e Kiro "encontrado" ✓ | "Um: confira os pré-requisitos." |
| 52–57 | Passo 2: escolher a pasta; lista de repositórios | "Dois: escolha a pasta onde ficam os seus projetos." |
| 57–62 | Passo 3: credencial (token / SSH / git do sistema), botão **Testar** ✓ | "Três: cadastre como o app acessa os seus repositórios." |
| 62–71 | Passo 4: login no Taiga, **Selecionar todos** e importar projetos, vincular repositório → projeto + branch | "Quatro: entre na sua conta do Taiga, importe os projetos e ligue cada repositório a um projeto e a uma branch." |
| 71–76 | Passo 5: Kiro logado (e-mail fictício) | "Cinco: entre no Kiro, que escreve os relatórios." |
| 76–80 | Tudo ✓; destaque do menu **Configurações** | "Pronto. E dá para mudar tudo depois, em Configurações." |

**Capítulo 4: commit → US (80–108 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 80–83 | Aba **Pasta local**; repositório com branches | "No dia a dia, abra a Pasta local." |
| 83–88 | Marcar branches; mensagem do commit sendo digitada | "Marque as branches e escreva a mensagem do commit." |
| 88–93 | Caixas **Fazer push** e **Criar US no Taiga** marcadas; botão **Commitar** | "Deixe marcado: fazer push e criar a US no Taiga. Clique em Commitar." |
| 93–101 | Notificação com etapas (commit, push, relatório pelo Kiro, US) | "O app envia o código, o Kiro lê as alterações e escreve o relatório." |
| 101–108 | Taiga fictício: cartão novo aparece em **Novo** com título e pontos | "E a história aparece no Taiga, com título, descrição e pontos." |

**Capítulo 5: revisar e fechar (108–130 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 108–112 | Aba **Taiga US's**; lista de US | "Terminou a entrega? Vá em Taiga US's." |
| 112–117 | US marcada; **Revisar e fechar**; resumo com "Feito" e "Pendências" | "Marque a US e clique em Revisar e fechar. O Kiro resume o que foi feito." |
| 117–122 | Popup **Fechar US no Taiga** | "Confira os pontos e confirme." |
| 122–130 | Kanban: o cartão desliza para **Fechado** | "No Taiga, a US vai para a coluna final. Se o resumo já existe, é só clicar em Fechar." |

**Capítulo 6: prestar conta (130–164 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 130–132 | Título **DIA DA REUNIÃO** | "Dia da reunião." |
| 132–137 | Aba **Prestar conta em reunião**; vários pontos marcados | "Em Prestar conta, marque os pontos que quer apresentar." |
| 137–141 | Botão **Prestar conta e exportar…**; popup pedindo o link | "Clique em Prestar conta e exportar. O app pede o link da US da reunião." |
| 141–150 | **Alt+Tab** → navegador; Kanban das reuniões; cartão "Reunião semanal · 12/11 · Ana Souza"; copiar a URL | "Abra o navegador, vá ao quadro das reuniões, escolha a US com a data e o seu nome, e copie o link." |
| 150–155 | Volta ao app; cola o link; **Exportar** | "Volte ao Taiga Assist, cole o link e exporte." |
| 155–164 | US da reunião aberta: descrição com **Feito** e itens agrupados por US | "Na US da reunião, seus itens já estão na descrição, sob Feito. Prestação de contas em segundos." |

**Capítulo 7: atualizações e fechamento (164–178 s)**

| Tempo | Visual | Narração |
|---|---|---|
| 164–168 | Faixa **Nova versão disponível** com **Atualizar agora / Agora não** | "Quando sai uma versão nova, o app avisa." |
| 168–172 | Destaque dos dois botões | "Você escolhe: atualizar agora ou depois." |
| 172–178 | Cartão final: **Taiga Assist** + texto **NTI Sistemas** + endereço da página de versões | "Taiga Assist. Menos relatório, mais código. Baixe pelo link." |

## 6. Pipeline técnico (para implementar)

```
roteiro (video/roteiros/video.json)          ← capítulos, cenas, narração, marcos (seção 5.2)
   │
   ├─ TTS pt-BR (Francisca) ─────────────────► video/audio/voz.wav  +  voz.words.json (palavra, início, fim)
   │
   ├─ gerador (video/scripts/build.*) ────────► lê roteiro + words.json + layouts e emite, para CADA formato:
   │                                            video/gerado/video-9x16.jsonl   video/gerado/video-16x9.jsonl
   │
   ├─ effectcraft-cli --empty --save-as out/video-9x16.ecproj run --commands gerado/video-9x16.jsonl   (idem 16x9)
   ├─ effectcraft-cli --project out/video-9x16.ecproj render --comp Video --out out/video-9x16.mp4 \
   │                  --format h264 --fps 30 --audio on                                               (idem 16x9)
   └─ áudio: voz + música (ducking) mixados e normalizados (~−14 LUFS) com soundcraft-cli/ffmpeg; `+faststart`
```

- **Pasta:** `video/` na raiz do repositório: `roteiros/` (JSON), `scripts/` (gerador, TTS), `arte/` (SVG do fundo, regerado
  pelos scripts), `gerado/`, `audio/` (voz e `musica/`), `out/` (todos gerados; **no .gitignore**, assim como `*.ecproj`) e
  `musica-licencas.md`.
- **Reprodutibilidade:** o vídeo se refaz do zero com `roteiro + arte + scripts`; nada de editar projeto na mão.
- **Conferência no loop:** `effectcraft-cli render-frame --time T --out f.png` para olhar quadros, como foi feito com a
  referência.
- **Ferramentas de voz** (criar no passo 1 da produção, precisam de internet):
  `python3 -m venv video/.venv && video/.venv/bin/pip install edge-tts` (o `pip3` global não existe; o venv traz o pip).
- **Saídas:** `out/taiga-assist-9x16.mp4` (mestre, Instagram) e `out/taiga-assist-16x9.mp4`.

### 6.1 Voz (decidida)
- **Voz feminina, sintética, pt-BR: `pt-BR-FranciscaNeural`** (reserva: `pt-BR-ThalitaNeural`). Escolha do autor ("feminina");
  o Claude não ouve, então a naturalidade é avaliada pelo autor no vídeo montado. Se não agradar, trocar para a reserva
  muda só um parâmetro e refaz o vídeo.
- **Tempos das palavras:** o `edge-tts` entrega o instante de cada palavra (*WordBoundary*) → `voz.words.json`, que
  alimenta as legendas e a duração das cenas. Ajuste de velocidade permitido (±10 %) para caber no alvo de 178 s.
- **Versão final:** **Azure AI Speech** (mesma voz, **serviço oficial**, camada gratuita mensal; conferir limites atuais).
  Para usar, o autor cria o recurso gratuito e exporta `AZURE_SPEECH_KEY` e `AZURE_SPEECH_REGION`; o pipeline troca a
  fonte do áudio sem mudar mais nada. **Sem chave**, o vídeo é publicado com a mesma voz via `edge-tts`, que usa o
  serviço do Edge de forma **não oficial** (risco de termos assumido pelo autor).
- **Áudio final:** normalizar para ~**−14 LUFS** (referência comum em redes sociais).

## 7. Verificação (como Claude confere sem ouvir/assistir)

1. Renderizar e extrair **folhas de contato** (`ffmpeg ... fps=1,scale=270:-1,tile=6x3`) e **olhar os quadros**,
   como foi feito com a referência (comandos na seção 3 / histórico da sessão).
2. Conferir cada legenda contra o `words.json` (a palavra destacada no quadro certo), as **zonas seguras** e o
   contraste do texto sobre o fundo.
3. Medir o áudio (`ffmpeg -af volumedetect,ebur128`) e a duração (`ffprobe`).
4. **O autor assiste e ouve** cada parte antes de publicar (movimento, ritmo, voz e música só se avaliam assim).

## 8. Decisões fechadas e o que só o autor faz

**Todas as perguntas anteriores foram respondidas (2026-11-07):**

| Tema | Resposta |
|---|---|
| Motor | EffectCraft (Remotion descartado); plano B Revideo/Motion Canvas |
| Voz | Feminina, `pt-BR-FranciscaNeural` |
| Marca | Só o texto "NTI Sistemas", branco com borda preta |
| Música | Pixabay: faixa 1 principal + 2 reservas (2.4) |
| 16:9 | Aprovado como versão derivada do vertical |
| Formato | **Um vídeo único** (capítulos), não série |
| Taiga fictício | Inspirado no real, simplificado, marca própria (5.0) |
| Ambiente | CLIs extraídas, wrappers e MCPs registrados (2.3) |

**O que só o autor faz (não bloqueia o início da produção):**
1. Baixar o **MP3 da faixa 1** do Pixabay para `video/audio/musica/principal.mp3` (necessário só na mixagem final).
2. *(Opcional)* Criar a chave do **Azure AI Speech** e exportar `AZURE_SPEECH_KEY`/`AZURE_SPEECH_REGION` para a voz final oficial.
3. **Assistir e ouvir** o resultado, no fim: movimento, ritmo, voz e música só se avaliam assim.

**Riscos aceitos (registrados):** os apps Craft são **alfa**; a voz do `edge-tts` é não oficial; as faixas não foram
ouvidas; o limite de duração dos Reels precisa ser conferido.

## 9. Entregas (ordem de produção)

1. **Ferramentas de voz:** criar o venv e instalar o `edge-tts`; gerar `voz.wav` e `voz.words.json` do roteiro 5.2.
2. **Piloto 2 (EffectCraft):** legendas palavra a palavra, importar SVG/PNG e áudio, fontes, formas/vinheta, easing.
   Registrar o resultado aqui; se algo bloquear, usar o plano B.
3. **Arte e kit:** SVGs no VectorCraft (`video/arte/`) e os modelos `Fundo`, `Titulo`, `Pilulas`, `Janela`, `Terminal`,
   `Cartao`, `Legenda` (V e H).
4. **Vídeo 9:16 completo**, capítulo por capítulo, **mostrando quadros ao autor** para validar o estilo no capítulo 1
   antes de seguir.
5. **Versão 16:9** a partir dos mesmos dados.
6. **Mixagem:** voz + música (ducking), normalização, `+faststart`.
7. **Verificação** (seção 7) e entrega dos dois MP4.

## 10. Como retomar (e iniciar a produção)

1. Abra uma **nova sessão do Claude Code em `~/Projetos/ci_cd-taiga-assist`** (os MCPs `effectcraft`, `vectorcraft` e
   `soundcraft` só aparecem em sessão nova nessa pasta; confira com `/mcp`).
2. *(Se quiser)* baixe a música (8, item 1) e/ou configure o Azure (8, item 2) antes ou depois: não bloqueiam.
3. Peça ao Claude:
> *"Siga `docs/video-spec.md` a partir da seção 9, passo 1, e produza o vídeo único (9:16 e 16:9). Mostre-me quadros do
> capítulo 1 para validar o estilo antes de continuar."*

Conferir que `ffmpeg` e `node` existem e que `~/.local/bin` está no PATH. Não commitar `video/arte/`, o vídeo de referência nem
`video/out/`, `video/gerado/`, `video/audio/` ou `*.ecproj`.

## 11. Estado final da produção (2026-11-07)

**Feito (16:9, 1920×1080, 30 fps, ~181 s, 7 capítulos)**
- Voz `pt-BR-FranciscaNeural` (edge-tts) com **tabela de pronúncia** em `video/roteiros/pronuncia.json` (guíti, quíro cí él ái,
  deplói, púxi, brântchi, iúzer stóri, ene tê í...). As legendas mantêm o texto original; o tempo vem das palavras faladas.
- Estilo aprovado: fundo Kanban escurecido com vinheta, **colunas animadas** (alternam sobe-e-desce, 26–38 px, ~21 s por ciclo),
  títulos em caixa alta, pílulas, janelas do app/navegador/Taiga fictício, legendas com a palavra falada em laranja.
- **Música:** o arquivo escolhido pelo autor (`zephiramusic-lofi-tech-588185.mp3`, 91 s), não as faixas do Pixabay da seção 2.4.
  Em loop com transição de 3 s, fade-in de 1 s e **fade-out de 3 s no fim**; volume 0,13 com ducking sob a voz (≈13,5 dB abaixo).
  Áudio final −14,2 LUFS, pico −1,5 dBFS.
- Saídas em `video/out/`: `taiga-assist-16x9.mp4` (v1, 149 MB), `taiga-assist-16x9-v2.mp4` (119 MB, fundo animado e voz nova) e
  `taiga-assist-16x9-v2-reduzido.mp4` (13,6 MB; H.264 CRF 22, VMAF 96,6 contra a v2).
- Fluxo adotado: **preparar → conferir quadros (`revisar.sh`) → renderizar só quando o autor pedir**.

**Mudanças em relação à spec original**
- Só o **16:9** foi produzido (o autor pediu); o 9:16 virou pendência.
- `video/arte/` **fica fora do git** (os SVGs são regerados por `fundo.mjs`); no git ficam só `video/roteiros/` e `video/scripts/`.
- A produção usou a **CLI do EffectCraft**, `ffmpeg`, ImageMagick e `rsvg-convert`; as ferramentas MCP (EffectCraft/VectorCraft/
  SoundCraft) **não foram usadas**. Expressões em `text/sourceText` não funcionam; texto digitado e legendas são camadas por passo.
- No mock do vídeo "Prestar conta" aparece como item de menu; no app é uma **subaba de "Taiga US's"**.

**Pendências do vídeo:** 9:16, voz oficial (Azure), `video/musica-licencas.md`, publicar e pôr o link no README.

