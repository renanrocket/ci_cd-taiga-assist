# Pendências para o final

Estado em 2026-11-07: app funcional, CI, publicação e vídeo prontos; `v0.2.1` publicada para Windows, macOS e Linux. Faltam os testes em máquina real (Windows e macOS).

## Retomar aqui (2026-11-07)

0. **Kiro e Taiga opcionais** publicado desde a `v0.2.0`. O **vídeo v3 reduzido** já reflete isso (`video/out/taiga-assist-16x9-v3-reduzido.mp4`); falta publicar.

O que **só uma pessoa/máquina real** resolve (nada disso dá para provar na CI):
1. **Windows real:** instalar o `.exe` da última Release e conferir (a CI roda 113 testes e gera o instalador, mas ninguém abriu o app).
2. **macOS real:** abrir o `.dmg` (Apple Silicon e Intel).
3. **Colega no Linux** validando o AppImage: coletar o retorno. (O AppImage da `v0.1.1` já foi aberto na máquina do autor: inicia, cria o banco e fica rodando.)
4. **Atualização automática:** publicar uma versão nova (ex.: `v0.1.2`) e abrir o app da anterior: aparece a faixa? Precisa de uma Release nova.
5. **Taiga real**, **Kiro login**, **credenciais token/usuário+senha/SSH** contra remoto real (ver "Verificações" abaixo).
6. **Vídeo:** publicar e pôr o link no README; depois 9:16, voz oficial (Azure, precisa da chave) e `video/musica-licencas.md` (precisa da origem da música).

## Distribuição
- [x] **CI de testes** (`.github/workflows/ci.yml`) nos 3 sistemas e **workflow de publicação**
      (`.github/workflows/release.yml`): a tag `vX.Y.Z` gera `.exe` (Windows), `.dmg` (macOS Apple Silicon e Intel),
      `.AppImage` e `.deb` (Linux), assina as atualizações e publica a Release com o `latest.json`.
- [x] **Atualização automática** (plugin updater do Tauri), com chave própria de assinatura cadastrada nos Secrets do
      GitHub (`TAURI_SIGNING_PRIVATE_KEY`, sem senha: decisão de 2026-11-07). Ver [atualizacoes.md](atualizacoes.md).
- [x] **README** para colegas (para que serve, instalar, configurar), com resumo do vídeo.
- [x] Revisão do `tauri.conf.json` (versões coerentes nos 3 arquivos, chave pública igual à do `.key.pub`, ícones,
      categoria, descrição, `longDescription`, `publisher` e `copyright`). Opcional, não feito: `webviewInstallMode` do Windows
      (hoje o padrão, que baixa o WebView2 se faltar).
- [x] Workflow **Teste Windows** (`teste-windows.yml`): testes com falhas anotadas + instalador `.exe` como artefato, sem publicar.
- [x] **Windows na `v0.1.1`**: job reativado em `release.yml`. O `cargo test` do Windows falhava com `0xc0000139` (o executável de
      teste da lib não tinha o manifesto Common Controls v6, exigido pelo `tauri-plugin-dialog`); corrigido com `RUSTFLAGS` só no
      passo de teste (`cargo test --lib`, manifesto em `src-tauri/windows-test.manifest`). **Não** usar `cargo:rustc-link-arg-tests`
      no `build.rs`: o cargo rejeita sem alvo de teste de integração e quebra o build. Um teste ajustado para CRLF (autocrlf do Git).
- [ ] **Testar o `.exe` em um Windows real** (a CI roda 113 testes e gera o instalador, mas ninguém abriu o app ainda).
- [x] **Primeira Release (`v0.1.0`)** publicada só com Linux e macOS (AppImage, .deb, 2 .dmg e o `latest.json`). Falta confirmar
      com o colega no Linux e testar num Mac real. O Windows entrou na `v0.1.1`.
- [x] Actions atualizados para `checkout@v7`, `setup-node@v7`, `upload-artifact@v7` (fim do aviso de Node 20); CI verde nos 3 sistemas.
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
- [x] Chave SSH temporária no **Windows**: teste `ssh_aceita_as_permissoes_da_chave_temporaria` gera uma chave com `ssh-keygen`, grava pelo caminho do app e confere que o `ssh` a carrega; passa nos 3 sistemas na CI. Falta só o uso contra um remoto SSH real.

## Funcionalidade
- [x] **Taiga US's** (lista, resumo Kiro "Feito/Pendências", revisão, fechar em lote, prestação de contas e
      exportar para a US de destino digitada na hora) — implementado; falta o teste do usuário no Taiga real.
- [x] Prazo da US (`prazo_dias`) tem tela em Configurações → Geral.

## Possíveis melhorias (depois do uso real)
- [x] **"Atualizar do remoto" (git pull só fast-forward)** ao lado de cada branch desatualizada na Pasta local (`gitops::pull_branch`,
      comando `update_branch`, 4 testes). Publicado na `v0.2.1`; o vídeo (cap. 4) pode citar o botão.
- [ ] Editar/regerar o texto de um ponto do resumo antes de prestar conta (hoje só se regera pelo Kiro).
- [ ] Mostrar na lista de US quantos pontos pendentes cada US tem.
