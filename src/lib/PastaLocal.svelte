<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Requisito from "$lib/Requisito.svelte";
  import { capacidades } from "$lib/capacidades.svelte";
  import { nav } from "$lib/nav.svelte";
  import { onMount } from "svelte";
  import Modal from "$lib/Modal.svelte";
  import { tarefas } from "$lib/tasks.svelte";

  type Pasta = { id: number; caminho: string; existe: boolean };
  type Branch = { nome: string; atual: boolean; atras_de: string[] };
  type Repo = {
    rel: string;
    nome: string;
    atual: string | null;
    sujo: number;
    branches: Branch[];
    cred_id: number | null;
    cred_nome: string | null;
    projeto: string | null;
    branch_gatilho: string | null;
    pendentes: number;
    remote_url: string | null;
  };

  const ESTADOS: Record<string, { texto: string; classe: string; dica: string }> = {
    ok: { texto: "em dia", classe: "", dica: "Igual ao remoto." },
    adiantado: { texto: "a enviar", classe: "info", dica: "Tem commits locais que ainda não estão no remoto." },
    desatualizado: { texto: "desatualizada", classe: "aviso", dica: "O remoto tem commits que esta branch não tem." },
    sem_remoto: { texto: "só local", classe: "", dica: "A branch não existe no remoto." },
  };

  let pastas = $state<Pasta[]>([]);
  let pastaId = $state<number | null>(null);
  let repos = $state<Repo[]>([]);
  let remoto = $state<Record<string, Record<string, string>>>({});
  let remotoErro = $state<Record<string, string>>({});
  let marcadas = $state<Record<string, boolean>>({});
  let criarUs = $state(true); // global: desmarcado = sem relatório nem US em nenhum repositório
  let mensagem = $state("");
  let push = $state(true);
  let carregando = $state(true);
  let erro = $state("");
  let aviso = $state("");
  let confirmando = $state(false);

  const chave = (r: Repo, b: Branch) => `${r.rel}::${b.nome}`;

  // "Atualizar do remoto" (git pull só fast-forward) de uma branch; recusa se divergiu ou se há conflito local.
  let atualizando = $state<Record<string, boolean>>({});
  async function atualizarBranch(r: Repo, b: Branch) {
    const k = chave(r, b);
    atualizando[k] = true;
    erro = aviso = "";
    try {
      aviso = `${r.nome} · ${b.nome}: ` + (await invoke<string>("update_branch", { folderId: pastaId, rel: r.rel, branch: b.nome }));
      await carregarRepos();
    } catch (e) {
      erro = `${r.nome} · ${b.nome}: ${e}`;
    } finally {
      atualizando[k] = false;
    }
  }
  const temTaiga = (r: Repo) => !!(r.projeto && r.branch_gatilho);
  // Relatório + US só existem com Kiro pronto E conta Taiga; senão o app faz só commit e push.
  const podeUs = $derived(capacidades.taigaPronto);

  async function carregarRepos() {
    if (pastaId === null) return;
    carregando = true;
    erro = "";
    try {
      repos = await invoke<Repo[]>("folder_overview", { folderId: pastaId });
      // O estado frente ao remoto vem depois, sem travar a tela.
      for (const r of repos) {
        if (r.cred_id === null) continue;
        delete remotoErro[r.rel];
        invoke<Record<string, string>>("remote_states", { folderId: pastaId, rel: r.rel })
          .then((m) => (remoto[r.rel] = m))
          .catch((e) => (remotoErro[r.rel] = String(e)));
      }
    } catch (e) {
      erro = String(e);
      repos = [];
    } finally {
      carregando = false;
    }
  }

  onMount(async () => {
    capacidades.atualizar();
    try {
      pastas = (await invoke<Pasta[]>("list_folders")).filter((p) => p.existe);
      pastaId = pastas[0]?.id ?? null;
      await carregarRepos();
    } catch (e) {
      erro = String(e);
      carregando = false;
    }
  });

  // Quando uma tarefa termina (commit/reenvio), recarrega o status.
  let vistas = "";
  $effect(() => {
    const v = tarefas.versaoTerminadas;
    if (v !== vistas) {
      const primeira = vistas === "";
      vistas = v;
      if (!primeira) carregarRepos();
    }
  });

  function trocarPasta(valor: string) {
    pastaId = Number(valor);
    marcadas = {};
    remoto = {};
    carregarRepos();
  }

  const bloqueada = (r: Repo) => push && r.cred_id === null;
  const todas = (r: Repo) => r.branches.length > 0 && r.branches.every((b) => marcadas[chave(r, b)]);
  function marcarTodas(r: Repo, valor: boolean) {
    for (const b of r.branches) marcadas[chave(r, b)] = valor;
  }

  const qtd = $derived(Object.values(marcadas).filter(Boolean).length);
  const itens = $derived(
    repos
      .map((r) => ({
        rel: r.rel,
        nome: r.nome,
        branches: r.branches.filter((b) => marcadas[chave(r, b)]).map((b) => b.nome),
        taiga: criarUs && podeUs && push && temTaiga(r),
        gatilho: r.branch_gatilho,
        sujo: r.sujo,
        atual: r.atual,
      }))
      .filter((i) => i.branches.length > 0),
  );
  // A mensagem só é obrigatória se há alterações pendentes na branch atual marcada.
  const precisaMensagem = $derived(itens.some((i) => i.sujo > 0 && i.atual !== null && i.branches.includes(i.atual)));
  const podeCommitar = $derived(qtd > 0 && (!precisaMensagem || mensagem.trim() !== "") && !itens.some((i) => push && repos.find((r) => r.rel === i.rel)?.cred_id === null));

  async function commitar() {
    confirmando = false;
    erro = aviso = "";
    try {
      await invoke("start_commit", {
        folderId: pastaId,
        items: itens.map((i) => ({ rel: i.rel, branches: i.branches, taiga: i.taiga })),
        message: mensagem,
        push,
      });
      aviso = "Commit iniciado em segundo plano. Acompanhe pela caixa de notificações.";
      marcadas = {};
    } catch (e) {
      erro = String(e);
    }
  }

  async function reenviar(r: Repo) {
    erro = aviso = "";
    try {
      await invoke("start_resend", { folderId: pastaId, rel: r.rel });
      aviso = "Reenvio iniciado em segundo plano.";
    } catch (e) {
      erro = String(e);
    }
  }
</script>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}
{#if aviso}<p class="det" style="color: var(--ok)">{aviso}</p>{/if}

{#if pastas.length === 0 && !carregando}
  <p class="vazio">Nenhuma pasta local cadastrada. Abra <strong>Configurações</strong> para cadastrar.</p>
{:else}
  <div class="linha" style="align-items: center; margin-bottom: 16px">
    {#if pastas.length > 1}
      <select value={String(pastaId)} onchange={(e) => trocarPasta(e.currentTarget.value)} aria-label="Pasta local">
        {#each pastas as p (p.id)}<option value={String(p.id)}>{p.caminho}</option>{/each}
      </select>
    {:else if pastas[0]}
      <code style="flex: 1">{pastas[0].caminho}</code>
    {/if}
    <button type="button" onclick={carregarRepos} disabled={carregando}>{carregando ? "Carregando…" : "Atualizar"}</button>
  </div>

  <ul class="lista">
    {#each repos as r (r.rel)}
      <li class="item" class:ok={r.cred_id !== null && r.sujo === 0} class:aviso={r.sujo > 0}>
        <div class="cab">
          <strong>{r.nome}</strong>
          <span class="linha" style="flex-wrap: wrap; justify-content: flex-end">
            {#if r.sujo > 0}<span class="badge aviso">{r.sujo} alteração(ões)</span>{:else}<span class="badge">limpo</span>{/if}
            {#if r.cred_nome}<span class="badge ok" title="Usada no push">credencial: {r.cred_nome}</span>{:else}<span class="badge falha">sem credencial</span>{/if}
            {#if temTaiga(r)}<span class="badge info" title="Projeto Taiga">{r.projeto} · gatilho: {r.branch_gatilho}</span>{/if}
          </span>
        </div>
        {#if remotoErro[r.rel]}<p class="det">Remoto indisponível: {remotoErro[r.rel]}</p>{/if}

        {#if r.pendentes > 0}
          <div class="faixa">
            <span>{r.pendentes} relatório(s) pendente(s) de envio ao Taiga.</span>
            <button type="button" onclick={() => reenviar(r)}>Reenviar ao Taiga</button>
          </div>
        {/if}

        <div class="ramos">
          <label class="marca">
            <input type="checkbox" checked={todas(r)} disabled={bloqueada(r)} onchange={(e) => marcarTodas(r, e.currentTarget.checked)} />
            Selecionar todos
          </label>
          {#each r.branches as b (b.nome)}
            {@const st = remoto[r.rel]?.[b.nome]}
            <label class="marca" title={bloqueada(r) ? "Vincule uma credencial em Configurações para enviar (push)." : ""}>
              <input type="checkbox" bind:checked={marcadas[chave(r, b)]} disabled={bloqueada(r)} />
              <code>{b.nome}</code>
              {#if b.atual}<span class="badge info">atual</span>{/if}
              {#if b.atras_de.length > 0}
                <span class="badge aviso" title="Será avançada (fast-forward) até a branch à frente, sem criar commit.">atrás de {b.atras_de.join(", ")}</span>
              {/if}
              {#if st && ESTADOS[st]}<span class="badge {ESTADOS[st].classe}" title={ESTADOS[st].dica}>{ESTADOS[st].texto}</span>{/if}
              {#if st === "desatualizado"}
                <button
                  type="button"
                  class="mini"
                  disabled={r.cred_id === null || atualizando[chave(r, b)]}
                  title="Traz os commits do remoto para esta branch (git pull, só fast-forward). Recusa se divergiu ou se alterações locais conflitam."
                  onclick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    atualizarBranch(r, b);
                  }}
                >
                  {atualizando[chave(r, b)] ? "Atualizando…" : "Atualizar do remoto"}
                </button>
              {/if}
            </label>
          {/each}
        </div>
      </li>
    {:else}
      {#if !carregando}<li class="vazio">Nenhum repositório encontrado nesta pasta.</li>{/if}
    {/each}
  </ul>

  {#if repos.length > 0}
    <form
      class="campo envio"
      onsubmit={(e) => {
        e.preventDefault();
        confirmando = true;
      }}
    >
      <label for="msg">Mensagem do commit {precisaMensagem ? "(obrigatória: há alterações pendentes)" : "(opcional: só há promoção entre branches)"}</label>
      <textarea id="msg" rows="3" bind:value={mensagem} placeholder="Ex.: Ajusta validação do cadastro"></textarea>
      <label class="marca" style="margin-top: 8px">
        <input type="checkbox" bind:checked={push} />
        Fazer push depois do commit
      </label>
      <label
        class="marca"
        style="margin-top: 6px; {push ? '' : 'opacity: 0.5'}"
        title="Marcado: depois do push da branch gatilho de cada repositório com Taiga, gera o relatório com o Kiro e cria a US. Desmarcado: nenhum relatório nem US é criado neste commit, mesmo com Taiga e Kiro configurados. Requer o push."
      >
        <input type="checkbox" checked={criarUs && podeUs} onchange={(e) => (criarUs = e.currentTarget.checked)} disabled={!push || !podeUs} />
        Criar US no Taiga (gera o relatório com o Kiro)
      </label>
      {#if capacidades.carregado && !podeUs}
        <div style="margin-top: 6px">
          <Requisito
            texto={capacidades.kiroPronto
              ? "Para criar a US a cada push, cadastre sua conta do Taiga. Sem isso o app faz só commit e push."
              : "Para criar a US a cada push, configure o Kiro e depois o Taiga. Sem isso o app faz só commit e push."}
            acao={capacidades.kiroPronto ? "Configurar o Taiga" : "Configurar o Kiro"}
            onAcao={() => nav.ir("config", capacidades.kiroPronto ? "taiga" : "kiro")}
          />
        </div>
      {/if}
      <div class="linha" style="justify-content: flex-end; margin-top: 8px">
        <button type="submit" class="primario" disabled={!podeCommitar}>Commitar ({qtd} branch(es))</button>
      </div>
    </form>
  {/if}
{/if}

{#if confirmando}
  <Modal titulo="Confirmar commit" confirmar="Commitar" onConfirm={commitar} onCancel={() => (confirmando = false)}>
    {#each itens as i}
      <p style="margin: 6px 0">
        <strong>{i.nome}</strong>: {i.branches.join(", ")}
        {#if i.taiga && i.branches.includes(i.gatilho ?? "")}<span class="badge info">gera relatório e US</span>{/if}
      </p>
    {/each}
    <p class="det">Mensagem: {mensagem.trim() || "(sem mensagem)"}</p>
    <p class="det">{push ? "Depois do commit, as branches serão enviadas (push)." : "Sem push: os commits ficam só nesta máquina."}</p>
    <p class="det">Nada é reescrito: só fast-forward, commit e cherry-pick. Em conflito a branch fica como estava.</p>
  </Modal>
{/if}

<style>
  .mini {
    flex: none;
    padding: 3px 9px;
    font-size: 0.78rem;
  }
  .ramos {
    display: grid;
    gap: 6px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--borda);
  }
  .faixa {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    padding: 8px 10px;
    border: 1px solid var(--aviso);
    border-radius: 6px;
    font-size: 0.88rem;
  }
  .envio {
    margin-top: 20px;
    padding: 14px 16px;
    border: 1px dashed var(--borda);
    border-radius: 8px;
  }
</style>
