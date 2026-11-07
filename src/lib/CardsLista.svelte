<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import CardsRevisao from "$lib/CardsRevisao.svelte";
  import Modal from "$lib/Modal.svelte";
  import { tarefas } from "$lib/tasks.svelte";
  import { fmtData, type Card, type CardRef, type CardsPage } from "$lib/cards-types";

  let projeto = $state(""); // id do projeto ("" = todos)
  let origem = $state("todos");
  let ocultarFechados = $state(false);
  let pagina = $state(1);
  let dados = $state<CardsPage | null>(null);
  let carregando = $state(true);
  let atualizando = $state(false);
  let erro = $state("");
  type Marcada = { ref: CardRef; resumido: boolean; rotulo: string };
  let sel = $state<Record<string, Marcada>>({});
  let confirmandoFechar = $state(false);
  let aviso = $state("");
  let detalhe = $state<Card | null>(null);
  let revisao = $state<CardRef[] | null>(null);

  const chaveCache = () => `cards:${projeto}:${origem}:${ocultarFechados}:${pagina}`;

  function lerCache(): CardsPage | null {
    try {
      const t = localStorage.getItem(chaveCache());
      return t ? (JSON.parse(t) as CardsPage) : null;
    } catch {
      return null;
    }
  }

  function gravarCache(d: CardsPage) {
    try {
      localStorage.setItem(chaveCache(), JSON.stringify(d));
    } catch {
      /* cache é só conveniência */
    }
  }

  // Abre instantâneo com o que já foi visto e atualiza por baixo.
  async function carregar(refresh = false) {
    erro = "";
    const antigo = refresh ? null : lerCache();
    if (antigo) {
      dados = antigo;
      carregando = false;
      atualizando = true;
    } else {
      carregando = true;
    }
    try {
      const d = await invoke<CardsPage>("cards_list", {
        page: pagina,
        projectId: projeto ? Number(projeto) : null,
        origem,
        ocultarFechados,
        refresh,
      });
      dados = d;
      pagina = d.page;
      gravarCache(d);
    } catch (e) {
      erro = String(e);
    } finally {
      carregando = false;
      atualizando = false;
    }
  }

  onMount(() => carregar());

  // Quando uma tarefa termina (resumo, fechamento), a lista se atualiza.
  let vistas = "";
  $effect(() => {
    const v = tarefas.versaoTerminadas;
    if (v !== vistas) {
      const primeira = vistas === "";
      vistas = v;
      if (!primeira && !revisao) carregar(true);
    }
  });

  function mudouFiltro() {
    pagina = 1;
    carregar();
  }

  const cards = $derived(dados?.cards ?? []);
  const selecionaveis = $derived(cards.filter((c) => c.selecionavel));
  const todos = $derived(selecionaveis.length > 0 && selecionaveis.every((c) => sel[c.chave]));
  const qtd = $derived(Object.keys(sel).length);

  function marcar(c: Card, v: boolean) {
    if (v)
      sel[c.chave] = {
        ref: { report_id: c.report_id, project_id: c.project_id, us_id: c.us_id },
        resumido: c.resumo_pronto && c.report_id !== null,
        rotulo: `${c.ref ? `#${c.ref} ` : ""}${c.titulo}`,
      };
    else delete sel[c.chave];
  }

  function marcarTodos(v: boolean) {
    for (const c of selecionaveis) marcar(c, v);
  }

  function prazo(c: Card): { texto: string; classe: string } | null {
    switch (c.prazo_estado) {
      case "atrasado":
        return { texto: `atrasado ${Math.abs(c.dias ?? 0)}d`, classe: "falha" };
      case "hoje":
        return { texto: "vence hoje", classe: "aviso" };
      case "proximo":
        return { texto: `em ${c.dias} dia(s)`, classe: "aviso" };
      case "ok":
        return { texto: `prazo ${fmtData(c.prazo)}`, classe: "" };
      default:
        return null;
    }
  }

  function revisar() {
    revisao = Object.values(sel).map((s) => s.ref);
  }

  // Todas as selecionadas já têm resumo: dá para fechar direto, sem gerar o resumo de novo.
  const todosResumidos = $derived(qtd > 0 && Object.values(sel).every((s) => s.resumido));

  async function fecharDireto() {
    confirmandoFechar = false;
    erro = aviso = "";
    try {
      await invoke("cards_close", { ids: Object.values(sel).map((s) => s.ref.report_id) });
      aviso = "Fechamento iniciado em segundo plano; a lista atualiza quando terminar.";
      sel = {};
    } catch (e) {
      erro = String(e);
    }
  }
</script>

{#if revisao}
  <CardsRevisao
    refs={revisao}
    onVoltar={() => {
      revisao = null;
      sel = {};
      carregar(true);
    }}
  />
{:else}
  {#if erro}<p class="erro" role="alert">{erro}</p>{/if}
  {#if aviso}<p class="det" style="color: var(--ok)">{aviso}</p>{/if}

  <div class="filtros">
    <select bind:value={projeto} onchange={mudouFiltro} aria-label="Projeto">
      <option value="">Todos os projetos</option>
      {#each dados?.projetos ?? [] as p (p.id)}<option value={String(p.id)}>{p.nome}</option>{/each}
    </select>
    <select bind:value={origem} onchange={mudouFiltro} aria-label="Origem">
      <option value="todos">Todas as US</option>
      <option value="ferramenta">Criadas pelo app</option>
      <option value="externo">Criadas no Taiga</option>
    </select>
    <label class="marca" title="Externos já fechados aparecem no fim da lista (costumam ser centenas). Marque para escondê-los.">
      <input type="checkbox" bind:checked={ocultarFechados} onchange={mudouFiltro} />
      Ocultar fechados
    </label>
    <button type="button" onclick={() => carregar(true)} disabled={carregando || atualizando}>
      {carregando || atualizando ? "Atualizando…" : "Atualizar"}
    </button>
  </div>

  {#if carregando}
    <ul class="lista" aria-busy="true">
      {#each [1, 2, 3, 4] as n (n)}<li class="item esqueleto"><div></div><div></div></li>{/each}
    </ul>
  {:else}
    <div class="barra">
      <label class="marca">
        <input type="checkbox" checked={todos} disabled={selecionaveis.length === 0} onchange={(e) => marcarTodos(e.currentTarget.checked)} />
        Selecionar todos (desta página)
      </label>
      <span class="det" style="margin: 0">{dados?.total ?? 0} US</span>
      <span class="linha" style="flex: none; gap: 8px">
        {#if todosResumidos}
          <button type="button" onclick={revisar} title="Rever os pontos do resumo antes de fechar">Revisar e fechar ({qtd})</button>
          <button type="button" class="primario" onclick={() => (confirmandoFechar = true)}>Fechar ({qtd})</button>
        {:else}
          <button type="button" class="primario" disabled={qtd === 0} onclick={revisar}>Revisar e fechar ({qtd})</button>
        {/if}
      </span>
    </div>

    <ul class="lista">
      {#each cards as c (c.chave)}
        {@const pz = prazo(c)}
        <li class="item" class:ok={c.estado === "ok" && !c.fechado} class:falha={c.estado !== "ok"} class:aviso={c.fechado}>
          <div class="linha-card">
            <input
              type="checkbox"
              checked={!!sel[c.chave]}
              disabled={!c.selecionavel}
              onchange={(e) => marcar(c, e.currentTarget.checked)}
              title={c.selecionavel ? "Selecionar para fechar" : c.fechado ? "Já está fechada" : "Indisponível"}
              aria-label={`Selecionar ${c.titulo}`}
            />
            <button type="button" class="corpo" onclick={() => (detalhe = c)}>
              <span class="cab">
                <strong>{c.ref ? `#${c.ref} ` : ""}{c.titulo}</strong>
              </span>
              <span class="badges">
                <span class="badge">{c.projeto}</span>
                {#if c.status}<span class="badge info">{c.status}</span>{/if}
                {#if c.fechado}<span class="badge ok">fechada</span>{/if}
                {#if pz}<span class="badge {pz.classe}">{pz.texto}</span>{/if}
                {#if c.pontos_total}<span class="badge">{c.pontos_total} pts</span>{/if}
                {#if c.externo}<span class="badge" title="Criada direto no Taiga">externa</span>{/if}
                {#if c.resumo_pronto}<span class="badge ok" title="Resumo já gerado">resumida</span>{/if}
                {#if c.estado === "erro"}<span class="badge falha">indisponível</span>{/if}
                {#if c.estado === "removido"}<span class="badge falha">removida</span>{/if}
              </span>
              {#if c.erro}<span class="det erro">{c.erro}</span>{:else if c.resumo}<span class="det">{c.resumo}</span>{/if}
            </button>
          </div>
        </li>
      {:else}
        <li class="vazio">Nenhuma US encontrada. As US criadas pelo app e as dos projetos importados aparecem aqui.</li>
      {/each}
    </ul>

    {#if (dados?.pages ?? 1) > 1}
      <div class="acoes">
        <button type="button" disabled={pagina <= 1} onclick={() => { pagina--; carregar(); }}>← Anterior</button>
        <span class="det" style="margin: 0">Página {pagina} de {dados?.pages}</span>
        <button type="button" disabled={pagina >= (dados?.pages ?? 1)} onclick={() => { pagina++; carregar(); }}>Próxima →</button>
      </div>
    {/if}
  {/if}
{/if}

{#if confirmandoFechar}
  <Modal titulo="Fechar US no Taiga" confirmar="Fechar no Taiga" onConfirm={fecharDireto} onCancel={() => (confirmandoFechar = false)}>
    <p style="margin-top: 0">
      Isto <strong>altera o Taiga</strong>: {qtd} US serão movidas para o status fechado do projeto de cada uma (por
      exemplo, “Done”). Os resumos que já existem são mantidos.
    </p>
    <ul>
      {#each Object.values(sel) as s}<li>{s.rotulo}</li>{/each}
    </ul>
    <p class="det">US que já estiverem fechadas são ignoradas. Os pontos continuam disponíveis na prestação de contas.</p>
  </Modal>
{/if}

{#if detalhe}
  <Modal titulo={`${detalhe.ref ? `#${detalhe.ref} ` : ""}${detalhe.titulo}`} confirmar={detalhe.url ? "Abrir no Taiga" : null} cancelar="Fechar" onConfirm={() => detalhe?.url && openUrl(detalhe.url)} onCancel={() => (detalhe = null)}>
    <p class="badges" style="margin-top: 0">
      <span class="badge">{detalhe.projeto}</span>
      {#if detalhe.status}<span class="badge info">{detalhe.status}</span>{/if}
      {#if detalhe.prazo}<span class="badge">prazo {fmtData(detalhe.prazo)}</span>{/if}
      {#if detalhe.criado_taiga}<span class="badge">criada em {fmtData(detalhe.criado_taiga)}</span>{/if}
      {#if detalhe.criado_por}<span class="badge">por {detalhe.criado_por}</span>{/if}
    </p>
    {#if detalhe.pontos_papeis.length > 0}
      <p class="det">Pontos: {detalhe.pontos_papeis.map((p) => `${p.papel} ${p.pontos}`).join(" · ")}{detalhe.pontos_total ? ` (total ${detalhe.pontos_total})` : ""}</p>
    {/if}
    {#if detalhe.erro}<p class="erro">{detalhe.erro}</p>{/if}
    <pre class="desc">{detalhe.descricao || (detalhe.descricao_carregada ? "(sem descrição)" : "Descrição não carregada.")}</pre>
  </Modal>
{/if}

<style>
  .filtros {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    margin-bottom: 14px;
  }
  .filtros select {
    flex: 0 1 220px;
    width: auto;
  }
  .barra {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .linha-card {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .linha-card input[type="checkbox"] {
    flex: none;
    margin-top: 4px;
  }
  .corpo {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 6px;
    text-align: left;
    border: none;
    background: none;
    padding: 0;
    color: inherit;
    font: inherit;
  }
  .badges {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .desc {
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--bg);
    border: 1px solid var(--borda);
    border-radius: 6px;
    padding: 10px 12px;
    font-family: inherit;
    font-size: 0.88rem;
    max-height: 50vh;
    overflow: auto;
  }
  .esqueleto div {
    height: 12px;
    border-radius: 6px;
    background: var(--borda);
    margin: 8px 0;
    animation: pulso 1.2s ease-in-out infinite;
  }
  .esqueleto div:first-child {
    width: 55%;
  }
  .esqueleto div:last-child {
    width: 85%;
  }
  @keyframes pulso {
    50% {
      opacity: 0.4;
    }
  }
</style>
