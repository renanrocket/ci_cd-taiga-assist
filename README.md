# Taiga Assist

App desktop (Windows, macOS e Linux) para **fazer commit e push em lote, gerar o relatório com o Kiro CLI e manter o
Taiga em dia**: criar a US da entrega, resumir e fechar US e prestar conta na reunião.

## Para que serve

O fluxo de quem desenvolve era *programar → fazer o deploy → escrever o relatório no Taiga*. Com IA, programar ficou
muito mais rápido, e o deploy já está sendo automatizado. **Sobrou o relatório**: em semana cheia, até duas horas.
O Taiga Assist tira essa parte do caminho: o app envia o código, o Kiro lê as alterações e escreve o relatório, e a
história aparece no Taiga com título, descrição e pontos.

## O que ele faz

- **Pasta local:** escolha as branches do repositório, escreva a mensagem do commit e, com um clique, faça o
  commit, o push e crie a US no Taiga (o Kiro escreve a descrição e os pontos a partir do que mudou).
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

   | Sistema | Arquivo | Observação |
   |---|---|---|
   | **Linux** | [AppImage](https://github.com/renanrocket/ci_cd-taiga-assist/releases/download/v0.1.1/Taiga.Assist_0.1.1_amd64.AppImage) | Recomendado: `chmod +x` e abrir. Atualiza sozinho. |
   | **Linux (Debian/Ubuntu)** | [.deb](https://github.com/renanrocket/ci_cd-taiga-assist/releases/download/v0.1.1/Taiga.Assist_0.1.1_amd64.deb) | Instala pelo apt; só avisa de versão nova. |
   | **macOS, Apple Silicon (M1 a M4)** | [.dmg (aarch64)](https://github.com/renanrocket/ci_cd-taiga-assist/releases/download/v0.1.1/Taiga.Assist_0.1.1_aarch64.dmg) | Macs com chip Apple. |
   | **macOS, Intel** | [.dmg (x64)](https://github.com/renanrocket/ci_cd-taiga-assist/releases/download/v0.1.1/Taiga.Assist_0.1.1_x64.dmg) | Macs com processador Intel. |
   | **Windows** | [.exe (instalador)](https://github.com/renanrocket/ci_cd-taiga-assist/releases/download/v0.1.1/Taiga.Assist_0.1.1_x64-setup.exe) | Windows 10/11 (64 bits). Se aparecer o SmartScreen: "Mais informações", "Executar assim mesmo". |

   > Links da versão `0.1.1`. Versões mais novas: <https://github.com/renanrocket/ci_cd-taiga-assist/releases/latest>.

   Os arquivos `.sig`, `.tar.gz` e `latest.json` são do atualizador automático; não precisa baixá-los.
2. Na primeira abertura o sistema pode avisar que o app **não é assinado** (Windows SmartScreen, macOS Gatekeeper).
   É só confirmar ("Executar assim mesmo" / "Abrir mesmo assim"). As atualizações seguintes não repetem o aviso.
3. Tenha instalados, à parte, o **Git** e o **Kiro CLI** (o app só confere se eles existem).

## Configurar (cinco passos, na primeira abertura)

1. **Pré-requisitos:** confira se o Git e o Kiro CLI foram encontrados.
2. **Pasta local:** escolha a pasta onde ficam os seus projetos.
3. **Credencial do repositório:** cadastre como o app acessa os repositórios (token, chave SSH ou o Git do sistema) e
   clique em **Testar**.
4. **Taiga:** entre na sua conta, importe os projetos e ligue cada repositório a um projeto e a uma branch.
5. **Kiro:** entre no Kiro, que escreve os relatórios.

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
