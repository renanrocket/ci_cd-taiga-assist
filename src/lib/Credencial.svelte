<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let {
    onNext,
    onBack,
    embutido = false,
  }: { onNext?: () => void; onBack?: () => void; embutido?: boolean } = $props();

  type Cred = { id: number; nome: string; tipo: string; usuario: string | null };
  type Link = {
    folder_id: number;
    folder: string;
    rel: string;
    nome: string;
    cred_id: number | null;
    remote_url: string | null;
  };
  type Teste = { ok: boolean; detail: string; carregando?: boolean };

  const TIPOS: Record<string, string> = {
    sistema: "Usar o git do sistema (já configurado nesta máquina)",
    token: "Token de acesso (HTTPS)",
    userpass: "Usuário e senha (HTTPS)",
    ssh_key: "Chave SSH privada",
  };

  let creds = $state<Cred[]>([]);
  let links = $state<Link[]>([]);
  let testes = $state<Record<string, Teste>>({});
  let erro = $state("");
  let ocupado = $state(false);

  let nome = $state("");
  let tipo = $state("sistema");
  let usuario = $state("");
  let segredo = $state("");

  const chave = (l: Link) => `${l.folder_id}:${l.rel}`;

  async function guard(fn: () => Promise<void>) {
    ocupado = true;
    erro = "";
    try {
      await fn();
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = false;
    }
  }

  const carregar = () =>
    guard(async () => {
      creds = await invoke<Cred[]>("list_git_credentials");
      links = await invoke<Link[]>("list_repo_links");
    });

  const adicionar = () =>
    guard(async () => {
      creds = await invoke<Cred[]>("add_git_credential", {
        nome,
        tipo,
        usuario: usuario.trim() || null,
        segredo: segredo || null,
      });
      nome = usuario = segredo = "";
      tipo = "sistema";
    });

  const remover = (id: number) =>
    guard(async () => {
      creds = await invoke<Cred[]>("remove_git_credential", { id });
      links = await invoke<Link[]>("list_repo_links");
      testes = {};
    });

  async function vincular(l: Link, valor: string) {
    const credId = valor ? Number(valor) : null;
    await guard(async () => {
      await invoke("set_repo_credential", { folderId: l.folder_id, rel: l.rel, credId });
      l.cred_id = credId;
      delete testes[chave(l)];
    });
  }

  async function testar(l: Link) {
    testes[chave(l)] = { ok: false, detail: "Testando…", carregando: true };
    try {
      const r = await invoke<{ ok: boolean; detail: string }>("test_repo_access", {
        folderId: l.folder_id,
        rel: l.rel,
      });
      testes[chave(l)] = r;
    } catch (e) {
      testes[chave(l)] = { ok: false, detail: String(e) };
    }
  }

  const algumVinculado = $derived(links.some((l) => l.cred_id !== null));
  const precisaUsuario = $derived(tipo === "userpass");
  const mostraUsuario = $derived(tipo === "userpass" || tipo === "token" || tipo === "ssh_key");
  const rotuloUsuario = $derived(
    tipo === "userpass" ? "Usuário" : tipo === "ssh_key" ? "Usuário SSH (opcional, ex.: git)" : "Usuário (opcional)",
  );
  const podeAdicionar = $derived(
    nome.trim() !== "" && (tipo === "sistema" || segredo.trim() !== "") && (!precisaUsuario || usuario.trim() !== ""),
  );

  onMount(carregar);
</script>

<p class="det" style="margin-top: 0">
  Cadastre como o app acessa o remoto dos seus repositórios e vincule uma credencial a cada um. Os segredos
  ficam no chaveiro do sistema, nunca em arquivo.
</p>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}

<h2>Credenciais</h2>
<form
  class="campo form"
  onsubmit={(e) => {
    e.preventDefault();
    adicionar();
  }}
>
  <label for="cred-nome">Nome</label>
  <input id="cred-nome" bind:value={nome} placeholder="Ex.: GitHub pessoal" disabled={ocupado} />

  <label for="cred-tipo">Tipo</label>
  <select id="cred-tipo" bind:value={tipo} disabled={ocupado}>
    {#each Object.entries(TIPOS) as [valor, rotulo]}
      <option value={valor}>{rotulo}</option>
    {/each}
  </select>

  {#if mostraUsuario}
    <label for="cred-usuario">{rotuloUsuario}</label>
    <input id="cred-usuario" bind:value={usuario} disabled={ocupado} autocomplete="off" />
  {/if}

  {#if tipo === "token" || tipo === "userpass"}
    <label for="cred-segredo">{tipo === "token" ? "Token" : "Senha"}</label>
    <input id="cred-segredo" type="password" bind:value={segredo} disabled={ocupado} autocomplete="new-password" />
  {:else if tipo === "ssh_key"}
    <label for="cred-chave">Chave privada</label>
    <textarea id="cred-chave" bind:value={segredo} rows="5" disabled={ocupado} placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"></textarea>
  {/if}

  <div class="linha" style="justify-content: flex-end">
    <button type="submit" class="primario" disabled={ocupado || !podeAdicionar}>Adicionar credencial</button>
  </div>
</form>

<ul class="lista" style="margin-top: 16px">
  {#each creds as c (c.id)}
    <li class="item">
      <div class="cab">
        <strong>{c.nome}</strong>
        <span class="det" style="margin: 0">{TIPOS[c.tipo] ?? c.tipo}{c.usuario ? ` · ${c.usuario}` : ""}</span>
      </div>
      <div class="linha" style="margin-top: 10px; justify-content: flex-end">
        <button type="button" class="perigo" onclick={() => remover(c.id)} disabled={ocupado}>Remover</button>
      </div>
    </li>
  {:else}
    <li class="vazio">Nenhuma credencial ainda. Crie uma abaixo.</li>
  {/each}
</ul>

<h2>Repositórios</h2>
<ul class="lista">
  {#each links as l (chave(l))}
    {@const t = testes[chave(l)]}
    <li class="item" class:ok={t?.ok} class:falha={t && !t.ok && !t.carregando}>
      <div class="cab">
        <strong>{l.nome}</strong>
        {#if t}<span class="selo">{t.ok ? "acesso OK" : t.carregando ? "testando…" : "falhou"}</span>{/if}
      </div>
      <p class="det">{l.remote_url ?? "sem remote 'origin'"}</p>
      <div class="linha" style="margin-top: 10px">
        <select
          value={l.cred_id === null ? "" : String(l.cred_id)}
          onchange={(e) => vincular(l, e.currentTarget.value)}
          disabled={ocupado}
          aria-label={`Credencial de ${l.nome}`}
        >
          <option value="">— escolha a credencial —</option>
          {#each creds as c (c.id)}
            <option value={String(c.id)}>{c.nome}</option>
          {/each}
        </select>
        <button type="button" onclick={() => testar(l)} disabled={ocupado || l.cred_id === null || t?.carregando}>
          Testar
        </button>
      </div>
      {#if t && !t.carregando}<p class="det" class:erro={!t.ok}>{t.detail}</p>{/if}
    </li>
  {:else}
    <li class="vazio">Nenhum repositório encontrado. Volte e cadastre uma pasta.</li>
  {/each}
</ul>

{#if !embutido}
  <div class="acoes">
    <button type="button" onclick={onBack}>Voltar</button>
    <button type="button" class="primario" disabled={!algumVinculado} onclick={onNext}>Continuar</button>
  </div>
{/if}

<style>
  h2 {
    font-size: 1rem;
    margin: 24px 0 8px;
  }
  .form {
    display: grid;
    gap: 6px;
    margin-top: 12px;
    padding: 14px 16px;
    border: 1px dashed var(--borda);
    border-radius: 8px;
  }
</style>
