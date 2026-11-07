# Convenções de UI — ci_cd-taiga-assist

Regra geral: controles com a mesma função têm o mesmo estilo, rótulo, posição e comportamento em
todas as telas. Antes de criar um controle, procure o equivalente nas telas irmãs e reutilize.
Estilos compartilhados em `src/lib/app.css` (botão, botão primário, botão perigoso, campo, `select`,
item de lista, mensagem de erro). Tema claro/escuro por `prefers-color-scheme`.

## Padrões registrados

- **Formulário de cadastro acima, lista dos itens cadastrados abaixo.** Quem adiciona algo precisa ver o
  resultado logo abaixo do formulário, sem rolar a tela. Vale para contas Taiga, credenciais git e
  qualquer lista com formulário de "adicionar".
- **Seleção em lote:** caixa **"Selecionar todos"** no topo da lista, seguida das caixas individuais
  (ex.: importar projetos do Taiga). O contador do botão de ação mostra a quantidade marcada
  ("Importar selecionados (N)").
- **Ações do item** ficam no canto inferior direito do cartão: ação secundária (ex.: *Importar
  projetos…*) antes, **Remover** (botão perigoso) por último.
- **Teste de conexão:** botão **Testar** ao lado do campo que o configura; o resultado aparece no
  próprio cartão (borda verde = OK, vermelha = falhou) com a mensagem do erro abaixo.
- **Navegação do assistente:** **Voltar** à esquerda, **Continuar** (primário) à direita; "Continuar"
  desabilitado até o passo ter o mínimo necessário. Em passo **opcional** (Kiro, Taiga) o mesmo botão, no mesmo lugar,
  vira **Pular** (secundário, sem destaque) enquanto o passo não está configurado, e volta a **Continuar** (primário)
  quando está. No último passo, "Concluir" / "Concluir sem Taiga".
- **Selects:** opções estilizadas com as cores do tema (`select option` em `app.css`) e
  `color-scheme: light dark` na raiz, para a lista nativa não abrir branca no tema escuro.
- **Confirmação** de ações que alteram dados externos (Taiga) em popup do app, nunca `alert`/`confirm`.
- **Configurações:** menu lateral com as seções (Geral, Taiga, Acesso aos repositórios, Pastas locais,
  Kiro, Pré-requisitos); clicar abre só o formulário daquela seção, sem passar pelo assistente. O
  assistente de 5 passos existe apenas no primeiro uso e reaproveita os mesmos componentes
  (`embutido` esconde Voltar/Continuar). Cada seção grava na hora, pelo botão do próprio formulário
  (Adicionar, Salvar, Remover); não há um "Salvar tudo" global.
- **Campos de formulário** (`input`, `select`, `textarea`) ocupam a largura toda do container; rótulos de
  caixa de seleção (`.marca`) não herdam o estilo de rótulo de campo.
- **Selos (`.badge`)** com as variantes `ok`, `falha`, `aviso`, `info` para estado de itens e branches.
- **Tarefas em segundo plano** aparecem na caixa de notificações do canto (etapas, detalhes, Dispensar);
  telas que dependem do resultado recarregam quando a tarefa termina.
- **Ícone do app:** `assets/icon.svg` (fonte) → `npx tauri icon assets/icon.svg` gera `src-tauri/icons/`
  (apagar as pastas `android/` e `ios/` geradas, o app é só desktop).
- **Recurso que depende de outro (`Requisito.svelte`):** o controle fica **desabilitado, nunca escondido**, e um aviso
  amarelo diz o que falta, com um botão que leva direto à configuração ("Configurar o Kiro", "Configurar o Taiga" →
  Configurações na seção certa, via `nav.svelte.ts`). Quem decide é `capacidades.svelte.ts`: **base** = Git + pasta
  (commit e push); **Kiro pronto** = binário achado e logado/API key; **Taiga pronto** = conta cadastrada **e** Kiro
  pronto (o Taiga depende do Kiro). Vale para a caixa "Criar US" (Pasta local), a aba Taiga US's e o cadastro do Taiga.
- **Criar US no Taiga (Pasta local):** um único checkbox **global**, logo abaixo de "Fazer push depois do
  commit", marcado por padrão, com tooltip. Desmarcado, nenhum relatório nem US é criado, mesmo com Taiga e
  Kiro configurados. Depende do push (desabilitado sem push) e de Kiro + Taiga prontos (senão fica desabilitado, com o
  aviso `Requisito`). Mesmo controle, rótulo e posição no `ci_cd`.
- **Taiga US's:** aba própria entre "Pasta local" e "Configurações", com duas sub-abas iguais às do `ci_cd`:
  **User Stories (US)** e **Prestar conta em reunião** (com selo de pendentes). Filtros no topo da lista (projeto, origem,
  "Ocultar fechados", Atualizar), "Selecionar todos (desta página)" + botão primário **Revisar e fechar (N)**,
  paginação "← Anterior / Próxima →". A lista abre instantânea com o último resultado visto (`localStorage`)
  e atualiza por baixo. Mostra sempre o **nome** do projeto, nunca o slug.
- **Ações que alteram o Taiga** (fechar US, exportar para a US de destino) pedem confirmação em popup que diz
  exatamente o que será alterado. Fechar só vale para quem tem resumo gerado; exportar só marca como
  prestado se a edição da US deu certo.
- **Fechar direto:** se todas as US selecionadas já têm resumo, aparece o botão primário **Fechar (N)** (com
  popup de confirmação) e "Revisar e fechar" vira secundário; se alguma não tem, só "Revisar e fechar".
- **Caixa de notificações:** reserva espaço no fim da página (não cobre botões) e pode ser minimizada. Telas
  que dependem do resultado de uma tarefa guardam a própria cópia: dispensar a notificação não as afeta.
- **Detalhe de uma US:** clicar na linha abre um popup só informativo (botão "Fechar" + "Abrir no Taiga").
