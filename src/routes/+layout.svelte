<script lang="ts">
  import "../lib/app.css";
  import AtualizacaoAviso from "$lib/AtualizacaoAviso.svelte";
  import Notificacoes from "$lib/Notificacoes.svelte";

  let { children } = $props();
</script>

<AtualizacaoAviso />
{@render children()}
<Notificacoes />
