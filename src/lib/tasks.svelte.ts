// Tarefas em segundo plano (commit em lote, reenvio): estado compartilhado + eventos do backend.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Tarefa = {
  id: string;
  key: string;
  titulo: string;
  estado: "rodando" | "ok" | "erro";
  mensagem: string;
  passos: string[];
  resultado: unknown;
  iniciado_em: string;
  terminado_em: string | null;
};

class Tarefas {
  lista = $state<Tarefa[]>([]);
  private iniciado = false;

  async iniciar() {
    if (this.iniciado) return;
    this.iniciado = true;
    try {
      this.lista = await invoke<Tarefa[]>("list_tasks");
      await listen<Tarefa>("task-update", (e) => this.aplicar(e.payload));
    } catch {
      this.iniciado = false;
    }
  }

  private aplicar(t: Tarefa) {
    const i = this.lista.findIndex((x) => x.id === t.id);
    if (i >= 0) this.lista[i] = t;
    else this.lista.unshift(t);
  }

  async dispensar(id: string) {
    await invoke("dismiss_task", { id });
    this.lista = this.lista.filter((t) => t.id !== id);
  }

  /** Muda quando alguma tarefa termina (para telas recarregarem seus dados). */
  get versaoTerminadas(): string {
    return this.lista
      .filter((t) => t.estado !== "rodando")
      .map((t) => t.id)
      .join(",");
  }
}

export const tarefas = new Tarefas();
