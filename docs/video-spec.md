# Vídeo de apresentação do Taiga Assist — spec

> Documento para **construir o vídeo em outra sessão** (ou por outra pessoa). Tudo o que foi descoberto e
> decidido em 2026-11-07 está aqui. Nenhum código de vídeo existe ainda: este arquivo é o ponto de partida.
> Como retomar: ver a seção 10.

## 1. Objetivo e público

- **Objetivo:** ensinar colegas de trabalho (desenvolvedores do NTI Sistemas) a instalar e usar o Taiga Assist,
  de forma didática e bonita, no lugar de um README.
- **Problema contado no vídeo:** o fluxo era *programar → deploy → relatório no Taiga*. Com IA, programar ficou
  rápido; o deploy já está sendo automatizado; **sobrou o relatório** (em semana cheia, até ~2 h). O Taiga Assist
  resolve isso.
- **Onde será visto:** principalmente no **Instagram** (celular), e também compartilhado entre colegas.

## 2. Decisões tomadas

| Tema | Decisão |
|---|---|
| **Orientação** | **Vertical 9:16, 1080×1920, 30 fps**, H.264 + AAC (MP4). Ver 2.1 |
| **Formato** | **Série de partes curtas (20–45 s)**, não um vídeo longo (estilo Reels; a referência tem 33 s e é "PART 2") |
| **Estilo visual** | *Motion design* de **objetos vetoriais planos**, **sem personagens** (a referência não tem; personagem animado é o ponto mais caro e arriscado) |
| **Fundo** | **Quadro Kanban visto de cima** (colunas e cartões), escurecido, com vinheta forte (equivale ao mapa da cidade da referência) |
| **Legendas** | Frases curtas (2–3 palavras por vez), brancas, com **a palavra falada destacada em laranja**, sincronizadas palavra a palavra |
| **Telas do app** | **Mocks vetoriais simplificados** (rótulos grandes, dados fictícios), nunca dados reais; Taiga também fictício |
| **Voz** | **Sintética e bem natural** (decisão do autor). Escolher por **amostras às cegas** (seção 6.1). `edge-tts` só para protótipo/tempos; versão final por serviço com licença oficial (ex.: Azure AI Speech, mesmas vozes pt-BR) |
| **Marca** | **NTI Sistemas** aparece (nome e logo). **Pendente:** arquivo do logo, cores da marca e OK da empresa para uso público |
| **Música** | Instrumental leve, estilo tech/lo-fi (~100–120 BPM), **baixa sob a voz**; fonte com licença clara (2.4) |
| **Ferramenta** | **Remotion** *ou* **Revideo/Motion Canvas (MIT)**. Decisão pendente por causa da licença do Remotion (2.2) |

### 2.1 Por que vertical (e a ressalva)
- O Instagram prioriza **Reels em 9:16**; o 16:9 aparece pequeno no celular. Vertical também funciona em Stories e
  status do WhatsApp.
- **Ressalva:** o app é **desktop** (tela larga). Em 1080 px de largura, uma janela inteira do app fica com
  texto pequeno. Mitigação: mocks **simplificados** (poucos elementos, texto grande), **zoom** em cada parte da
  tela, e a janela ocupando a largura toda como na referência.
- **Zona segura:** o feed corta o vídeo vertical para 4:5 e o Reels cobre a base com a interface do Instagram.
  Manter o texto importante na faixa central (o 4:5 do centro) e as legendas por volta de 60–65 % da altura, como
  na referência. *Conferir com um teste de upload antes de fechar a série.*
- Quem vai instalar **seguindo o vídeo ao lado do computador** se beneficia do horizontal. Como o código é
  parametrizado, uma **exportação 16:9 (1920×1080)** pode ser feita depois com layouts responsivos (custo extra
  de ajuste e revisão; não faz parte da primeira entrega).

### 2.2 Licença das ferramentas (verificar antes de usar)
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
    usados após cancelar. Dá para pagar **1 mês**, produzir a série e cancelar.
  - A licença do Remotion **não cobre patentes de codecs** (H.264/AAC), um assunto à parte e comum a qualquer
    software de vídeo.
- **Plano B:** **Revideo** (fork do Motion Canvas com renderização sem interface) ou **Motion Canvas**, ambos MIT.
  Mais trabalho manual nos elementos de interface (janelas, cartões), pois desenham em canvas.
- **Fontes:** usar OFL (Inter ou Poppins Black para títulos; JetBrains Mono para terminal).
- **Música:** ver 2.4.
### 2.3 Suíte ArtCraft/Craft (instalada na máquina do autor) — em avaliação
- **O que é:** suíte gratuita e open source (Apache-2.0/MIT; site getartcraft.com), apps nativos em Rust, **cada app
  com um servidor MCP** (CLI `<app>-cli mcp`). O *launcher* em si não tem MCP; os apps instalados por ele têm.
  Não confundir com o "ArtCraft Studio" (geração de imagem/vídeo com IA, repositório storytold/artcraft).
- **Apps úteis aqui:** **VectorCraft** (ilustração vetorial; SVG/PNG), **FilmCraft** (edição de vídeo),
  **EffectCraft** (motion graphics com keyframes), **SoundCraft** (áudio; `bounce_mix` mede pico e LUFS),
  **AniCraft** (animação 2D). **Instalados em 2026-11-07:** VectorCraft 0.22.0, PhotoCraft 0.21.0, CADCraft 0.18.0
  (Flatpak; a CLI fica dentro do Flatpak). FilmCraft/EffectCraft/SoundCraft **não** estão instalados.
- **Maturidade:** todos declarados **alfa/em desenvolvimento**; aviso de comportamento imprevisível;
  `command_batch` **não é transacional**; o MCP "não é uma fronteira de autorização" e FilmCraft/LightCraft não
  confinam pastas (usar pasta dedicada e trabalhar em cópia). Não foi encontrado: **legendas, exportação MP4,
  proporção 9:16** (fontes secundárias; conferir no app).
- **Decisão provisória:** o motor do vídeo continua sendo **composição por código** (opção livre: Revideo ou
  Motion Canvas, sem o problema da licença do Remotion), porque o vídeo exige ~35 legendas sincronizadas por parte,
  7 partes com o mesmo visual e render reproduzível/versionável. As ferramentas Craft entram como **complemento**:
  VectorCraft (arte em SVG para a composição) e SoundCraft (mixagem e medição de LUFS).
- **Piloto antes de decidir se dá para usar só Craft:** (a) VectorCraft via MCP desenha um ícone e exporta SVG;
  (b) EffectCraft cria um projeto 1080×1920 com texto + keyframes e exporta MP4 com áudio; (c) FilmCraft importa clipe e
  áudio e exporta MP4 9:16. Se (b) e (c) passarem, reavaliar. Registrar o MCP altera a configuração do Claude Code
  do autor (escopo local, fora do repositório): pedir confirmação antes.

### 2.4 Música (licença)
- **Mixkit** (<https://mixkit.co/free-stock-music/>): a própria página diz que a licença permite uso comercial, sem
  atribuição. **Pixabay Music:** fontes secundárias dizem uso comercial sem crédito; **conferir na página de
  licença do Pixabay**. **YouTube Audio Library:** parte das faixas exige crédito e a licença vale para o YouTube
  (não confirmado para Instagram): evitar como fonte principal.
- **Instagram:** a opção mais segura para música *dentro do Instagram* é adicioná-la pelo próprio app ao publicar
  (biblioteca licenciada pela plataforma; contas comerciais têm biblioteca limitada). *Não verificado.* Se a
  música for embutida no arquivo, usar só faixas com licença clara.
- **Regra de produção:** para **cada faixa usada**, guardar no repositório (`video/musica-licencas.md`) o nome, a URL,
  a data e um print da licença. Misturar a música **baixa sob a narração** (ducking, ~−18 a −22 dB sob a voz).
- **Escolha:** o Claude não ouve áudio. Separar 3 candidatos (busca: "corporate tech", "upbeat technology",
  "lo-fi productivity", instrumental, sem vocais) e o autor escolhe ouvindo.

- **O vídeo de referência é só inspiração.** Não reaproveitar arte, texto nem áudio dele (é obra de terceiros).

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

## 4. Design system da série (tokens e componentes)

**Tokens (proposta; ajustar com a marca do Taiga Assist):**
`--navy #0b111f`, `--navy-2 #1c2332`, `--gold #f1d390`, `--cream #f6efe2`, `--laranja #f07a1a`,
`--verde #2fa364`, `--vermelho #d9503f`, `--azul-app #2f6fed` (azul do app, para a janela do Taiga Assist).
Tipografia: títulos Poppins/Inter **Black** em caixa alta; corpo Inter; terminal JetBrains Mono.

**Componentes (cada um vira um componente React no Remotion):**

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

## 5. A série (partes)

| Parte | Assunto | Duração |
|---|---|---|
| 1 | O problema e o Taiga Assist | ~35 s |
| 2 | Instalar (e o aviso de segurança do sistema na primeira abertura) | ~40 s |
| 3 | Configurar (os 5 passos do assistente, com dados fictícios) | ~45 s |
| 4 | Commit e push → relatório pelo Kiro → US no Taiga | ~40 s |
| 5 | Revisar e fechar → conferir no Kanban | ~35 s |
| 6 | Prestar conta na reunião (marcar, exportar, Alt+Tab, Kanban das reuniões, copiar URL, colar, ver "Feito") | ~45 s |
| 7 | Atualizações ("Atualizar agora / Agora não") | ~20 s |

### 5.1 Parte 1: roteiro (rascunho; narração em pt-BR)

| Tempo | Visual | Narração / legenda |
|---|---|---|
| 0–3 s | "PARTE 1" + título **CHEGA DE RELATÓRIO MANUAL**; cartão do Taiga com relógio "2 h" | "Quanto tempo você perde escrevendo relatório?" |
| 3–10 s | `Pilulas` **1 PROGRAMAR · 2 DEPLOY · 3 RELATÓRIO**, com barras de tempo (programar é a mais longa) | "Antes: programar, fazer o deploy e gerar o relatório. Programar era o que mais demorava." |
| 10–17 s | `Terminal` com código digitado rápido; a barra de "programar" encolhe | "Com IA, programar ficou muito mais rápido." |
| 17–23 s | Pílula DEPLOY vira verde ✓ (ícone de engrenagem) | "O deploy, a gente já está automatizando." |
| 23–29 s | Sobra a pílula RELATÓRIO em vermelho; pilha de cartões; "ATÉ 2 H POR SEMANA" | "Sobrou o relatório. Em semana cheia, são até duas horas." |
| 29–35 s | Ícone + nome **Taiga Assist** entram; janela do app aparece; chamada "Parte 2: como instalar" | "Por isso criei o Taiga Assist. Commit, relatório e prestação de contas, sem retrabalho." |

*(Citar "NTI Sistemas" no texto/visual: decisão pendente, seção 8.)*

### 5.2 Partes 2–7: pontos obrigatórios (para detalhar no momento de produzir)
- **2 Instalar:** baixar o instalador na página de versões; abrir; aviso do SmartScreen/Gatekeeper ("abrir mesmo
  assim"); pré-requisitos Git e Kiro CLI (instalações à parte).
- **3 Configurar:** pré-requisitos → pasta local → acesso aos repositórios (token/SSH/"git do sistema") → conta Taiga
  (login + importar projetos + vincular repositório, projeto e branch gatilho) → Kiro (login ou API key).
- **4 Commit→US:** marcar branches, mensagem, **"Fazer push"** e **"Criar US no Taiga"** marcados, Commitar; notificação
  com etapas; a US aparece no Taiga (fictício).
- **5 Fechar:** em "Taiga US's → User Stories (US)", marcar a US, **Revisar e fechar** (resumo do Kiro) ou **Fechar**
  direto se já tem resumo; popup de confirmação; voltar ao Kanban e ver a US na coluna final.
- **6 Prestar conta:** em "Prestar conta em reunião" marcar vários pontos → **Prestar conta e exportar…** → popup pede
  a URL → **Alt+Tab** → navegador → Kanban das reuniões → escolher a US da reunião (data + nome) → copiar a URL → voltar →
  colar → exportar → abrir a US e ver os itens sob "Feito".
- **7 Atualizações:** faixa "Nova versão disponível", **Atualizar agora** / **Agora não**; dados mantidos.

## 6. Pipeline técnico (para implementar)

```
roteiro (video/roteiros/parteN.json)        ← cenas, texto da narração, marcos de tempo
   │
   ├─ TTS pt-BR (edge-tts) ───────────────► video/public/parteN.mp3  +  parteN.words.json (palavra, início, fim)
   │
   └─ composição Remotion (React) ────────► lê words.json (legendas) + linha do tempo das cenas
          npx remotion render ParteN out/parteN.mp4 --width 1080 --height 1920 --fps 30
   │
   └─ pós: ffmpeg (loudnorm no áudio, H.264/AAC, +faststart) → video/out/parteN.mp4
```

- **Pasta:** `video/` na raiz do repositório, com **package.json próprio** (não misturar com o app Tauri);
  `video/src/` (composições e componentes), `video/public/` (áudio, fontes), `video/roteiros/` (JSON),
  `video/scripts/tts.py` (gera áudio e `words.json`), `video/out/` (saída; **no .gitignore**).
### 6.1 Escolha da voz (natural)
- O Claude **não ouve**; a escolha é por **amostras às cegas**: o mesmo parágrafo de ~15 s gerado por 4–5 vozes
  (rotuladas A, B, C…), e o autor escolhe ouvindo.
- **Candidatas:** vozes neurais pt-BR da Microsoft (Francisca, Antonio, Thalita; via `edge-tts` para testar),
  **Azure AI Speech** (mesmas vozes, **serviço oficial** com camada gratuita mensal; conferir limites atuais),
  Google Cloud TTS (vozes "Chirp 3 HD"; conferir camada gratuita), e abertas: **Kokoro** (Apache-2.0),
  **Chatterbox multilíngue** (MIT; pesado sem GPU), **Piper** (MIT; mais "robótico"). **Evitar XTTS-v2** (licença
  não comercial). ElevenLabs soa muito natural, mas **o plano gratuito tem restrições de uso comercial: conferir**.
- **Cuidado com o `edge-tts`:** usa o serviço online do Microsoft Edge de forma **não oficial**; serve para
  protótipo, mas para vídeo da empresa preferir um serviço com licença/termos oficiais.
- Pesquisa não achou comparação direta e confiável de qualidade pt-BR entre as abertas: **testar de ouvido**.

- **Voz:** `edge-tts` com vozes pt-BR (`pt-BR-FranciscaNeural`, `pt-BR-AntonioNeural`, `pt-BR-ThalitaNeural`); o
  evento *WordBoundary* dá o tempo de cada palavra. Com voz gravada pelo autor, alinhar o áudio ao texto
  (ex.: `faster-whisper`) para gerar o `words.json`. Ouvir os candidatos e escolher é decisão do autor.
- **Áudio final:** normalizar para ~**−14 LUFS** (referência comum para redes sociais) com `ffmpeg loudnorm`.
- **Telas do app:** componentes React que **imitam** a UI (versões simplificadas). Opcional: gravar o app real com
  dados fictícios (`@tauri-apps/api/mocks` → `mockIPC` no navegador + Playwright) e usar o vídeo como camada.

## 7. Verificação (como Claude confere sem ouvir/assistir)

1. Renderizar e extrair **folhas de contato** (`ffmpeg ... fps=1,scale=270:-1,tile=6x3`) e **olhar os quadros**,
   como foi feito com a referência (comandos na seção 3 / histórico da sessão).
2. Conferir cada legenda contra o `words.json` (a palavra destacada no quadro certo), as **zonas seguras** e o
   contraste do texto sobre o fundo.
3. Medir o áudio (`ffmpeg -af volumedetect,ebur128`) e a duração (`ffprobe`).
4. **O autor assiste e ouve** cada parte antes de publicar (movimento, ritmo, voz e música só se avaliam assim).

## 8. Perguntas em aberto

1. **Remotion:** pagar 1 mês de seat (US$ 25) pelo NTI e confirmar com a Remotion, **ou** usar Revideo/Motion Canvas (MIT)?
2. **Voz:** *respondido: sintética e natural.* Falta **ouvir as amostras** (6.1) e escolher.
3. **Marca:** *respondido: NTI Sistemas.* Falta o **arquivo do logo**, as **cores da marca** e o **OK da empresa** para uso
   público (Instagram). Qual logo do Taiga Assist usar (`assets/icon.svg`)?
4. **Música:** *respondido: instrumental gratuita e compatível.* Falta escolher entre 3 candidatas (2.4).
5. **Exportação 16:9** extra? (custo de layout responsivo)
6. Série em **7 partes** está ok, ou juntar/dividir?
7. Como o Taiga **fictício** deve parecer (Kanban genérico ou lembrar o visual real do Taiga)?

## 9. Entregas (ordem sugerida)

1. **Kit da série:** `Fundo`, `Titulo`, `Pilulas`, `Janela`, `Terminal`, `Cartao`, `Legenda` + tokens.
2. **Parte 1 completa** (narração fechada, voz, legendas, render) e **validação de estilo** pelo autor com quadros.
3. Partes 2–7, reaproveitando o kit.
4. Pós: normalização de áudio, exportação, teste de upload no Instagram (zonas seguras).

## 10. Como retomar numa sessão futura

Diga ao Claude: *"Siga `docs/video-spec.md` e construa o kit da série e a Parte 1."* Antes, responda (ou deixe o
Claude perguntar) as perguntas da seção 8. Conferir que `ffmpeg` e `node` estão instalados e, se for usar
Remotion, que a licença foi verificada. Não commitar o vídeo de referência nem arquivos de `video/out/`.
