// O que o app consegue fazer agora, conforme o que o usuário configurou.
//   base   : git + pasta  -> commit e push em lote (único requisito de verdade)
//   kiro   : kiro-cli achado e logado (ou com API key)
//   taiga  : pelo menos uma conta cadastrada; EXIGE o Kiro (relatórios e resumos são gerados por ele)
// As telas leem este estado para habilitar ou desabilitar controles, sempre dizendo o que falta.
import { invoke } from "@tauri-apps/api/core";

type Ferramenta = { tool: string; found: boolean };
type KiroStatus = { login: { logged_in: boolean }; api_key: boolean };

class Capacidades {
  carregado = $state(false);
  git = $state(false);
  kiroEncontrado = $state(false);
  kiroPronto = $state(false);
  contasTaiga = $state(0);

  /** Taiga com conta cadastrada E Kiro pronto: libera relatório + US e a prestação de contas. */
  get taigaPronto() {
    return this.kiroPronto && this.contasTaiga > 0;
  }

  private emAndamento: Promise<void> | null = null;

  /** Relê tudo do backend (as telas de configuração também atualizam os campos direto, sem custo). */
  atualizar(): Promise<void> {
    this.emAndamento ??= this.ler().finally(() => (this.emAndamento = null));
    return this.emAndamento;
  }

  private async ler() {
    try {
      const tools = await invoke<Ferramenta[]>("check_prerequisites");
      this.definirFerramentas(tools);
      if (this.kiroEncontrado) {
        const k = await invoke<KiroStatus>("kiro_status");
        this.kiroPronto = k.login.logged_in || k.api_key;
      } else {
        this.kiroPronto = false;
      }
    } catch {
      /* sem leitura, mantém o último estado conhecido */
    }
    try {
      this.contasTaiga = (await invoke<unknown[]>("list_taiga_accounts")).length;
    } catch {
      /* idem */
    }
    this.carregado = true;
  }

  definirFerramentas(tools: Ferramenta[]) {
    this.git = tools.some((t) => t.tool === "git" && t.found);
    this.kiroEncontrado = tools.some((t) => t.tool === "kiro" && t.found);
    if (!this.kiroEncontrado) this.kiroPronto = false;
    this.carregado = true;
  }

  definirKiroPronto(pronto: boolean) {
    this.kiroPronto = pronto;
    this.carregado = true;
  }

  definirContasTaiga(qtd: number) {
    this.contasTaiga = qtd;
    this.carregado = true;
  }
}

export const capacidades = new Capacidades();
