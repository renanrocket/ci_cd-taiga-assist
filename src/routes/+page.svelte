<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Configuracoes from "$lib/Configuracoes.svelte";
  import Credencial from "$lib/Credencial.svelte";
  import Kiro from "$lib/Kiro.svelte";
  import Pasta from "$lib/Pasta.svelte";
  import PastaLocal from "$lib/PastaLocal.svelte";
  import Prereq from "$lib/Prereq.svelte";
  import TaigaCards from "$lib/TaigaCards.svelte";
  import Taiga from "$lib/Taiga.svelte";
  import { capacidades } from "$lib/capacidades.svelte";
  import { nav } from "$lib/nav.svelte";

  const PASSOS = [
    "Pré-requisitos",
    "Pasta local",
    "Credencial do repositório",
    "Kiro (opcional)",
    "Taiga (opcional, precisa do Kiro)",
  ];

  let pronto = $state(false); // já leu do banco se o assistente foi concluído
  let concluido = $state(false);
  let passo = $state(0);

  const proximo = () => (passo = Math.min(passo + 1, PASSOS.length - 1));
  const anterior = () => (passo = Math.max(passo - 1, 0));

  async function concluir() {
    await invoke("set_wizard_done", { done: true });
    concluido = true;
    capacidades.atualizar();
  }

  onMount(async () => {
    try {
      concluido = await invoke<boolean>("wizard_done");
    } finally {
      pronto = true;
    }
  });
</script>

<main class:largo={concluido}>
  <h1>Taiga Assist</h1>

  {#if !pronto}
    <p class="sub">Carregando…</p>
  {:else if concluido}
    <nav class="abas" aria-label="Seções">
      <button type="button" class:ativa={nav.aba === "pasta"} onclick={() => nav.ir("pasta")}>Pasta local</button>
      <button type="button" class:ativa={nav.aba === "cards"} onclick={() => nav.ir("cards")}>Taiga US's</button>
      <button type="button" class:ativa={nav.aba === "config"} onclick={() => nav.ir("config")}>Configurações</button>
    </nav>
    {#if nav.aba === "pasta"}
      <PastaLocal />
    {:else if nav.aba === "cards"}
      <TaigaCards />
    {:else}
      <Configuracoes />
    {/if}
  {:else}
    <p class="sub">Passo {passo + 1} de {PASSOS.length} — {PASSOS[passo]}</p>

    {#if passo === 0}
      <Prereq onNext={proximo} />
    {:else if passo === 1}
      <Pasta onNext={proximo} onBack={anterior} />
    {:else if passo === 2}
      <Credencial onNext={proximo} onBack={anterior} />
    {:else if passo === 3}
      <Kiro onNext={proximo} onBack={anterior} />
    {:else}
      <Taiga onNext={concluir} onBack={anterior} rotuloProximo="Concluir" rotuloPular="Concluir sem Taiga" irKiro={() => (passo = 3)} />
    {/if}
  {/if}
</main>
