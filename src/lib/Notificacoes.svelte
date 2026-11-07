<script lang="ts">
  import { onMount } from "svelte";
  import { tarefas, type Tarefa } from "$lib/tasks.svelte";

  type Ramo = {
    branch: string;
    status: string;
    detail: string;
    pushed: boolean | null;
    push_detail: string;
    taiga: string;
    taiga_ok: boolean | null;
  };
  type Repo = { name: string; branches: Ramo[]; error: string; note: string };
  type Reenvio = { arquivo: string; ok: boolean; detail: string };
  type CardRes = { ref: number | null; titulo: string; projeto: string; ok: boolean; detail: string };

  let abertas = $state<Record<string, boolean>>({});
  let minimizada = $state(false);
  let altura = $state(0);

  // Reserva espaço no fim da página para a caixa não ficar em cima dos botões.
  $effect(() => {
    document.body.style.paddingBottom = tarefas.lista.length > 0 ? `${altura + 24}px` : "";
  });

  onMount(() => tarefas.iniciar());

  const rotulo = (t: Tarefa) => (t.estado === "rodando" ? "em andamento" : t.estado === "ok" ? "concluída" : "com problema");
  const classe = (t: Tarefa) => (t.estado === "ok" ? "ok" : t.estado === "erro" ? "falha" : "info");
  const repos = (t: Tarefa) => (t.key.startsWith("commit:") && Array.isArray(t.resultado) ? (t.resultado as Repo[]) : []);
  const reenvios = (t: Tarefa) => (t.key.startsWith("resend:") && Array.isArray(t.resultado) ? (t.resultado as Reenvio[]) : []);
  const cardsRes = (t: Tarefa): CardRes[] => {
    const r = t.resultado as { resultados?: CardRes[] } | null;
    return t.key.startsWith("cards-") && Array.isArray(r?.resultados) ? r.resultados : [];
  };
  const statusClasse = (s: string) => (s === "erro" ? "falha" : s === "commitado" ? "ok" : s === "atualizado" ? "info" : "");
</script>

{#if tarefas.lista.length > 0}
  <aside class="caixa" aria-label="Notificações" bind:clientHeight={altura}>
    <div class="topo">
      <span>Notificações ({tarefas.lista.length})</span>
      <button type="button" class="mini" onclick={() => (minimizada = !minimizada)}>{minimizada ? "Mostrar" : "Minimizar"}</button>
    </div>
    {#each minimizada ? [] : tarefas.lista as t (t.id)}
      <section class="item" class:ok={t.estado === "ok"} class:falha={t.estado === "erro"}>
        <div class="cab">
          <strong>{t.titulo}</strong>
          <span class="badge {classe(t)}">{rotulo(t)}</span>
        </div>
        <p class="det">{t.mensagem}</p>

        {#if abertas[t.id]}
          {#each repos(t) as r}
            <div class="repo">
              <strong>{r.name}</strong>
              {#if r.error}<p class="erro">{r.error}</p>{/if}
              {#if r.note}<p class="det">{r.note}</p>{/if}
              {#each r.branches as b}
                <div class="ramo">
                  <div class="linha" style="flex-wrap: wrap; align-items: center">
                    <code>{b.branch}</code>
                    <span class="badge {statusClasse(b.status)}">{b.status.replace("_", " ")}</span>
                    {#if b.pushed === true}<span class="badge ok">push ok</span>{/if}
                    {#if b.pushed === false}<span class="badge falha">push falhou</span>{/if}
                    {#if b.taiga_ok === true}<span class="badge ok">Taiga ok</span>{/if}
                    {#if b.taiga_ok === false}<span class="badge falha">Taiga falhou</span>{/if}
                  </div>
                  {#if b.detail}<p class="det">{b.detail}</p>{/if}
                  {#if b.pushed === false && b.push_detail}<p class="det erro">{b.push_detail}</p>{/if}
                  {#if b.taiga}<p class="det" class:erro={b.taiga_ok === false}>{b.taiga}</p>{/if}
                </div>
              {/each}
            </div>
          {/each}
          {#each reenvios(t) as r}
            <div class="ramo">
              <code>{r.arquivo}</code> <span class="badge {r.ok ? 'ok' : 'falha'}">{r.ok ? "enviado" : "falhou"}</span>
              <p class="det" class:erro={!r.ok}>{r.detail}</p>
            </div>
          {/each}
          {#each cardsRes(t) as c}
            <div class="ramo">
              <strong>{c.ref ? `#${c.ref} ` : ""}{c.titulo}</strong>
              <span class="badge {c.ok ? 'ok' : 'falha'}">{c.ok ? "ok" : "falhou"}</span>
              <p class="det" class:erro={!c.ok}>{c.projeto ? `${c.projeto} · ` : ""}{c.detail}</p>
            </div>
          {/each}
          {#if t.passos.length > 0}
            <details class="passos">
              <summary>Etapas ({t.passos.length})</summary>
              <ul>
                {#each t.passos as p}<li>{p}</li>{/each}
              </ul>
            </details>
          {/if}
        {/if}

        <div class="linha" style="justify-content: flex-end; margin-top: 8px">
          {#if t.estado !== "rodando"}
            <button type="button" onclick={() => (abertas[t.id] = !abertas[t.id])}>{abertas[t.id] ? "Ocultar" : "Detalhes"}</button>
            <button type="button" onclick={() => tarefas.dispensar(t.id)}>Dispensar</button>
          {:else}
            <span class="det" style="margin: 0">desde {t.iniciado_em}</span>
          {/if}
        </div>
      </section>
    {/each}
  </aside>
{/if}

<style>
  .caixa {
    position: fixed;
    right: 16px;
    bottom: 16px;
    width: min(380px, calc(100vw - 32px));
    max-height: 70vh;
    overflow: auto;
    display: grid;
    gap: 8px;
    z-index: 50;
  }
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--card);
    border: 1px solid var(--borda);
    border-radius: 8px;
    padding: 4px 10px;
    font-size: 0.8rem;
    color: var(--suave);
  }
  .mini {
    padding: 2px 9px;
    font-size: 0.75rem;
  }
  .item {
    background: var(--card);
    border: 1px solid var(--borda);
    border-left: 4px solid var(--acento);
    border-radius: 8px;
    padding: 10px 12px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
    font-size: 0.88rem;
  }
  .item.ok {
    border-left-color: var(--ok);
  }
  .item.falha {
    border-left-color: var(--falha);
  }
  .det {
    margin: 4px 0 0;
  }
  .repo {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--borda);
  }
  .ramo {
    margin-top: 6px;
    padding-left: 8px;
  }
  .passos {
    margin-top: 8px;
    color: var(--suave);
  }
  .passos ul {
    margin: 4px 0 0;
    padding-left: 18px;
    max-height: 140px;
    overflow: auto;
  }
</style>
