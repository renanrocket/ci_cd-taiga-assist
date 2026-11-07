<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import { atualizacao, PAGINA_DOWNLOADS } from "$lib/atualizacao.svelte";

  onMount(() => atualizacao.iniciar());

  const visivel = $derived(
    !!atualizacao.disponivel && (!atualizacao.adiada || ["baixando", "reiniciando", "erro"].includes(atualizacao.estado)),
  );
  const ocupado = $derived(atualizacao.estado === "baixando" || atualizacao.estado === "reiniciando");
</script>

{#if visivel && atualizacao.disponivel}
  <aside class="aviso" role="status" aria-live="polite">
    <div class="topo">
      <div>
        <strong>Nova versão disponível: {atualizacao.disponivel.versao}</strong>
        {#if atualizacao.versaoAtual}<span class="det" style="margin: 0 0 0 8px">(você está na {atualizacao.versaoAtual})</span>{/if}
      </div>
      <div class="linha" style="flex: none; gap: 8px">
        {#if atualizacao.modo === "auto"}
          <button type="button" onclick={() => atualizacao.adiar()} disabled={ocupado}>Agora não</button>
          <button type="button" class="primario" onclick={() => atualizacao.instalar()} disabled={ocupado}>
            {atualizacao.estado === "baixando" ? "Baixando…" : atualizacao.estado === "reiniciando" ? "Reiniciando…" : "Atualizar agora"}
          </button>
        {:else}
          <button type="button" onclick={() => atualizacao.adiar()}>Agora não</button>
          <button type="button" class="primario" onclick={() => openUrl(PAGINA_DOWNLOADS)}>Baixar nova versão</button>
        {/if}
      </div>
    </div>

    {#if atualizacao.estado === "baixando"}
      <progress max="100" value={atualizacao.progresso ?? undefined} aria-label="Progresso do download"></progress>
    {/if}
    {#if atualizacao.erro}<p class="erro" style="margin: 6px 0 0">{atualizacao.erro}</p>{/if}
    {#if atualizacao.modo === "manual"}
      <p class="det" style="margin: 6px 0 0">
        Esta instalação (.deb) não se atualiza sozinha: baixe o novo pacote na página de versões. Com o AppImage a
        atualização é feita pelo próprio app.
      </p>
    {:else}
      <p class="det" style="margin: 6px 0 0">Seus dados e configurações são mantidos. O app reinicia ao terminar.</p>
    {/if}
    {#if atualizacao.disponivel.notas}
      <details>
        <summary class="det">O que mudou</summary>
        <pre class="notas">{atualizacao.disponivel.notas}</pre>
      </details>
    {/if}
  </aside>
{/if}

<style>
  .aviso {
    position: sticky;
    top: 0;
    z-index: 60;
    background: var(--card);
    border-bottom: 3px solid var(--acento);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    padding: 10px 16px;
  }
  .topo {
    max-width: 960px;
    margin: 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .aviso > :not(.topo) {
    max-width: 960px;
    margin-left: auto;
    margin-right: auto;
  }
  progress {
    display: block;
    width: 100%;
    margin-top: 8px;
  }
  .notas {
    white-space: pre-wrap;
    background: var(--bg);
    border: 1px solid var(--borda);
    border-radius: 6px;
    padding: 8px 10px;
    font-family: inherit;
    font-size: 0.85rem;
    max-height: 30vh;
    overflow: auto;
  }
</style>
