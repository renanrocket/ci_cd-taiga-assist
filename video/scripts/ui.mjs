// Componentes de UI do vídeo (mocks vetoriais): janela do app, caixas, botões, Kanban fictício.
import { COR } from "./lib.mjs";
const { navy, navy2, gold, cream, laranja, verde, vermelho, azul, branco } = COR;
const bd = navy;

export function tituloCap(c, rotulo, titulo, t0, t1) {
  c.texto(rotulo, 0, 0, { size: 38, cor: gold, caixa: [146, 56, 900, 60], t0, t1, borda: bd, bordaW: 8 });
  c.texto(titulo, 0, 0, { size: 76, cor: branco, caixa: [140, 100, 1400, 100], t0: t0 + 0.1, t1, borda: bd, bordaW: 12 });
}
export const JAN = { cx: 960, cy: 500, w: 1240, h: 540 };
// janela do app Taiga Assist: moldura, 3 botões, barra lateral com a aba ativa
export function appJanela(c, t0, t1, aba, { f = 0.3 } = {}) {
  const { cx, cy, w, h } = JAN;
  c.forma("rounded", cx, cy, w, h, cream, { raio: 30, t0, t1, f, borda: bd, bordaW: 6 });
  [["#ff5f56", -1], ["#ffbd2e", 0], ["#27c93f", 1]].forEach(([k, i]) => c.forma("ellipse", cx - w / 2 + 40 + (i + 1) * 34, cy - h / 2 + 32, 20, 20, k, { t0, t1, f }));
  c.forma("rounded", cx - w / 2 + 160, cy + 26, 260, h - 100, "#e6dcc6", { raio: 22, t0, t1, f });
  ["Pasta local", "Taiga US's", "Prestar conta", "Configurações"].forEach((m, i) =>
    c.texto(m, cx - w / 2 + 160, cy - 100 + i * 70, { size: 30, cor: m.startsWith(aba) ? azul : navy, t0, t1, f, estilo: m.startsWith(aba) ? "Bold" : "Medium" }));
  return { x0: cx - w / 2 + 320, y0: cy - h / 2 + 80, x1: cx + w / 2 - 40, y1: cy + h / 2 - 40 };
}
export function caixa(c, x, y, rotulo, t0, t1, marcaEm, { size = 30, cor = navy } = {}) {
  c.forma("rounded", x, y, 36, 36, "#ffffff", { raio: 8, t0, t1, borda: "#8a8f9c", bordaW: 3 });
  if (marcaEm !== undefined) { c.forma("rounded", x, y, 36, 36, azul, { raio: 8, t0: marcaEm, t1, f: 0.15, de: 70 });
    c.texto("✓", x, y, { size: 28, cor: branco, fonte: "DejaVu Sans", t0: marcaEm, t1, f: 0.15 }); }
  c.texto(rotulo, 0, 0, { size, cor, caixa: [x + 36, y - 22, 700, 50], t0, t1, estilo: "Medium" });
}
export function botao(c, cx, cy, w, h, txt, t0, t1, { cor = azul, destaque, size = 30, corTxt = branco } = {}) {
  const n = c.forma("rounded", cx, cy, w, h, cor, { raio: h / 2, t0, t1, borda: bd, bordaW: 4 });
  c.texto(txt, cx, cy, { size, cor: corTxt, t0: t0 + 0.05, t1 });
  if (destaque !== undefined) c.forma("rounded", cx, cy, w + 24, h + 24, null, { raio: (h + 24) / 2, t0: destaque, t1, borda: gold, bordaW: 6, f: 0.2, de: 105 });
  return n;
}
export function pilula(c, cx, cy, w, txt, t0, t1, { cor = navy2, borda = "#3b4660", size = 36 } = {}) {
  const n = c.forma("rounded", cx, cy, w, 96, cor, { raio: 48, t0, t1, borda, bordaW: 4 });
  c.texto(txt, cx, cy, { size, cor: branco, t0: t0 + 0.05, t1 }); return n;
}
export function campo(c, x, y, w, txt, t0, t1, { size = 28, rotulo } = {}) {
  if (rotulo) c.texto(rotulo, 0, 0, { size: 24, cor: "#5b6272", caixa: [x, y - 58, w, 40], t0, t1, estilo: "Medium" });
  c.forma("rounded", x + w / 2, y, w, 58, "#ffffff", { raio: 14, t0, t1, borda: "#cfc4aa", bordaW: 3 });
  c.texto(txt, 0, 0, { size, cor: navy, caixa: [x + 18, y - 20, w - 30, 50], t0: t0 + 0.05, t1 });
}
// Kanban fictício: 4 colunas; retorna centros das colunas
export const COLS = ["Novo", "Em andamento", "Pronto", "Fechado"];
export function kanban(c, cx, cy, w, h, t0, t1, { cartoes = [], f = 0.3 } = {}) {
  const gap = 20, cw = (w - gap * 5) / 4, xs = [];
  c.forma("rounded", cx, cy, w, h, "#e9e1cf", { raio: 28, t0, t1, f, borda: bd, bordaW: 6 });
  COLS.forEach((nome, i) => {
    const x = cx - w / 2 + gap + cw / 2 + i * (cw + gap); xs.push(x);
    c.forma("rounded", x, cy + 28, cw, h - 100, "#d8cdb4", { raio: 18, t0, t1, f });
    c.texto(nome, x, cy - h / 2 + 52, { size: 28, cor: navy, t0, t1, f });
  });
  cartoes.forEach(([col, linha, num, titulo]) => cartao(c, xs[col], cy - h / 2 + 190 + linha * 170, cw - 30, num, titulo, t0, t1));
  return { xs, cw, topo: cy - h / 2 + 190 };
}
// cartão do Kanban: devolve nomes das camadas (para mover)
export function cartao(c, x, y, w, num, titulo, t0, t1, { realce } = {}) {
  // cartão de 150 px: título em até 2 linhas ("\n"), selos na base
  const nomes = [c.forma("rounded", x, y, w, 150, "#ffffff", { raio: 14, t0, t1, borda: realce || "#b9ae95", bordaW: realce ? 6 : 3 })];
  nomes.push(c.texto(`#${num}`, x, y - 54, { size: 22, cor: "#6b7488", t0, t1, estilo: "Medium" }));
  nomes.push(c.texto(titulo, x, y - 8, { size: 24, cor: navy, t0, t1, leading: 28 }));
  nomes.push(c.forma("rounded", x - w / 2 + 52, y + 58, 80, 18, verde, { raio: 9, t0, t1 }));
  nomes.push(c.forma("rounded", x - w / 2 + 140, y + 58, 80, 18, azul, { raio: 9, t0, t1 }));
  return nomes;
}
export function notificacao(c, cx, cy, w, h, t0, t1) {
  c.forma("rounded", cx, cy, w, h, navy2, { raio: 26, t0, t1, borda: gold, bordaW: 5 });
}
