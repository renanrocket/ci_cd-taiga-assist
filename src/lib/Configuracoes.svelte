<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import Credencial from "$lib/Credencial.svelte";
  import Kiro from "$lib/Kiro.svelte";
  import Pasta from "$lib/Pasta.svelte";
  import Prereq from "$lib/Prereq.svelte";
  import Taiga from "$lib/Taiga.svelte";
  import { atualizacao } from "$lib/atualizacao.svelte";

  const SECOES = [
    { id: "geral", nome: "Geral" },
    { id: "taiga", nome: "Taiga" },
    { id: "git", nome: "Acesso aos repositórios" },
    { id: "pastas", nome: "Pastas locais" },
    { id: "kiro", nome: "Kiro" },
    { id: "requisitos", nome: "Pré-requisitos" },
  ] as const;

  let secao = $state<(typeof SECOES)[number]["id"]>("geral");
  const atual = $derived(SECOES.find((s) => s.id === secao)!);

  // Geral
  let prazo = $state("15");
  let versao = $state("");
  let salvo = $state(false);
  let erro = $state("");

  onMount(async () => {
    try {
      prazo = String(await invoke<number>("get_prazo_dias"));
      versao = await getVersion();
    } catch (e) {
      erro = String(e);
    }
  });

  async function abrirLogs() {
    erro = "";
    try {
      await revealItemInDir(await invoke<string>("logs_dir"));
    } catch (e) {
      erro = String(e);
    }
  }

  async function salvarPrazo() {
    erro = "";
    salvo = false;
    try {
      const dias = Number(prazo);
      if (!Number.isInteger(dias)) throw new Error("Informe um número inteiro de dias.");
      prazo = String(await invoke<number>("set_prazo_dias", { dias }));
      salvo = true;
    } catch (e) {
      erro = String(e instanceof Error ? e.message : e);
    }
  }
</script>

<div class="config">
  <nav class="menu" aria-label="Seções de configuração">
    {#each SECOES as s (s.id)}
      <button type="button" class:ativa={secao === s.id} onclick={() => (secao = s.id)}>{s.nome}</button>
    {/each}
  </nav>

  <section>
    <h2 class="titulo">{atual.nome}</h2>

    {#key secao}
      {#if secao === "geral"}
        <p class="det" style="margin-top: 0">Valores padrão usados nas histórias (US) criadas a partir dos relatórios.</p>
        {#if erro}<p class="erro" role="alert">{erro}</p>{/if}
        <form
          class="campo"
          onsubmit={(e) => {
            e.preventDefault();
            salvarPrazo();
          }}
        >
          <label for="prazo">Prazo da US (dias corridos a partir de hoje; sábado e domingo passam para segunda)</label>
          <div class="linha">
            <input id="prazo" type="number" min="0" max="365" bind:value={prazo} oninput={() => (salvo = false)} />
            <button type="submit" class="primario">Salvar</button>
          </div>
          {#if salvo}<p class="det" style="color: var(--ok)">Salvo.</p>{/if}
        </form>

        <h3 class="sub-titulo">Atualizações</h3>
        <p class="det" style="margin-top: 0">
          O app avisa quando existe uma versão nova; você escolhe se atualiza agora. Seus dados e configurações são mantidos.
        </p>
        <div class="linha" style="align-items: center">
          <button type="button" onclick={() => atualizacao.verificar(true)} disabled={atualizacao.estado !== "ocioso"}>
            {atualizacao.estado === "verificando" ? "Verificando…" : "Verificar atualizações"}
          </button>
          {#if atualizacao.resultadoManual}<span class="det" style="margin: 0">{atualizacao.resultadoManual}</span>{/if}
        </div>

        <h3 class="sub-titulo">Suporte</h3>
        <div class="linha" style="align-items: center">
          <button type="button" onclick={abrirLogs}>Abrir pasta de logs</button>
          <span class="det" style="margin: 0">Se algo der errado, envie o arquivo mais recente desta pasta.</span>
        </div>
        {#if versao}<p class="det" style="margin-top: 32px">Taiga Assist, versão {versao}</p>{/if}
      {:else if secao === "taiga"}
        <Taiga embutido />
      {:else if secao === "git"}
        <Credencial embutido />
      {:else if secao === "pastas"}
        <Pasta embutido />
      {:else if secao === "kiro"}
        <Kiro embutido />
      {:else}
        <Prereq embutido />
      {/if}
    {/key}
  </section>
</div>
