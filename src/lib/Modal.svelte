<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    titulo,
    confirmar = "Confirmar",
    cancelar = "Cancelar",
    onConfirm,
    onCancel,
    children,
  }: {
    titulo: string;
    /** `null` esconde o botão de confirmar (modal só informativo). */
    confirmar?: string | null;
    cancelar?: string;
    onConfirm?: () => void;
    onCancel: () => void;
    children: Snippet;
  } = $props();

  function tecla(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
  }
</script>

<svelte:window onkeydown={tecla} />

<div class="fundo" role="presentation" onclick={onCancel}>
  <div class="caixa" role="dialog" aria-modal="true" aria-label={titulo} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <h2>{titulo}</h2>
    <div class="corpo">{@render children()}</div>
    <div class="acoes">
      <button type="button" onclick={onCancel}>{cancelar}</button>
      {#if confirmar !== null}
        <button type="button" class="primario" onclick={onConfirm}>{confirmar}</button>
      {:else}
        <span></span>
      {/if}
    </div>
  </div>
</div>

<style>
  .fundo {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: grid;
    place-items: center;
    padding: 16px;
    z-index: 100;
  }
  .caixa {
    background: var(--card);
    border: 1px solid var(--borda);
    border-radius: 10px;
    padding: 20px;
    width: min(520px, 100%);
    max-height: 85vh;
    overflow: auto;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 1.1rem;
  }
  .corpo {
    font-size: 0.92rem;
  }
  .acoes {
    margin-top: 16px;
  }
</style>
