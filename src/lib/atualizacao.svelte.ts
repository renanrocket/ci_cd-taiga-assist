// Atualização do aplicativo: verifica no GitHub Releases, avisa e, se o usuário quiser, baixa e instala.
//
// - Windows/macOS e Linux em AppImage: modo "auto" (baixa, confere a assinatura, instala e reinicia).
// - Linux instalado por .deb: modo "manual" (só avisa e leva à página de download).
// O usuário sempre escolhe: "Atualizar agora" ou "Agora não" (o aviso volta na próxima abertura).
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export const PAGINA_DOWNLOADS = "https://github.com/renanrocket/ci_cd-taiga-assist/releases/latest";

const A_CADA_MS = 6 * 60 * 60 * 1000; // reverifica enquanto o app fica aberto
const ATRASO_INICIAL_MS = 4000; // não disputa com a abertura do app

type Estado = "ocioso" | "verificando" | "baixando" | "reiniciando" | "erro";

class Atualizacao {
  versaoAtual = $state("");
  modo = $state<"auto" | "manual">("auto");
  estado = $state<Estado>("ocioso");
  /** Nova versão encontrada (null = nenhuma). */
  disponivel = $state<{ versao: string; notas: string } | null>(null);
  /** Usuário escolheu "Agora não" nesta sessão. */
  adiada = $state(false);
  /** 0–100 durante o download (null = tamanho desconhecido). */
  progresso = $state<number | null>(0);
  erro = $state("");
  /** Resultado da última verificação manual, para mostrar em Configurações. */
  resultadoManual = $state("");

  private update: Update | null = null;
  private iniciado = false;

  async iniciar() {
    if (this.iniciado) return;
    this.iniciado = true;
    try {
      this.versaoAtual = await getVersion();
      this.modo = (await invoke<string>("update_mode")) === "manual" ? "manual" : "auto";
    } catch {
      /* sem a versão o aviso ainda funciona */
    }
    setTimeout(() => this.verificar(false), ATRASO_INICIAL_MS);
    setInterval(() => this.verificar(false), A_CADA_MS);
  }

  /** `manual = true` quando o usuário clicou em "Verificar atualizações" (mostra o resultado). */
  async verificar(manual: boolean) {
    if (this.estado === "verificando" || this.estado === "baixando" || this.estado === "reiniciando") return;
    this.estado = "verificando";
    this.resultadoManual = "";
    try {
      const u = await check();
      if (u) {
        this.update = u;
        this.disponivel = { versao: u.version, notas: (u.body ?? "").trim() };
        if (manual) this.adiada = false;
        this.resultadoManual = `Nova versão ${u.version} disponível.`;
      } else {
        this.update = null;
        this.disponivel = null;
        this.resultadoManual = `Você já está na versão mais recente${this.versaoAtual ? ` (${this.versaoAtual})` : ""}.`;
      }
      this.erro = "";
    } catch (e) {
      // Verificação automática falha em silêncio (sem internet, ainda sem versão publicada…).
      if (manual) this.resultadoManual = "Não foi possível verificar agora. Confira a internet e tente de novo.";
      console.warn("verificação de atualização falhou:", e);
    } finally {
      this.estado = "ocioso";
    }
  }

  adiar() {
    this.adiada = true;
  }

  async instalar() {
    if (!this.update) return;
    this.erro = "";
    this.estado = "baixando";
    this.progresso = 0;
    let total = 0;
    let baixado = 0;
    try {
      await this.update.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        else if (e.event === "Progress") {
          baixado += e.data.chunkLength;
          this.progresso = total > 0 ? Math.min(100, Math.round((baixado / total) * 100)) : null;
        } else if (e.event === "Finished") this.progresso = 100;
      });
      this.estado = "reiniciando";
      await relaunch();
    } catch (e) {
      this.estado = "erro";
      this.erro = `Não consegui instalar a atualização: ${String(e)}`;
    }
  }
}

export const atualizacao = new Atualizacao();
