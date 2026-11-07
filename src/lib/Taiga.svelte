<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let {
    onNext,
    onBack,
    embutido = false,
  }: { onNext?: () => void; onBack?: () => void; embutido?: boolean } = $props();

  type Conta = { id: number; nome: string; host: string; usuario: string };
  type Projeto = { id: number; account_id: number; account_nome: string; slug: string; nome: string };
  type Remoto = { slug: string; nome: string; importado: boolean };
  type Link = {
    folder_id: number;
    rel: string;
    nome: string;
    project_id: number | null;
    branch: string | null;
    remote_url: string | null;
  };
  type Teste = { ok: boolean; detail: string; carregando?: boolean };

  let contas = $state<Conta[]>([]);
  let projetos = $state<Projeto[]>([]);
  let links = $state<Link[]>([]);
  let branches = $state<Record<string, string[]>>({});
  let testes = $state<Record<string, Teste>>({});
  let erro = $state("");
  let ocupado = $state(false);

  // formulário da conta
  let nome = $state("");
  let host = $state("");
  let usuario = $state("");
  let senha = $state("");

  // importação de projetos
  let importando = $state<number | null>(null); // id da conta
  let remotos = $state<Remoto[]>([]);
  let marcados = $state<Record<string, boolean>>({});

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

  async function carregar() {
    await guard(async () => {
      contas = await invoke<Conta[]>("list_taiga_accounts");
      projetos = await invoke<Projeto[]>("list_taiga_projects");
      links = await invoke<Link[]>("list_repo_links");
    });
    for (const l of links) {
      invoke<string[]>("list_repo_branches", { folderId: l.folder_id, rel: l.rel })
        .then((b) => (branches[chave(l)] = b))
        .catch(() => (branches[chave(l)] = []));
    }
  }

  const adicionarConta = () =>
    guard(async () => {
      contas = await invoke<Conta[]>("add_taiga_account", { nome, host, usuario, senha });
      nome = host = usuario = senha = "";
    });

  const removerConta = (id: number) =>
    guard(async () => {
      contas = await invoke<Conta[]>("remove_taiga_account", { id });
      projetos = await invoke<Projeto[]>("list_taiga_projects");
      links = await invoke<Link[]>("list_repo_links");
      if (importando === id) importando = null;
      testes = {};
    });

  const abrirImportacao = (id: number) =>
    guard(async () => {
      remotos = await invoke<Remoto[]>("fetch_remote_projects", { accountId: id });
      marcados = Object.fromEntries(remotos.filter((r) => !r.importado).map((r) => [r.slug, false]));
      importando = id;
    });

  const novos = $derived(remotos.filter((r) => !r.importado));
  const todosMarcados = $derived(novos.length > 0 && novos.every((r) => marcados[r.slug]));
  const qtdMarcados = $derived(novos.filter((r) => marcados[r.slug]).length);

  function marcarTodos(valor: boolean) {
    for (const r of novos) marcados[r.slug] = valor;
  }

  const importar = () =>
    guard(async () => {
      if (importando === null) return;
      const slugs = novos.filter((r) => marcados[r.slug]).map((r) => r.slug);
      projetos = await invoke<Projeto[]>("import_taiga_projects", { accountId: importando, slugs });
      importando = null;
    });

  const removerProjeto = (id: number) =>
    guard(async () => {
      projetos = await invoke<Projeto[]>("remove_taiga_project", { id });
      links = await invoke<Link[]>("list_repo_links");
      testes = {};
    });

  async function salvarVinculo(l: Link, projectId: number | null, branch: string | null) {
    await guard(async () => {
      await invoke("set_repo_taiga", { folderId: l.folder_id, rel: l.rel, projectId, branch });
      l.project_id = projectId;
      l.branch = branch;
      delete testes[chave(l)];
    });
  }

  async function testar(l: Link) {
    testes[chave(l)] = { ok: false, detail: "Testando…", carregando: true };
    try {
      testes[chave(l)] = await invoke<Teste>("test_repo_taiga", { folderId: l.folder_id, rel: l.rel });
    } catch (e) {
      testes[chave(l)] = { ok: false, detail: String(e) };
    }
  }

  const completo = (l: Link) => l.project_id !== null && !!l.branch;
  const algumCompleto = $derived(links.some(completo));
  const podeAdicionar = $derived(!!(nome.trim() && host.trim() && usuario.trim() && senha));

  onMount(carregar);
</script>

<p class="det" style="margin-top: 0">
  Cadastre sua conta do Taiga, importe os projetos que você usa e diga, para cada repositório, qual é o projeto e
  qual branch dispara o relatório e a criação da história (US).
</p>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}

<h2>Contas Taiga</h2>
<form
  class="campo form"
  onsubmit={(e) => {
    e.preventDefault();
    adicionarConta();
  }}
>
  <label for="tg-nome">Nome da conta</label>
  <input id="tg-nome" bind:value={nome} placeholder="Ex.: Taiga do trabalho" disabled={ocupado} />
  <label for="tg-host">URL do Taiga</label>
  <input id="tg-host" bind:value={host} placeholder="https://taiga.suaempresa.com.br" disabled={ocupado} />
  <label for="tg-user">Usuário</label>
  <input id="tg-user" bind:value={usuario} autocomplete="off" disabled={ocupado} />
  <label for="tg-senha">Senha</label>
  <input id="tg-senha" type="password" bind:value={senha} autocomplete="new-password" disabled={ocupado} />
  <div class="linha" style="justify-content: flex-end">
    <button type="submit" class="primario" disabled={ocupado || !podeAdicionar}>Adicionar conta (testa o login)</button>
  </div>
</form>

<ul class="lista" style="margin-top: 16px">
  {#each contas as c (c.id)}
    <li class="item">
      <div class="cab">
        <strong>{c.nome}</strong>
        <span class="det" style="margin: 0">{c.usuario} · {c.host}</span>
      </div>
      <div class="linha" style="margin-top: 10px; justify-content: flex-end">
        <button type="button" onclick={() => abrirImportacao(c.id)} disabled={ocupado}>Importar projetos…</button>
        <button type="button" class="perigo" onclick={() => removerConta(c.id)} disabled={ocupado}>Remover</button>
      </div>

      {#if importando === c.id}
        <div class="importacao">
          {#if novos.length === 0}
            <p class="det">Todos os seus projetos neste Taiga já foram importados.</p>
          {:else}
            <label class="marca">
              <input type="checkbox" checked={todosMarcados} onchange={(e) => marcarTodos(e.currentTarget.checked)} />
              Selecionar todos
            </label>
            {#each novos as r (r.slug)}
              <label class="marca">
                <input type="checkbox" bind:checked={marcados[r.slug]} />
                {r.nome} <code>{r.slug}</code>
              </label>
            {/each}
          {/if}
          <div class="linha" style="justify-content: flex-end; margin-top: 8px">
            <button type="button" onclick={() => (importando = null)}>Cancelar</button>
            <button type="button" class="primario" onclick={importar} disabled={ocupado || qtdMarcados === 0}>
              Importar selecionados ({qtdMarcados})
            </button>
          </div>
        </div>
      {/if}
    </li>
  {:else}
    <li class="vazio">Nenhuma conta ainda. Cadastre abaixo.</li>
  {/each}
</ul>

<h2>Projetos importados</h2>
<ul class="lista">
  {#each projetos as p (p.id)}
    <li class="item">
      <div class="cab">
        <span><strong>{p.nome}</strong> <code>{p.slug}</code></span>
        <span class="det" style="margin: 0">{p.account_nome}</span>
      </div>
      <div class="linha" style="margin-top: 10px; justify-content: flex-end">
        <button type="button" class="perigo" onclick={() => removerProjeto(p.id)} disabled={ocupado}>Remover</button>
      </div>
    </li>
  {:else}
    <li class="vazio">Nenhum projeto importado. Use “Importar projetos…” em uma conta.</li>
  {/each}
</ul>

<h2>Repositórios</h2>
<ul class="lista">
  {#each links as l (chave(l))}
    {@const t = testes[chave(l)]}
    <li class="item" class:ok={t?.ok} class:falha={t && !t.ok && !t.carregando}>
      <div class="cab">
        <strong>{l.nome}</strong>
        {#if t}<span class="selo">{t.ok ? "projeto OK" : t.carregando ? "testando…" : "falhou"}</span>{/if}
      </div>
      <p class="det">{l.remote_url ?? "sem remote 'origin'"}</p>
      <div class="campo" style="margin-top: 10px">
        <label for={"pr-" + chave(l)}>Projeto Taiga</label>
        <select
          id={"pr-" + chave(l)}
          value={l.project_id === null ? "" : String(l.project_id)}
          onchange={(e) => salvarVinculo(l, e.currentTarget.value ? Number(e.currentTarget.value) : null, l.branch)}
          disabled={ocupado}
        >
          <option value="">— escolha o projeto —</option>
          {#each projetos as p (p.id)}
            <option value={String(p.id)}>{p.nome} ({p.account_nome})</option>
          {/each}
        </select>
      </div>
      <div class="campo" style="margin-top: 8px">
        <label for={"br-" + chave(l)}>Branch que dispara o relatório e a US</label>
        <div class="linha">
          <select
            id={"br-" + chave(l)}
            value={l.branch ?? ""}
            onchange={(e) => salvarVinculo(l, l.project_id, e.currentTarget.value || null)}
            disabled={ocupado}
          >
            <option value="">— escolha a branch —</option>
            {#each branches[chave(l)] ?? [] as b (b)}
              <option value={b}>{b}</option>
            {/each}
          </select>
          <button type="button" onclick={() => testar(l)} disabled={ocupado || l.project_id === null || t?.carregando}>
            Testar
          </button>
        </div>
      </div>
      {#if t && !t.carregando}<p class="det" class:erro={!t.ok}>{t.detail}</p>{/if}
    </li>
  {:else}
    <li class="vazio">Nenhum repositório encontrado.</li>
  {/each}
</ul>

{#if !embutido}
  <div class="acoes">
    <button type="button" onclick={onBack}>Voltar</button>
    <button type="button" class="primario" disabled={!algumCompleto} onclick={onNext}>Continuar</button>
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
  .importacao {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--borda);
    display: grid;
    gap: 6px;
  }
  .marca {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.9rem;
    color: var(--texto);
    margin: 0;
  }
  .marca input {
    flex: none;
  }
</style>
