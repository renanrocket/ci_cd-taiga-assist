<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  let {
    onNext,
    onBack,
    embutido = false,
  }: { onNext?: () => void; onBack?: () => void; embutido?: boolean } = $props();

  type Repo = { rel: string; nome: string };
  type Folder = { id: number; caminho: string; existe: boolean; repos: Repo[] };

  let pastas = $state<Folder[]>([]);
  let caminho = $state("");
  let erro = $state("");
  let ocupado = $state(false);

  async function executar(fn: () => Promise<Folder[]>) {
    ocupado = true;
    erro = "";
    try {
      pastas = await fn();
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = false;
    }
  }

  const carregar = () => executar(() => invoke<Folder[]>("list_folders"));

  async function escolher() {
    const sel = await open({ directory: true, multiple: false, title: "Escolha a pasta dos seus projetos" });
    if (typeof sel === "string") await adicionar(sel);
  }

  async function adicionar(path: string) {
    if (!path.trim()) return;
    await executar(() => invoke<Folder[]>("add_folder", { path }));
    if (!erro) caminho = "";
  }

  const remover = (id: number) => executar(() => invoke<Folder[]>("remove_folder", { id }));

  const totalRepos = $derived(pastas.reduce((n, p) => n + p.repos.length, 0));

  onMount(carregar);
</script>

<p class="det" style="margin-top: 0">
  Cadastre a pasta onde ficam seus projetos. Se a própria pasta for um repositório git, ela é usada;
  senão, usamos as subpastas que forem repositórios.
</p>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}

<form
  class="campo"
  onsubmit={(e) => {
    e.preventDefault();
    adicionar(caminho);
  }}
>
  <label for="caminho-pasta">Caminho da pasta</label>
  <div class="linha">
    <input id="caminho-pasta" bind:value={caminho} placeholder="/home/voce/Projetos" disabled={ocupado} />
    <button type="button" onclick={escolher} disabled={ocupado}>Escolher…</button>
    <button type="submit" class="primario" disabled={ocupado || !caminho.trim()}>Adicionar</button>
  </div>
</form>

<ul class="lista" style="margin-top: 16px">
  {#each pastas as p (p.id)}
    <li class="item" class:ok={p.existe && p.repos.length > 0} class:aviso={p.existe && p.repos.length === 0} class:falha={!p.existe}>
      <div class="cab">
        <strong><code>{p.caminho}</code></strong>
        <span class="selo">
          {#if !p.existe}pasta não encontrada
          {:else if p.repos.length === 0}nenhum repositório git
          {:else}{p.repos.length} repositório(s){/if}
        </span>
      </div>
      {#if p.repos.length > 0}
        <p class="det">{p.repos.map((r) => (r.rel === "." ? `${r.nome} (esta pasta)` : r.nome)).join(" · ")}</p>
      {/if}
      <div class="linha" style="margin-top: 10px; justify-content: flex-end">
        <button type="button" class="perigo" onclick={() => remover(p.id)} disabled={ocupado}>Remover</button>
      </div>
    </li>
  {:else}
    <li class="vazio">Nenhuma pasta cadastrada ainda.</li>
  {/each}
</ul>

{#if !embutido}
  <div class="acoes">
    <button type="button" onclick={onBack}>Voltar</button>
    <button type="button" class="primario" disabled={totalRepos === 0} onclick={onNext}>Continuar</button>
  </div>
{/if}
