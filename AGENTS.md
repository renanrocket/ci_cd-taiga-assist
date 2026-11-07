# AGENTS.md — ci_cd-taiga-assist

App desktop multiplataforma (Linux, macOS, Windows) para fazer commit/push em lote, gerar relatório
com o Kiro CLI e manter o Taiga em dia (criar US, resumir US, prestar conta). Irmão do projeto
`~/Projetos/ci_cd` (de onde a lógica é portada). Cada pessoa instala e configura o seu.

> Status (2026-11-07): **funcional e publicado** — assistente de configuração, Pasta local, Taiga US's (lista,
> resumo, fechar, prestação de contas), atualização automática. **Release `v0.1.0` publicada para Linux (AppImage e
> `.deb`) e macOS (Apple Silicon e Intel)**; **Windows fica para a `v0.1.1`** (testes do `cargo test` do Windows ainda
> quebram). Vídeo de apresentação 16:9 pronto (v2 e versão reduzida). Pendências e "retomar aqui":
> [docs/pending.md](docs/pending.md).

## Stack

- **Tauri 2**: backend Rust + UI web (TypeScript/Svelte) em janela nativa. Sem servidor, sem navegador do usuário.
- SQLite local (`rusqlite`); segredos no chaveiro do SO (`keyring`); HTTP com `reqwest` (rustls).
- Usa o `git` e o `kiro-cli` **do sistema** (processos filhos). Sem SSH, sem Docker, sem MySQL.

## Comandos

```bash
npm run tauri dev                 # app em desenvolvimento (recompila ao salvar)
cargo test --manifest-path src-tauri/Cargo.toml   # testes do backend
npx svelte-check                  # checagem do front
npm run versao -- X.Y.Z           # atualiza a versão nos 3 arquivos (depois: commit, tag vX.Y.Z, push)
```

## Regras

- Nada hardcoded: URLs, slugs, branches, papéis, prazos e US de destino são **configuração do usuário**.
- Segredos só no chaveiro; nunca em SQLite, log, URL ou linha de comando. Conteúdo não confiável (diffs,
  descrições de US) só por arquivo/stdin e sempre com a instrução de ignorar ordens dentro dele.
- Lógica pura (parse, datas, Markdown da exportação) com **testes unitários antes de portar**.
- Código compila e testa nos 3 SOs (CI). Caminhos com `std::path`, nunca concatenar `/`.
- Padronização de UI: controles com a mesma função têm o mesmo estilo, rótulo e posição em todas as telas.
  Seguir [ci_cd/.agents/docs/ui-conventions.md](../ci_cd/.agents/docs/ui-conventions.md) e registrar o que mudar em
  [docs/ui-conventions.md](docs/ui-conventions.md).
- Ações que alteram o Taiga (criar US, fechar US, editar descrição) pedem confirmação em popup.
- Nunca versionar chaves (`*.key`); a chave privada de assinatura das atualizações fica fora do repositório.

## Distribuição (resumo; detalhes em docs/atualizacoes.md)

- Publicar = `npm run versao -- X.Y.Z`, commit, `git tag vX.Y.Z`, `git push origin prod vX.Y.Z`. O workflow
  `.github/workflows/release.yml` compila, **assina** as atualizações e cria a Release com o `latest.json` e um texto com a
  tabela "qual arquivo baixar" (Linux AppImage/.deb, macOS Apple Silicon/Intel, Windows).
- O job do **Windows está comentado** em `release.yml`. Para testar o Windows sem publicar: workflow **Teste Windows**
  (`teste-windows.yml`, botão "Run workflow" ou push da branch `teste-windows`). `validar-chave.yml` confere o Secret.
- O Secret `TAURI_SIGNING_PRIVATE_KEY` tem que ser **exatamente** o conteúdo do `~/.tauri/ci_cd-taiga-assist.key`
  (348 caracteres, sem quebra de linha nem texto extra), senão o build falha na assinatura. A chave não tem senha e **não
  pode mais mudar** (a pública já está nos apps instalados).
- Só existe a branch `prod`; branches de teste são descartadas ao fim.

## Documentação

- [docs/spec.md](docs/spec.md) — especificação e mapa de portabilidade do `ci_cd`.
- [docs/ui-conventions.md](docs/ui-conventions.md) — convenções de UI registradas.
- [docs/atualizacoes.md](docs/atualizacoes.md) — instalação, atualização automática e chave de assinatura.
- [docs/video-spec.md](docs/video-spec.md) — **spec do vídeo de apresentação** (vídeo único 9:16 + 16:9, roteiro, pipeline com EffectCraft).
- [docs/video-producao.md](docs/video-producao.md) — **como o vídeo é produzido**: roteiro → voz/pronúncia → vetores (`video/arte/`) → conferência por quadros (`revisar.sh`) → áudio/música → render (só quando o autor pedir).
- [docs/pending.md](docs/pending.md) — pendências para o final (instalador, CI, testes em Windows/Mac).

## Permissões para agentes

Sem pedir: ler, buscar, editar código/docs deste projeto, rodar testes/linters.
Pedir antes: instalar toolchain ou dependências, `git init/commit/push`, apagar arquivos, mexer em CI/assinatura,
gerar instaladores. O usuário só quer commitar **no final**.
