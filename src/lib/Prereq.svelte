<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { onNext, embutido = false }: { onNext?: () => void; embutido?: boolean } = $props();

  type ToolStatus = {
    tool: "git" | "kiro";
    found: boolean;
    path: string | null;
    version: string | null;
    custom: boolean;
    hint: string;
    error: string | null;
  };

  const NOMES: Record<string, string> = { git: "Git", kiro: "Kiro CLI" };

  let tools = $state<ToolStatus[]>([]);
  let caminhos = $state<Record<string, string>>({});
  let carregando = $state(true);
  let erro = $state("");

  async function verificar() {
    carregando = true;
    erro = "";
    try {
      tools = await invoke<ToolStatus[]>("check_prerequisites");
      for (const t of tools) caminhos[t.tool] ??= t.custom ? (t.path ?? "") : "";
    } catch (e) {
      erro = String(e);
    } finally {
      carregando = false;
    }
  }

  async function salvarCaminho(tool: string) {
    erro = "";
    try {
      const novo = await invoke<ToolStatus>("set_tool_path", { tool, path: caminhos[tool] ?? "" });
      tools = tools.map((t) => (t.tool === tool ? novo : t));
    } catch (e) {
      erro = String(e);
    }
  }

  const tudoOk = $derived(tools.length > 0 && tools.every((t) => t.found));

  onMount(verificar);
</script>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}

<ul class="lista">
  {#each tools as t (t.tool)}
    <li class="item" class:ok={t.found} class:falha={!t.found}>
      <div class="cab">
        <strong>{NOMES[t.tool]}</strong>
        <span class="selo">{t.found ? "encontrado" : "não encontrado"}</span>
      </div>
      {#if t.found}
        <p class="det">{t.version} — <code>{t.path}</code></p>
      {:else}
        {#if t.error}<p class="det">{t.error}</p>{/if}
        <p class="det">{t.hint}</p>
      {/if}
      <form
        class="campo"
        style="margin-top: 12px"
        onsubmit={(e) => {
          e.preventDefault();
          salvarCaminho(t.tool);
        }}
      >
        <label for={"p-" + t.tool}>Caminho do executável (opcional)</label>
        <div class="linha">
          <input id={"p-" + t.tool} bind:value={caminhos[t.tool]} placeholder="detectar automaticamente" />
          <button type="submit">Salvar e testar</button>
        </div>
      </form>
    </li>
  {/each}
</ul>

<div class="acoes">
  <button type="button" onclick={verificar} disabled={carregando}>
    {carregando ? "Verificando…" : "Verificar novamente"}
  </button>
  {#if !embutido}
    <button type="button" class="primario" disabled={!tudoOk} onclick={onNext}>Continuar</button>
  {:else}
    <span></span>
  {/if}
</div>
