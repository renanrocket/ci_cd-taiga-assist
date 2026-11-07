<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import CardsLista from "$lib/CardsLista.svelte";
  import Prestacao from "$lib/Prestacao.svelte";
  import Requisito from "$lib/Requisito.svelte";
  import { capacidades } from "$lib/capacidades.svelte";
  import { nav } from "$lib/nav.svelte";
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

  onMount(() => {
    capacidades.atualizar();
    atualizarPendentes();
  });

  $effect(() => {
    void tarefas.versaoTerminadas; // um resumo novo pode ter criado pontos pendentes
    atualizarPendentes();
  });
</script>

{#if capacidades.carregado && capacidades.contasTaiga === 0}
  <!-- Sem conta do Taiga não há o que listar nem prestar: o recurso fica desabilitado, com o motivo e o caminho. -->
  <Requisito
    texto={capacidades.kiroPronto
      ? "Cadastre sua conta do Taiga para listar as US, gerar o resumo e prestar contas. Sem o Taiga, o app faz só commit e push."
      : "O Taiga precisa do Kiro (os resumos das US são gerados por ele). Configure o Kiro e depois sua conta do Taiga. Sem isso, o app faz só commit e push."}
    acao={capacidades.kiroPronto ? "Configurar o Taiga" : "Configurar o Kiro"}
    onAcao={() => nav.ir("config", capacidades.kiroPronto ? "taiga" : "kiro")}
  />
{:else}
{#if capacidades.carregado && !capacidades.kiroPronto}
  <Requisito
    texto="O Kiro não está pronto (instalação ou login). Sem ele não dá para gerar o resumo das US, e a prestação de contas depende do resumo."
    acao="Configurar o Kiro"
    onAcao={() => nav.ir("config", "kiro")}
  />
{/if}
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
{/if}
