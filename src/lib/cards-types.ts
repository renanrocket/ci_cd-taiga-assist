// Tipos compartilhados das telas de Taiga cards (espelham as structs de src-tauri/src/cards.rs).
export type PontoPapel = { papel: string; pontos: string };

export type Card = {
  chave: string;
  report_id: number | null;
  project_id: number | null;
  us_id: number;
  ref: number | null;
  titulo: string;
  descricao: string;
  resumo: string;
  projeto: string;
  status: string | null;
  status_cor: string | null;
  fechado: boolean;
  prazo: string | null;
  dias: number | null;
  prazo_estado: "fechado" | "sem_prazo" | "atrasado" | "hoje" | "proximo" | "ok";
  url: string | null;
  estado: "ok" | "erro" | "removido";
  erro: string | null;
  status_final: string | null;
  selecionavel: boolean;
  criado_taiga: string | null;
  descricao_carregada: boolean;
  resumo_pronto: boolean;
  externo: boolean;
  criado_por: string | null;
  pontos_papeis: PontoPapel[];
  pontos_total: number | null;
  repo: string;
  branch: string;
};

export type CardRef = { report_id: number | null; project_id: number | null; us_id: number };
export type ProjetoOpt = { id: number; nome: string; conta: string };
export type CardsPage = { cards: Card[]; page: number; pages: number; total: number; projetos: ProjetoOpt[] };

export type Prepared = { id: number; ref: number | null; projeto: string; titulo: string; ok: boolean; detail: string };
export type CloseResult = { id: number | null; ref: number | null; titulo: string; projeto: string; ok: boolean; detail: string };

/** 2026-11-24 → 24/11/2026 */
export function fmtData(iso: string | null | undefined): string {
  const m = (iso ?? "").match(/^(\d{4})-(\d{2})-(\d{2})/);
  return m ? `${m[3]}/${m[2]}/${m[1]}` : "";
}
