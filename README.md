# Taiga Assist

App desktop (Windows, macOS e Linux) para **fazer commit e push em lote, gerar o relatório com o Kiro CLI e manter o
Taiga em dia**: criar a US da entrega, resumir e fechar US e prestar conta na reunião.

## Vídeo de apresentação

Veja em pouco mais de 3 minutos como instalar, configurar e usar (clique na imagem para assistir):

[![Taiga Assist: vídeo de apresentação](https://img.youtube.com/vi/7m5XXEiVybQ/maxresdefault.jpg)](https://youtu.be/7m5XXEiVybQ)

## Para que serve

O fluxo de quem desenvolve era *programar → fazer o deploy → escrever o relatório no Taiga*. Com IA, programar ficou
muito mais rápido, e o deploy já está sendo automatizado. **Sobrou o relatório**: em semana cheia, até duas horas.
O Taiga Assist tira essa parte do caminho: o app envia o código, o Kiro lê as alterações e escreve o relatório, e a
história aparece no Taiga com título, descrição e pontos.

## O que ele faz

- **Pasta local:** escolha as branches do repositório, escreva a mensagem do commit e, com um clique, faça o
  commit e o push, e traga do remoto as branches desatualizadas com **Atualizar do remoto** (só fast-forward, sem merge).
  Com o Kiro e o Taiga configurados, o mesmo clique cria a US no Taiga (o Kiro escreve a descrição e os
  pontos a partir do que mudou).
- **Taiga US's:** liste as suas US, peça ao Kiro um resumo do que foi feito e do que ficou pendente (**Revisar e
  fechar**), confira os pontos e feche a US no Taiga.
- **Prestar conta em reunião:** marque os pontos que quer apresentar e exporte direto para a US da reunião, que recebe
  os itens agrupados por US sob "Feito".
- **Atualização automática:** quando sai uma versão nova, o app avisa e você escolhe **Atualizar agora** ou **Agora
  não**. Banco, credenciais e configurações são mantidos.

Toda ação que altera o Taiga (criar, fechar, editar descrição) pede confirmação antes.

## Instalar

1. Baixe o arquivo do seu sistema na página de versões:
   **<https://github.com/renanrocket/ci_cd-taiga-assist/releases/latest>**

   | Sistema | Qual arquivo baixar | Observação |
   |---|---|---|
   | **Linux** | `.AppImage` | Recomendado: `chmod +x` e abrir. Atualiza sozinho. |
   | **Linux (Debian/Ubuntu)** | `.deb` | Instala pelo apt; só avisa de versão nova. |
   | **macOS, Apple Silicon (M1 a M4)** | `.dmg` com `aarch64` no nome | Macs com chip Apple. |
   | **macOS, Intel** | `.dmg` com `x64` no nome | Macs com processador Intel. |
   | **Windows** | `.exe` (`x64-setup`) | Windows 10/11 (64 bits). |

   A página de cada versão traz uma tabela no topo com o arquivo certo para cada sistema.

   Os arquivos `.sig`, `.tar.gz` e `latest.json` são do atualizador automático; não precisa baixá-los.
2. Na primeira abertura o sistema pode avisar que o app **não é assinado** (Windows SmartScreen, macOS Gatekeeper).
   É só confirmar ("Executar assim mesmo" / "Abrir mesmo assim"). As atualizações seguintes não repetem o aviso.
3. Tenha o **Git** instalado (obrigatório). O **Kiro CLI** é opcional: sem ele o app faz commit e push em lote; com ele
   (e com o Taiga) gera os relatórios e as US.

## Configurar (na primeira abertura)

Só os três primeiros passos são obrigatórios; com eles você já faz **commit e push em lote**. Kiro e Taiga podem ser
pulados e ligados depois em **Configurações**. O Taiga precisa do Kiro (os relatórios e resumos são gerados por ele).

1. **Pré-requisitos:** confira se o Git foi encontrado (o Kiro CLI é opcional).
2. **Pasta local:** escolha a pasta onde ficam os seus projetos.
3. **Credencial do repositório:** cadastre como o app acessa os repositórios (token, chave SSH ou o Git do sistema) e
   clique em **Testar**.
4. **Kiro (opcional):** entre no Kiro, que escreve os relatórios.
5. **Taiga (opcional, precisa do Kiro):** entre na sua conta, importe os projetos e ligue cada repositório a um projeto
   e a uma branch.

Tudo pode ser mudado depois em **Configurações**. Nada fica fixo no código: URLs, branches, papéis, prazos e a US de
destino são configuração de cada pessoa. Segredos ficam só no chaveiro do sistema.

## Desenvolvimento

Tauri 2 (backend em Rust) com interface em SvelteKit e TypeScript. Usa o `git` e o `kiro-cli` **do sistema**.

```bash
npm install
npm run tauri dev                                   # app em desenvolvimento
cargo test --manifest-path src-tauri/Cargo.toml     # testes do backend
npx svelte-check                                    # checagem do front
npm run versao -- X.Y.Z                             # atualiza a versão nos 3 arquivos (depois: commit, tag vX.Y.Z, push)
```

Para publicar uma versão, envie a tag `vX.Y.Z`: o GitHub Actions compila para os três sistemas, assina as
atualizações e cria a Release (passo a passo em [docs/atualizacoes.md](docs/atualizacoes.md)).

## Documentação

- [docs/spec.md](docs/spec.md): especificação e mapa de portabilidade do projeto irmão `ci_cd`.
- [docs/atualizacoes.md](docs/atualizacoes.md): instalação, atualização automática, publicação e chave de assinatura.
- [docs/ui-conventions.md](docs/ui-conventions.md): convenções de interface.
- [docs/video-producao.md](docs/video-producao.md) e [docs/video-spec.md](docs/video-spec.md): vídeo de apresentação.
- [docs/pending.md](docs/pending.md): pendências.
