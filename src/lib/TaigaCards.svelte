<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import CardsLista from "$lib/CardsLista.svelte";
  import Prestacao from "$lib/Prestacao.svelte";
  import { tarefas } from "$lib/tasks.svelte";

  let sub = $state<"cards" | "prestacao">("cards");
  let pendentes = $state(0);

  async function atualizarPendentes() {
    try {
      pendentes = await invoke<number>("prestacao_pendentes");
    } catch {
      /* o contador é só informativo */
    }
  }

  onMount(atualizarPendentes);

  $effect(() => {
    void tarefas.versaoTerminadas; // um resumo novo pode ter criado pontos pendentes
    atualizarPendentes();
  });
</script>

<nav class="abas" aria-label="Taiga US's">
  <button type="button" class:ativa={sub === "cards"} onclick={() => (sub = "cards")}>User Stories (US)</button>
  <button type="button" class:ativa={sub === "prestacao"} onclick={() => (sub = "prestacao")}>
    Prestar conta em reunião{#if pendentes > 0}&nbsp;<span class="badge aviso">{pendentes}</span>{/if}
  </button>
</nav>

{#if sub === "cards"}
  <CardsLista />
{:else}
  <Prestacao onMudou={atualizarPendentes} />
{/if}
