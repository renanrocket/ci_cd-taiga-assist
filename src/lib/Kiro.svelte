<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import { tarefas } from "$lib/tasks.svelte";
  import { capacidades } from "$lib/capacidades.svelte";

  let {
    onNext,
    onBack,
    rotuloProximo = "Continuar",
    rotuloPular = "Pular",
    embutido = false,
  }: {
    onNext?: () => void;
    onBack?: () => void;
    rotuloProximo?: string;
    rotuloPular?: string;
    embutido?: boolean;
  } = $props();

  type Status = {
    login: {
      logged_in: boolean;
      who: { account_type: string; email: string | null; region: string | null } | null;
      detail: string;
    };
    api_key: boolean;
  };
  type Teste = { ok: boolean; detail: string; carregando?: boolean };

  let status = $state<Status | null>(null);
  let teste = $state<Teste | null>(null);
  let chave = $state("");
  let erro = $state("");
  let ocupado = $state(false);

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

  const verificar = () =>
    guard(async () => {
      status = await invoke<Status>("kiro_status");
      teste = null;
    });

  const salvarChave = () =>
    guard(async () => {
      status = await invoke<Status>("save_kiro_api_key", { key: chave });
      chave = "";
      teste = null;
    });

  const removerChave = () =>
    guard(async () => {
      status = await invoke<Status>("clear_kiro_api_key");
      teste = null;
    });

  async function testar() {
    teste = { ok: false, detail: "Testando… (pode levar alguns segundos)", carregando: true };
    try {
      teste = await invoke<Teste>("test_kiro");
    } catch (e) {
      teste = { ok: false, detail: String(e) };
    }
  }

  // Login pelo app (código de dispositivo): o kiro-cli roda em segundo plano e suas linhas viram etapas.
  let tipoConta = $state<"free" | "pro">("free");
  let idp = $state("");
  let regiao = $state("us-east-1");
  const loginTarefa = $derived(tarefas.lista.find((t) => t.key === "kiro-login"));
  const loginRodando = $derived(loginTarefa?.estado === "rodando");
  const linkLogin = $derived.by(() => {
    for (const l of [...(loginTarefa?.passos ?? [])].reverse()) {
      const m = l.match(/https?:\/\/[^\s"')>]+/);
      if (m) return m[0];
    }
    return null;
  });
  const linhasLogin = $derived((loginTarefa?.passos ?? []).slice(-6).map((l) => l.replace(/^\d\d:\d\d:\d\d /, "")));

  async function entrar() {
    erro = "";
    try {
      await invoke("kiro_login_start", {
        license: tipoConta,
        idp: tipoConta === "pro" ? idp : null,
        region: tipoConta === "pro" ? regiao : null,
      });
    } catch (e) {
      erro = String(e);
    }
  }

  // Quando uma tarefa termina (o login), confere de novo o estado.
  let vistas = "";
  $effect(() => {
    const v = tarefas.versaoTerminadas;
    if (v !== vistas) {
      const primeira = vistas === "";
      vistas = v;
      if (!primeira) verificar();
    }
  });

  const pronto = $derived(!!status && (status.login.logged_in || status.api_key));
  $effect(() => {
    if (status) capacidades.definirKiroPronto(pronto);
  });

  onMount(verificar);
</script>

<p class="det" style="margin-top: 0">
  O Kiro gera o relatório de cada push e o resumo das US. Você precisa estar logado no Kiro CLI desta máquina
  (ou informar uma API key).
</p>

{#if erro}<p class="erro" role="alert">{erro}</p>{/if}

<h2>Login</h2>
{#if status}
  <div class="item" class:ok={status.login.logged_in} class:falha={!status.login.logged_in}>
    <div class="cab">
      <strong>Kiro CLI</strong>
      <span class="selo">{status.login.logged_in ? "logado" : "não logado"}</span>
    </div>
    {#if status.login.logged_in && status.login.who}
      <p class="det">
        {status.login.who.email ?? "conta sem e-mail"} · {status.login.who.account_type}{status.login.who.region
          ? ` · ${status.login.who.region}`
          : ""}
      </p>
    {:else}
      <p class="det">{status.login.detail || "Não está logado no Kiro."}</p>
      {#if !loginRodando}
        <form
          class="campo form-login"
          onsubmit={(e) => {
            e.preventDefault();
            entrar();
          }}
        >
          <label for="kiro-tipo">Tipo de conta</label>
          <select id="kiro-tipo" bind:value={tipoConta}>
            <option value="free">Pessoal (Builder ID, Google ou GitHub)</option>
            <option value="pro">Empresa (IAM Identity Center)</option>
          </select>
          {#if tipoConta === "pro"}
            <label for="kiro-idp">Endereço do portal da empresa</label>
            <input id="kiro-idp" bind:value={idp} placeholder="https://sua-empresa.awsapps.com/start" />
            <label for="kiro-regiao">Região</label>
            <input id="kiro-regiao" bind:value={regiao} />
          {/if}
          <div class="linha" style="justify-content: flex-end">
            <button type="submit" class="primario" disabled={tipoConta === "pro" && !idp.trim()}>Entrar no Kiro</button>
          </div>
        </form>
      {:else}
        <p class="det">Complete o login no navegador: abra a página, entre e confirme o código mostrado abaixo.</p>
        {#if linkLogin}
          <div class="linha" style="margin-top: 8px">
            <button type="button" class="primario" onclick={() => openUrl(linkLogin)}>Abrir página de login</button>
          </div>
        {/if}
        <pre class="cmd"><code>{linhasLogin.join("\n")}</code></pre>
      {/if}
      <details style="margin-top: 10px">
        <summary class="det">Prefere usar o terminal?</summary>
        <pre class="cmd"><code># conta pessoal (Builder ID / Google / GitHub)
kiro-cli login --license free

# conta corporativa (IAM Identity Center)
kiro-cli login --license pro --identity-provider https://SUA-EMPRESA.awsapps.com/start --region us-east-1</code></pre>
      </details>
    {/if}
    <div class="linha" style="margin-top: 10px; justify-content: flex-end">
      <button type="button" onclick={verificar} disabled={ocupado}>Verificar login</button>
    </div>
  </div>
{:else}
  <p class="vazio">Verificando…</p>
{/if}

<h2>API key (opcional)</h2>
<div class="item" class:ok={status?.api_key}>
  <div class="cab">
    <strong>KIRO_API_KEY</strong>
    <span class="selo">{status?.api_key ? "configurada" : "não configurada"}</span>
  </div>
  <p class="det">
    Alternativa ao login pelo navegador, disponível nos planos Pro, Pro+ e Power. Fica no chaveiro do sistema e é
    enviada ao Kiro por variável de ambiente.
  </p>
  <form
    class="campo"
    style="margin-top: 10px"
    onsubmit={(e) => {
      e.preventDefault();
      salvarChave();
    }}
  >
    <label for="kiro-key">API key</label>
    <div class="linha">
      <input id="kiro-key" type="password" bind:value={chave} autocomplete="new-password" disabled={ocupado} />
      <button type="submit" class="primario" disabled={ocupado || !chave.trim()}>Salvar</button>
      {#if status?.api_key}
        <button type="button" class="perigo" onclick={removerChave} disabled={ocupado}>Remover</button>
      {/if}
    </div>
  </form>
</div>

<h2>Teste</h2>
<div class="item" class:ok={teste?.ok} class:falha={teste && !teste.ok && !teste.carregando}>
  <div class="cab">
    <strong>Execução de ponta a ponta</strong>
    {#if teste}<span class="selo">{teste.ok ? "OK" : teste.carregando ? "testando…" : "falhou"}</span>{/if}
  </div>
  <p class="det">Envia um pedido mínimo ao Kiro, do jeito que o app fará nos relatórios (gasta cerca de 0,05 crédito).</p>
  {#if teste && !teste.carregando}<p class="det" class:erro={!teste.ok}>{teste.detail}</p>{/if}
  <div class="linha" style="margin-top: 10px; justify-content: flex-end">
    <button type="button" onclick={testar} disabled={ocupado || !pronto || teste?.carregando}>Testar o Kiro</button>
  </div>
</div>

{#if !embutido}
  <div class="acoes">
    <button type="button" onclick={onBack}>Voltar</button>
    <button type="button" class:primario={pronto} onclick={onNext}>{pronto ? rotuloProximo : rotuloPular}</button>
  </div>
{/if}

<style>
  h2 {
    font-size: 1rem;
    margin: 24px 0 8px;
  }
  .form-login {
    display: grid;
    gap: 6px;
    margin-top: 10px;
  }
  .cmd {
    background: var(--bg);
    border: 1px solid var(--borda);
    border-radius: 6px;
    padding: 10px 12px;
    overflow-x: auto;
    font-size: 0.82rem;
    margin: 8px 0 0;
    user-select: all;
  }
</style>
