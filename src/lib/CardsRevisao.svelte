<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Modal from "$lib/Modal.svelte";
  import { tarefas } from "$lib/tasks.svelte";
  import type { CardRef, Prepared } from "$lib/cards-types";

  let { refs, onVoltar }: { refs: CardRef[]; onVoltar: () => void } = $props();

  type Item = { id: number; tipo: string; texto: string };
  type Review = { id: number; itens: Item[] };
  type Resultado = { resultados: Prepared[]; ids: number[]; avisos: string[] };

  let taskId = $state<string | null>(null);
  let erro = $state("");
  let revisoes = $state<Record<number, Item[]>>({});
  let confirmando = $state(false);
  let carregouReview = "";

  const tarefa = $derived(tarefas.lista.find((t) => t.id === taskId));
  // Cópia local do resultado: dispensar a notificação não pode apagar a revisão em andamento.
  let resultado = $state<Resultado | null>(null);
  let falha = $state("");
  const rodando = $derived(!!taskId && !resultado && !falha && (!tarefa || tarefa.estado === "rodando"));

  $effect(() => {
    if (!tarefa || tarefa.estado === "rodando") return;
    if (tarefa.resultado) resultado = tarefa.resultado as Resultado;
    else falha = tarefa.mensagem || "O resumo falhou.";
  });

  const resultados = $derived(resultado?.resultados ?? []);
  const okIds = $derived(resultados.filter((r) => r.ok).map((r) => r.id));
  const ultimaLinha = $derived(tarefa?.passos.at(-1)?.replace(/^\d\d:\d\d:\d\d /, "") ?? "Iniciando…");

  async function iniciar(regerar: boolean) {
    erro = "";
    revisoes = {};
    resultado = null;
    falha = "";
    carregouReview = "";
    try {
      taskId = await invoke<string>("cards_prepare", { refs, regerar });
    } catch (e) {
      erro = String(e);
    }
  }

  onMount(() => iniciar(false));

  // Terminou o resumo: busca os pontos gravados para revisar.
  $effect(() => {
    if (!taskId || !resultado || resultado.ids.length === 0 || carregouReview === taskId) return;
    carregouReview = taskId;
    invoke<Review[]>("cards_review", { ids: resultado.ids })
      .then((rs) => (revisoes = Object.fromEntries(rs.map((r) => [r.id, r.itens]))))
      .catch((e) => (erro = String(e)));
  });

  async function fechar() {
    confirmando = false;
    erro = "";
    try {
      await invoke("cards_close", { ids: okIds });
      onVoltar(); // a lista recarrega quando a tarefa de fechamento termina
    } catch (e) {
      erro = String(e);
    }
  }
</script>

<div class="linha" style="align-items: center; margin-bottom: 12px">
  <button type="button" onclick={onVoltar}>← Voltar para as US</button>
</div>

<h2 class="titulo">Revisar e fechar</h2>
<p class="det" style="margin-top: 0">
  O Kiro resume o que foi feito em cada US (esses pontos alimentam a prestação de contas). Revise e só então feche:
  a US só fecha se o resumo foi gerado.
</p>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}

{#if rodando}
  <div class="item">
    <div class="cab"><strong>Gerando o resumo…</strong><span class="badge info">em andamento</span></div>
    <p class="det">{ultimaLinha}</p>
    <p class="det">Você pode sair desta tela; o andamento aparece na caixa de notificações.</p>
  </div>
{:else if falha}
  <div class="item falha">
    <div class="cab"><strong>Não foi possível gerar o resumo</strong><span class="badge falha">falhou</span></div>
    <p class="det erro">{falha}</p>
    <div class="linha" style="justify-content: flex-end; margin-top: 8px">
      <button type="button" onclick={() => iniciar(false)}>Tentar de novo</button>
    </div>
  </div>
{:else if resultado}
  {#each resultado.avisos as a}<p class="det" style="color: var(--aviso)">{a}</p>{/each}

  <ul class="lista" style="margin-top: 12px">
    {#each resultados as r (r.id)}
      <li class="item" class:ok={r.ok} class:falha={!r.ok}>
        <div class="cab">
          <strong>{r.ref ? `#${r.ref} ` : ""}{r.titulo}</strong>
          <span class="badge {r.ok ? 'ok' : 'falha'}">{r.ok ? "resumida" : "falhou"}</span>
        </div>
        <p class="det">{r.projeto} · {r.detail}</p>
        {#if r.ok && revisoes[r.id]}
          {#if revisoes[r.id].some((i) => i.tipo === "feito")}
            <p class="rotulo">Feito</p>
            <ul class="pontos">
              {#each revisoes[r.id].filter((i) => i.tipo === "feito") as i (i.id)}<li>{i.texto}</li>{/each}
            </ul>
          {/if}
          {#if revisoes[r.id].some((i) => i.tipo === "pendencia")}
            <p class="rotulo">Pendências externas</p>
            <ul class="pontos">
              {#each revisoes[r.id].filter((i) => i.tipo === "pendencia") as i (i.id)}<li>{i.texto}</li>{/each}
            </ul>
          {/if}
        {/if}
      </li>
    {:else}
      <li class="vazio">Nenhuma US para revisar.</li>
    {/each}
  </ul>

  <div class="acoes">
    <button type="button" onclick={() => iniciar(true)}>Regerar resumo</button>
    <button type="button" class="primario" disabled={okIds.length === 0} onclick={() => (confirmando = true)}>
      Fechar {okIds.length} US no Taiga
    </button>
  </div>
{/if}

{#if confirmando}
  <Modal titulo="Fechar US no Taiga" confirmar="Fechar no Taiga" onConfirm={fechar} onCancel={() => (confirmando = false)}>
    <p style="margin-top: 0">
      Isto <strong>altera o Taiga</strong>: {okIds.length} US serão movidas para o status fechado do projeto de
      cada um (por exemplo, “Done”).
    </p>
    <ul>
      {#each resultados.filter((r) => r.ok) as r}<li>{r.ref ? `#${r.ref} ` : ""}{r.titulo}</li>{/each}
    </ul>
    <p class="det">US que já estiverem fechadas são ignoradas. Os pontos do resumo continuam disponíveis na prestação de contas.</p>
  </Modal>
{/if}

<style>
  .rotulo {
    margin: 10px 0 2px;
    font-size: 0.8rem;
    color: var(--suave);
    font-weight: 600;
  }
  .pontos {
    margin: 0;
    padding-left: 20px;
    font-size: 0.9rem;
  }
  .pontos li {
    margin: 3px 0;
  }
</style>
