<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Modal from "$lib/Modal.svelte";
  import { tarefas } from "$lib/tasks.svelte";
  import { fmtData, type ProjetoOpt } from "$lib/cards-types";

  let { onMudou }: { onMudou?: () => void } = $props();

  type Item = { id: number; tipo: string; texto: string; prestado_em: string | null };
  type Grupo = {
    us_id: number;
    ref: number | null;
    titulo: string;
    project_id: number | null;
    projeto: string;
    data: string | null;
    report_id: number | null;
    pode_regerar: boolean;
    itens: Item[];
  };
  type Saida = { grupos: Grupo[]; total: number; projetos: ProjetoOpt[]; pendentes: number };

  let q = $state("");
  let projeto = $state("");
  let estado = $state("pendente");
  let dados = $state<Saida | null>(null);
  let sel = $state<Record<number, boolean>>({});
  let carregando = $state(true);
  let ocupado = $state(false);
  let erro = $state("");
  let ok = $state("");
  let exportando = $state(false);
  let url = $state("");
  let erroUrl = $state("");
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function carregar() {
    try {
      dados = await invoke<Saida>("prestacao_list", { q, projectId: projeto ? Number(projeto) : null, estado });
      // mantém só as marcações que ainda estão na tela
      const visiveis = new Set(dados.grupos.flatMap((g) => g.itens.map((i) => i.id)));
      for (const k of Object.keys(sel)) if (!visiveis.has(Number(k))) delete sel[Number(k)];
    } catch (e) {
      erro = String(e);
    } finally {
      carregando = false;
    }
  }

  onMount(carregar);

  function buscar() {
    clearTimeout(timer);
    timer = setTimeout(carregar, 300);
  }

  let vistas = "";
  $effect(() => {
    const v = tarefas.versaoTerminadas;
    if (v !== vistas) {
      const primeira = vistas === "";
      vistas = v;
      if (!primeira) carregar();
    }
  });

  const itens = $derived(dados?.grupos.flatMap((g) => g.itens) ?? []);
  const marcados = $derived(itens.filter((i) => sel[i.id]));
  const qtd = $derived(marcados.length);
  const qtdPrestados = $derived(marcados.filter((i) => i.prestado_em).length);
  const qtdPendentes = $derived(qtd - qtdPrestados);
  const todos = $derived(itens.length > 0 && itens.every((i) => sel[i.id]));

  function marcarTodos(v: boolean) {
    for (const i of itens) sel[i.id] = v;
  }
  const grupoMarcado = (g: Grupo) => g.itens.every((i) => sel[i.id]);
  function marcarGrupo(g: Grupo, v: boolean) {
    for (const i of g.itens) sel[i.id] = v;
  }

  async function executar(fn: () => Promise<string>) {
    ocupado = true;
    erro = ok = "";
    try {
      ok = await fn();
      sel = {};
      await carregar();
      onMudou?.();
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = false;
    }
  }

  const ids = () => marcados.map((i) => i.id);
  const prestar = () => executar(() => invoke<string>("prestar", { itemIds: ids(), exportarUrl: null }));
  const desfazer = () => executar(() => invoke<string>("desfazer", { itemIds: ids() }));

  async function exportar() {
    erroUrl = "";
    if (!url.trim()) {
      erroUrl = "Cole o link da US de destino.";
      return;
    }
    const destino = url.trim();
    exportando = false;
    await executar(() => invoke<string>("prestar", { itemIds: ids(), exportarUrl: destino }));
  }

  async function alternar(i: Item) {
    erro = ok = "";
    try {
      await invoke("toggle_item", { id: i.id, prestado: !i.prestado_em });
      await carregar();
      onMudou?.();
    } catch (e) {
      erro = String(e);
    }
  }

  async function regerar(g: Grupo) {
    erro = ok = "";
    try {
      await invoke("cards_prepare", { refs: [{ report_id: g.report_id, project_id: g.project_id, us_id: g.us_id }], regerar: true });
      ok = "Resumo sendo regerado em segundo plano; acompanhe pela caixa de notificações.";
    } catch (e) {
      erro = String(e);
    }
  }
</script>

<p class="det" style="margin-top: 0">
  Pontos do que foi feito em cada US, prontos para relatar na reunião. Marque os pontos, clique em
  <strong>Prestar conta</strong> e, se quiser, exporte direto para a US da reunião (o link é informado na hora).
</p>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}
{#if ok}<p class="det" style="color: var(--ok)">{ok}</p>{/if}

<div class="filtros">
  <input type="search" placeholder="Buscar nos pontos e títulos" bind:value={q} oninput={buscar} aria-label="Buscar" />
  <select bind:value={projeto} onchange={carregar} aria-label="Projeto">
    <option value="">Todos os projetos</option>
    {#each dados?.projetos ?? [] as p (p.id)}<option value={String(p.id)}>{p.nome}</option>{/each}
  </select>
  <select bind:value={estado} onchange={carregar} aria-label="Estado">
    <option value="pendente">Pendentes</option>
    <option value="prestado">Prestados</option>
    <option value="todos">Todos</option>
  </select>
</div>

<div class="barra">
  <label class="marca">
    <input type="checkbox" checked={todos} disabled={itens.length === 0} onchange={(e) => marcarTodos(e.currentTarget.checked)} />
    Selecionar todos
  </label>
  <span class="det" style="margin: 0">{dados?.total ?? 0} ponto(s) · {dados?.pendentes ?? 0} pendente(s) no total</span>
</div>

{#if carregando}
  <p class="vazio">Carregando…</p>
{:else}
  <ul class="lista">
    {#each dados?.grupos ?? [] as g (g.us_id)}
      <li class="item">
        <div class="cab">
          <label class="marca">
            <input type="checkbox" checked={grupoMarcado(g)} onchange={(e) => marcarGrupo(g, e.currentTarget.checked)} aria-label={`Selecionar os pontos de ${g.titulo}`} />
            <strong>{g.ref ? `#${g.ref} ` : ""}{g.titulo}</strong>
          </label>
          <span class="linha" style="flex: none; gap: 6px">
            {#if g.projeto}<span class="badge">{g.projeto}</span>{/if}
            {#if g.data}<span class="badge">{fmtData(g.data)}</span>{/if}
          </span>
        </div>
        <ul class="pontos">
          {#each g.itens as i (i.id)}
            <li>
              <label class="marca" style="align-items: flex-start">
                <input type="checkbox" bind:checked={sel[i.id]} style="margin-top: 4px" />
                <span class:feito={!!i.prestado_em}>
                  {#if i.tipo === "pendencia"}<span class="badge aviso">pendência</span>{/if}
                  {i.texto}
                  {#if i.prestado_em}<span class="badge ok" title={i.prestado_em}>prestado</span>{/if}
                </span>
              </label>
              <button type="button" class="mini" onclick={() => alternar(i)} title={i.prestado_em ? "Voltar a pendente" : "Marcar como prestado (sem exportar)"}>
                {i.prestado_em ? "Desfazer" : "Marcar prestado"}
              </button>
            </li>
          {/each}
        </ul>
        {#if g.pode_regerar}
          <div class="linha" style="justify-content: flex-end; margin-top: 8px">
            <button type="button" class="mini" onclick={() => regerar(g)} title="Gera de novo o resumo desta US (os pontos já prestados são mantidos)">Regerar resumo</button>
          </div>
        {/if}
      </li>
    {:else}
      <li class="vazio">
        {estado === "pendente" ? "Nenhum ponto pendente. Os pontos aparecem aqui quando uma US é resumida em “Revisar e fechar”." : "Nenhum ponto encontrado."}
      </li>
    {/each}
  </ul>

  <div class="acoes" style="flex-wrap: wrap; gap: 8px">
    <button type="button" class="perigo" disabled={ocupado || qtdPrestados === 0} onclick={desfazer} title="Volta a pendente os pontos prestados marcados (não altera o Taiga)">
      Desfazer prestação ({qtdPrestados})
    </button>
    <span class="linha" style="flex: none; gap: 8px">
      <button type="button" disabled={ocupado || qtdPendentes === 0} onclick={() => { url = ""; erroUrl = ""; exportando = true; }}>
        Prestar conta e exportar… ({qtdPendentes})
      </button>
      <button type="button" class="primario" disabled={ocupado || qtdPendentes === 0} onclick={prestar}>Prestar conta ({qtdPendentes})</button>
    </span>
  </div>
{/if}

{#if exportando}
  <Modal titulo="Prestar conta e exportar" confirmar="Exportar e prestar conta" onConfirm={exportar} onCancel={() => (exportando = false)}>
    <p style="margin-top: 0">
      Isto <strong>edita a descrição de uma US do Taiga</strong>: os {qtdPendentes} ponto(s) selecionado(s) entram sob o
      título “Feito” (e as pendências sob “Pendências”), agrupados por US de origem. Nada é movido de status.
    </p>
    <div class="campo">
      <label for="url-destino">Link da US de destino</label>
      <input id="url-destino" bind:value={url} placeholder="https://taiga.suaempresa.com.br/project/reunioes/us/350" />
    </div>
    {#if erroUrl}<p class="erro">{erroUrl}</p>{/if}
    <p class="det">Se a exportação falhar, nada é marcado como prestado.</p>
  </Modal>
{/if}

<style>
  .filtros {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .filtros input {
    flex: 1 1 240px;
  }
  .filtros select {
    flex: 0 1 200px;
    width: auto;
  }
  .barra {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }
  .pontos {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .pontos li {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 10px;
  }
  .feito {
    color: var(--suave);
  }
  .mini {
    flex: none;
    padding: 3px 9px;
    font-size: 0.78rem;
  }
</style>
