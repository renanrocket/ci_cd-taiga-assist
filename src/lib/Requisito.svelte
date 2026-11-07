<script lang="ts">
  // Aviso padrão de "falta configurar X": aparece no lugar (ou ao lado) de um recurso que depende de outro.
  // O recurso fica desabilitado, nunca escondido, e o botão leva direto à configuração que falta.
  let { texto, acao, onAcao }: { texto: string; acao?: string; onAcao?: () => void } = $props();
</script>

<div class="item aviso" role="note">
  <p class="det" style="margin: 0">{texto}</p>
  {#if acao && onAcao}
    <div class="linha" style="justify-content: flex-end; margin-top: 8px">
      <button type="button" onclick={onAcao}>{acao}</button>
    </div>
  {/if}
</div>
