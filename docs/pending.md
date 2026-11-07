# Pendências para o final

Estado em 2026-11-07: app funcional, CI, publicação e vídeo prontos; falta a primeira Release e os testes em Windows/macOS.

## Distribuição
- [x] **CI de testes** (`.github/workflows/ci.yml`) nos 3 sistemas e **workflow de publicação**
      (`.github/workflows/release.yml`): a tag `vX.Y.Z` gera `.exe` (Windows), `.dmg` (macOS Apple Silicon e Intel),
      `.AppImage` e `.deb` (Linux), assina as atualizações e publica a Release com o `latest.json`.
- [x] **Atualização automática** (plugin updater do Tauri), com chave própria de assinatura cadastrada nos Secrets do
      GitHub (`TAURI_SIGNING_PRIVATE_KEY`, sem senha: decisão de 2026-11-07). Ver [atualizacoes.md](atualizacoes.md).
- [x] **README** para colegas (para que serve, instalar, configurar), com resumo do vídeo.
- [x] Revisão do `tauri.conf.json` (versões coerentes nos 3 arquivos, chave pública igual à do `.key.pub`, ícones,
      categoria e descrição). Opcional, não feito: `longDescription`, `publisher`/`copyright` e
      `webviewInstallMode` do Windows (hoje o padrão, que baixa o WebView2 se faltar).
- [x] Workflow **Teste Windows** (`teste-windows.yml`): testes com falhas anotadas + instalador `.exe` como artefato, sem publicar.
- [ ] **Windows fora da `v0.1.0`** (job desativado em `release.yml`): reativar e publicar na `v0.1.1` depois de corrigir e testar.
- [ ] **CI do Windows falha no `cargo test`** (últimos pushes: Linux e macOS passam, e os 124 testes passam no Linux
      local). Abrir o log em Actions → Testes → windows-latest → "Run cargo test", achar o teste que falha e
      corrigir (suspeita: teste que assume caminhos/ambiente Unix).
- [ ] **Primeira Release (`v0.1.0`)**: tag e push, conferir os 4 jobs e os arquivos publicados.
- [ ] **Testar o instalador em Windows e macOS reais** e a atualização de uma versão para a seguinte (checklist em
      [atualizacoes.md](atualizacoes.md), seção 6). Só o `.deb` foi gerado localmente, como prova de empacotamento.
- [ ] **Assinatura** do Windows (certificado de code signing, evita o SmartScreen) e do macOS (conta Apple
      Developer + notarização, evita o Gatekeeper). Opcional: sem isso vale o "abrir mesmo assim", já explicado no vídeo.

## Vídeo de apresentação
- [x] **Vídeo único 16:9** produzido (v2 e versão reduzida, ~181 s), com voz Francisca, música em loop e fundo
      animado. Processo e scripts em [video-producao.md](video-producao.md); decisões em [video-spec.md](video-spec.md).
- [ ] **Versão 9:16** (Instagram): falta desenhar o layout vertical de cada componente.
- [ ] **Voz oficial** (Azure AI Speech), se o autor fornecer a chave; hoje a voz é do `edge-tts` (não oficial).
- [ ] Registrar origem e licença da música em `video/musica-licencas.md`.
- [ ] Publicar o vídeo e colocar o link no README.

## Verificações que dependem de máquina/pessoa sem login
- [ ] **Botão "Entrar no Kiro"** (login por código de dispositivo) nunca foi exercitado de verdade: o formato real da
      saída do `kiro-cli login --use-device-flow` não foi observado. Testar com quem ainda não está logado.
- [ ] Testar credenciais **token / usuário+senha / chave SSH** contra um remoto real (só o modo "git do sistema" foi
      exercitado) e o `CodeCommit`.
- [ ] Chave SSH temporária no **Windows** (sem `chmod`; o OpenSSH pode reclamar de permissões).

## Funcionalidade
- [x] **Taiga US's** (lista, resumo Kiro "Feito/Pendências", revisão, fechar em lote, prestação de contas e
      exportar para a US de destino digitada na hora) — implementado; falta o teste do usuário no Taiga real.
- [ ] Prazo da US (`prazo_dias`) já tem tela em Configurações → Geral.

## Possíveis melhorias (depois do uso real)
- [ ] Editar/regerar o texto de um ponto do resumo antes de prestar conta (hoje só se regera pelo Kiro).
- [ ] Mostrar na lista de US quantos pontos pendentes cada US tem.
