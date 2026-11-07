# Instalação e atualizações — documento de alinhamento

> Rascunho para alinhar antes de escrever o README dos colegas. Descreve o que o app **já faz** e o que
> ainda precisa ser **testado em máquina real** antes de prometer.

## 1. Em uma frase

O colega instala **uma vez**. Depois, quando você publica uma versão nova, o app **avisa** e ele escolhe
**Atualizar agora** ou **Agora não**. Atualizar é **no lugar**: não desinstala nem baixa instalador de novo, e
mantém banco, credenciais e configurações.

## 2. O que o colega vê

1. Ao abrir o app (e a cada 6 h, se ele ficar aberto), o app consulta se há versão nova.
2. Se houver, aparece uma faixa no topo: **"Nova versão disponível: X.Y.Z (você está na A.B.C)"**, com
   "O que mudou" (opcional) e dois botões:
   - **Atualizar agora**: baixa (com barra de progresso), confere a assinatura, instala e reinicia o app.
   - **Agora não**: some até a próxima abertura do app.
3. Também existe **Configurações → Geral → Verificar atualizações**, que mostra "Você já está na versão mais
   recente" ou avisa da nova.
4. Sem internet ou sem versão publicada: nada aparece (a verificação automática falha em silêncio).

| Sistema | Como atualiza | Observação |
|---|---|---|
| Windows | **Automático**: instalador `.exe` por usuário (sem pedir administrador), progresso na tela | O Windows pode mostrar o aviso do SmartScreen na **primeira instalação** (app sem certificado) |
| macOS | **Automático**: troca o `.app` | O macOS pede "abrir mesmo assim" na **primeira instalação** (sem conta Apple Developer). Comportamento das atualizações sem assinatura: **a testar** |
| Linux (AppImage) | **Automático**, igual aos outros | Formato recomendado para Linux |
| Linux (.deb) | **Só avisa** e leva à página de download | Um pacote instalado pelo apt não pode ser trocado pelo próprio app |

## 3. Como publicar uma versão (para quem mantém)

1. `npm run versao -- 0.2.0` — atualiza o número em `tauri.conf.json`, `package.json` e `Cargo.toml`.
2. Commit, `git tag v0.2.0` e `git push origin prod v0.2.0`.
3. O GitHub Actions (`.github/workflows/release.yml`) compila para Windows, macOS (Apple Silicon e Intel) e
   Linux, **assina** os pacotes de atualização e cria a Release com os instaladores e o `latest.json`.
   O workflow **recusa** a publicação se a tag não bater com a versão do app.
4. Os apps instalados passam a ver a versão nova na próxima verificação.

O repositório é público, então o próprio **GitHub Releases** serve os arquivos: não há servidor para manter.
A URL consultada pelos apps é
`https://github.com/renanrocket/ci_cd-taiga-assist/releases/latest/download/latest.json`.

## 4. Chave de assinatura das atualizações (importante)

- É uma chave **própria do Tauri** (gratuita; não é a assinatura do Windows/Mac). Garante que só quem tem a
  chave privada consegue publicar atualizações aceitas pelos apps.
- A parte **pública** está no `tauri.conf.json` (`plugins.updater.pubkey`) e vai dentro de cada app instalado.
- A parte **privada** está em `~/.tauri/ci_cd-taiga-assist.key` (fora do repositório, `.gitignore` bloqueia `*.key`).
  Ela precisa ir para o GitHub em **Settings → Secrets and variables → Actions**:
  - `TAURI_SIGNING_PRIVATE_KEY` = conteúdo do arquivo `.key`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = senha da chave (vazia se a chave foi gerada sem senha)
- **Guarde uma cópia em local seguro** (gerenciador de senhas). Se perder a chave, os apps já instalados **não
  aceitam mais atualizações** e todo mundo teria que reinstalar à mão. Se vazar, qualquer pessoa pode
  publicar uma "atualização" que os apps aceitam.
- **Decisão pendente:** a chave atual foi gerada **sem senha** (para poder ser usada no CI sem interação).
  Como nada foi distribuído ainda, dá para gerar outra, com senha, antes do primeiro lançamento
  (`npx tauri signer generate -w ~/.tauri/ci_cd-taiga-assist.key`) e atualizar a chave pública no
  `tauri.conf.json`. **Depois do primeiro lançamento a chave não pode mais mudar.**

## 5. Cuidados e limites

- **A primeira versão que você distribuir já precisa ter o atualizador** (ela tem). Quem instalar uma versão
  sem ele nunca receberia o aviso.
- A tag e a versão do app **têm que coincidir** (o workflow confere); versão menor ou igual à instalada não é
  oferecida.
- Se o usuário não atualizar, o app continua funcionando normalmente na versão antiga.
- Mudanças no banco: as migrações rodam sozinhas na primeira abertura da versão nova.
- O aviso só aparece com o app **aberto** (não há serviço em segundo plano).

## 6. Teste antes de prometer (checklist)

Fazer **em cada sistema**, com uma máquina limpa se possível:

- [ ] Instalar a versão `0.1.0` do instalador baixado da Release.
- [ ] Publicar uma `0.1.1` e abrir o app instalado: aparece a faixa com a versão nova?
- [ ] "Agora não" some e volta na próxima abertura?
- [ ] "Atualizar agora": baixa, instala, reinicia, mostra a versão nova em Configurações → Geral?
- [ ] Banco, credenciais (chaveiro), pastas e vínculos continuam lá depois de atualizar?
- [ ] Linux `.deb`: mostra "Baixar nova versão" (e **não** tenta instalar sozinho)?
- [ ] macOS sem assinatura: a atualização abre normalmente depois de instalada?

## 7. Pontos a decidir

1. Chave de assinatura **com senha** (mais seguro) ou sem (mais simples)? Ver seção 4.
2. Divulgar só o **AppImage** para Linux (atualiza sozinho) ou também o `.deb`?
3. Comprar certificado de assinatura (Windows) / conta Apple Developer (macOS) para remover os avisos da
   primeira instalação? Não é necessário para o funcionamento das atualizações.
