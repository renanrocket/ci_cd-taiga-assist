# Pendências para o final

Tudo aqui fica para quando o app estiver **completo** (decisão do usuário: nada de instalador/CI agora).

## Distribuição (só no final)
- [ ] **CI no GitHub Actions** gerando os instaladores dos 3 sistemas (`.msi`/`.exe`, `.dmg`, `.deb` + `.AppImage`) e
      publicando numa página de downloads (Releases).
- [ ] **Assinatura**: Windows (certificado de code signing, evita o SmartScreen) e macOS (conta Apple Developer
      + notarização, evita o Gatekeeper). Sem isso, documentar o "abrir assim mesmo" no README.
- [ ] **Atualização automática** (plugin updater do Tauri) para os colegas receberem versões novas.
- [ ] **README para colegas**: instalar, pré-requisitos (Git e Kiro CLI são instalações à parte), primeiros passos.
- [ ] Testar o instalador em Windows e macOS reais (só o `.deb` foi gerado, e só como prova de empacotamento).
- [ ] Revisar `tauri.conf.json` (categoria, descrição curta/longa, `webviewInstallMode` no Windows).

## Vídeo de apresentação
- [ ] Construir a série de vídeos (vertical 9:16, estilo motion design) seguindo [video-spec.md](video-spec.md):
      responder as perguntas da seção 8 (licença do Remotion, voz, marca, música), depois kit da série + Parte 1.
- [ ] O vídeo substitui o README dos colegas (decisão do autor): a explicação de instalação e atualização vai nele.

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
