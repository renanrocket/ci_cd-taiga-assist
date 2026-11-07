# Pendências para o final

Estado em 2026-11-07: app funcional e publicado (`v0.2.1`, Windows, macOS e Linux), vídeo v3 no ar. Confirmados pelo autor em máquina real:
Windows (instalador e app), login do Kiro, Taiga e atualização automática.

## Retomar aqui

O que ainda **só uma pessoa/máquina real** resolve:
1. **macOS real:** abrir o `.dmg` (Apple Silicon e Intel).
2. **Colega no Linux** validando o AppImage: coletar o retorno.
3. **Vídeo:** voz oficial (Azure, precisa da chave) e `video/musica-licencas.md` (precisa da origem da música). A versão 9:16 foi descartada.
4. Opcional: **assinatura** do Windows e do macOS (ver Distribuição).

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
- [x] **`.exe` testado em um Windows real** (instalador e app): ok, confirmado pelo autor em 2026-11-07.
- [x] **Primeira Release (`v0.1.0`)** publicada só com Linux e macOS (AppImage, .deb, 2 .dmg e o `latest.json`). Falta confirmar
      com o colega no Linux e testar num Mac real. O Windows entrou na `v0.1.1`.
- [x] Actions atualizados para `checkout@v7`, `setup-node@v7`, `upload-artifact@v7` (fim do aviso de Node 20); CI verde nos 3 sistemas.
- [x] **Atualização automática** testada de uma versão para a seguinte (faixa de versão nova, instalar e reabrir): ok.
- [ ] **Instalador no macOS real** (checklist em [atualizacoes.md](atualizacoes.md), seção 6).
- [ ] **Assinatura** do Windows (certificado de code signing, evita o SmartScreen) e do macOS (conta Apple
      Developer + notarização, evita o Gatekeeper). Opcional: sem isso vale o "abrir mesmo assim", já explicado no vídeo.

## Vídeo de apresentação
- [x] **Vídeo único 16:9** produzido (v2 e versão reduzida, ~181 s), com voz Francisca, música em loop e fundo
      animado. Processo e scripts em [video-producao.md](video-producao.md); decisões em [video-spec.md](video-spec.md).
- [x] ~~Versão 9:16~~ descartada: não faz mais parte do escopo.
- [ ] **Voz oficial** (Azure AI Speech), se o autor fornecer a chave; hoje a voz é do `edge-tts` (não oficial).
- [ ] Registrar origem e licença da música em `video/musica-licencas.md`.
- [x] Vídeo v3 publicado (<https://youtu.be/7m5XXEiVybQ>) e linkado no README.

## Verificações que dependem de máquina/pessoa sem login
- [x] **Login do Kiro** testado de verdade: ok, confirmado pelo autor.
- [x] Credenciais **token / usuário+senha / chave SSH** contra remoto real: funcionando, confirmado pelo autor.
- [x] Chave SSH temporária no **Windows**: teste `ssh_aceita_as_permissoes_da_chave_temporaria` gera uma chave com `ssh-keygen`, grava pelo caminho do app e confere que o `ssh` a carrega; passa nos 3 sistemas na CI. Uso contra remoto SSH real confirmado pelo autor.

## Funcionalidade
- [x] **Taiga US's** (lista, resumo Kiro "Feito/Pendências", revisão, fechar em lote, prestação de contas e
      exportar para a US de destino digitada na hora) — implementado e testado no Taiga real: ok.
- [x] Prazo da US (`prazo_dias`) tem tela em Configurações → Geral.

## Possíveis melhorias (depois do uso real)
- [x] **"Atualizar do remoto" (git pull só fast-forward)** ao lado de cada branch desatualizada na Pasta local (`gitops::pull_branch`,
      comando `update_branch`, 4 testes). Publicado na `v0.2.1`; o vídeo (cap. 4) pode citar o botão.
- [ ] Editar/regerar o texto de um ponto do resumo antes de prestar conta (hoje só se regera pelo Kiro).
- [ ] Mostrar na lista de US quantos pontos pendentes cada US tem.
