// Capítulo 1 (16:9): o problema e o Taiga Assist. Recebe a Cena e os tempos das falas.
import { COR } from "./lib.mjs";
const { navy, navy2, gold, cream, laranja, verde, vermelho, azul, branco } = COR;

export function cap1(c, T) {
  const S = T.inicio, E = i => S[i + 1] ?? T.fim;           // início/fim de cada fala
  const pal = (i, re) => { const p = T.palavras[i].find(p => re.test(p.w)); return S[i] + (p ? p.t : 0); };
  const W = c.W, H = c.H, bd = navy;
  // ── Cena 0: título + cartão do Taiga com relógio
  const t0 = S[0] - 0.2, t1 = E(0) + 0.2;
  c.texto("TAIGA ASSIST", 0, 0, { size: 52, cor: gold, caixa: [146, 150, 900, 80], t0, t1, borda: bd, bordaW: 8 });
  c.texto("CHEGA DE", 0, 0, { size: 150, cor: branco, caixa: [140, 235, 1000, 190], t0: t0 + 0.1, t1, borda: bd, bordaW: 14 });
  c.texto("RELATÓRIO", 0, 0, { size: 150, cor: branco, caixa: [140, 400, 1100, 190], t0: t0 + 0.2, t1, borda: bd, bordaW: 14 });
  c.texto("MANUAL", 0, 0, { size: 150, cor: gold, caixa: [140, 565, 1100, 190], t0: t0 + 0.3, t1, borda: bd, bordaW: 14 });
  const cx = 1470, cy = 470;
  c.forma("rounded", cx, cy, 560, 360, cream, { raio: 34, t0: t0 + 0.4, t1, borda: bd, bordaW: 6 });
  c.texto("#312  Relatório da semana", 0, 0, { size: 32, cor: navy, caixa: [cx - 250, cy - 150, 500, 50], t0: t0 + 0.5, t1 });
  c.forma("ellipse", cx - 120, cy + 30, 190, 190, "#ffffff", { raio: 0, t0: t0 + 0.5, t1, borda: navy, bordaW: 12 });
  c.forma("rounded", cx - 120, cy + 5, 10, 70, navy, { raio: 5, t0: t0 + 0.6, t1, anim: true });
  c.forma("rounded", cx - 95, cy + 30, 60, 10, navy, { raio: 5, t0: t0 + 0.6, t1 });
  c.texto("2 h", cx + 120, cy + 30, { size: 120, cor: laranja, t0: t0 + 0.7, t1 });
  // ── Cena 1..: NTI + pílulas + barras de tempo
  const a = E(0) - 0.1, bTop = 300;
  c.texto("NTI Sistemas", W / 2, 160, { size: 110, cor: branco, borda: "#000000", bordaW: 14, t0: S[1] - 0.1, t1: E(5) + 0.2 });
  const px = [480, 960, 1440], nomes = ["1  PROGRAMAR", "2  DEPLOY", "3  RELATÓRIO"];
  const tp = [pal(1, /^programar/i), pal(1, /^deploy/i), pal(1, /^relat/i)];
  const largura = [400, 220, 280];
  const pil = [], barras = [];
  for (let i = 0; i < 3; i++) {
    pil[i] = c.forma("rounded", px[i], bTop, 420, 110, navy2, { raio: 55, t0: tp[i] - 0.15, t1: E(5) + 0.2, borda: "#3b4660", bordaW: 4 });
    c.texto(nomes[i], px[i], bTop, { size: 42, cor: branco, t0: tp[i] - 0.1, t1: E(5) + 0.2 });
  }
  // barras de tempo (esquerda fixa): largura animada por expressão
  const bx0 = [px[0] - 210, px[1] - 210, px[2] - 210], by = 395;
  for (let i = 0; i < 3; i++) {
    const n = c.nome("barra"); barras[i] = n;
    c.c("layer.newShape", { kind: "rounded", name: n, size: [10, 36], fill: i === 0 ? gold : "#6b7488", position: [bx0[i], by] });
    c.c("prop.set", { layer: n, path: "contents/group/contents/rect/roundness", value: 18 });
    // programar encolhe na fala 3; relatório mantém; deploy encolhe até 0 quando automatizado (fala 4)
    const L = largura[i];
    let w = `ease(time,${tp[i]},${tp[i] + 0.7},0,${L})`;
    if (i === 0) w = `${w}*(1-0.6*ease(time,${S[3] + 0.8},${S[3] + 2.2},0,1))`;
    if (i === 1) w = `${w}*(1-ease(time,${S[4] + 0.4},${S[4] + 1.6},0,1))`;
    c.expr(n, "contents/group/contents/rect/size", `w=Math.max(${w},1);[w,36]`);
    c.expr(n, "contents/group/contents/rect/position", `w=Math.max(${w},1);[w/2,0]`);
    c.expr(n, "transform/opacity", `(${c.vis(tp[i], E(5) + 0.2)})*((${w})>6?1:0)`);
    c.expr(n, "transform/anchor", `[${0},0,0]`);
  }
  // legenda das barras
  // destaque "programar" (fala 2): pílula 1 fica laranja
  c.corPorTempo(pil[0], "contents/group/contents/fill/color", S[2], navy2, laranja);
  // ── fala 3: terminal com código digitado
  const tt0 = S[3] - 0.1, tt1 = E(3) + 0.1;
  const tx = W / 2, ty = 650;
  c.forma("rounded", tx, ty, 1000, 270, "#0e1420", { raio: 26, t0: tt0, t1: tt1, borda: "#3b4660", bordaW: 4 });
  [["#ff5f56", -440], ["#ffbd2e", -405], ["#27c93f", -370]].forEach(([k, dx]) => c.forma("ellipse", tx + dx, ty - 105, 20, 20, k, { raio: 0, t0: tt0, t1: tt1 }));
  const cod = "$ kiro \"gere o relatório\"\n> const us = await taiga.criar(rel);\n> us.pontos = { front: 3, back: 5 };\n> await git.push(branch);\n✓ pronto em 4 s";
  c.digitar(cod, [tx - 470, ty - 80, 940, 200], { t0: tt0 + 0.3, t1: tt1, size: 32, fonte: "JetBrains Mono", estilo: "Regular", cor: "#9ad1ff", leading: 42 });
  // ── fala 4: deploy ✓ verde
  const dv = S[4] + 0.2;
  const ver = c.forma("rounded", px[1], bTop, 420, 110, verde, { raio: 55, t0: dv, t1: E(5) + 0.2, borda: bd, bordaW: 4, de: 96 });
  c.texto("✓  DEPLOY", px[1], bTop, { size: 42, cor: branco, t0: dv + 0.05, t1: E(5) + 0.2, fonte: "DejaVu Sans", estilo: "Bold" });
  // ── fala 5: relatório vermelho, pilha de cartões, "até 2 h"
  const rv = pal(5, /^relat/i) - 0.2;
  c.corPorTempo(pil[2], "contents/group/contents/fill/color", rv, navy2, vermelho);
  for (let k = 0; k < 4; k++) {
    c.forma("rounded", 760 + k * 40, 610 - k * 18, 420, 190, k % 2 ? "#efe6d3" : cream, { raio: 24, t0: rv + 0.1 + k * 0.12, t1: S[6] - 0.1, borda: bd, bordaW: 5 });
    c.forma("rounded", 640 + k * 40, 600 - k * 18, 170, 20, "#c9bfa7", { raio: 10, t0: rv + 0.15 + k * 0.12, t1: S[6] - 0.1 });
  }
  c.texto("ATÉ 2 H", 1360, 580, { size: 120, cor: gold, borda: bd, bordaW: 14, t0: pal(5, /^at/i) - 0.1, t1: S[6] - 0.1 });
  c.texto("POR SEMANA", 1360, 690, { size: 70, cor: branco, borda: bd, bordaW: 10, t0: pal(5, /^at/i) + 0.1, t1: S[6] - 0.1 });
  // ── fala 6: Taiga Assist
  const g = S[6] - 0.1;
  c.forma("rounded", 560, 190, 150, 150, azul, { raio: 38, t0: g, borda: bd, bordaW: 6 });
  c.texto("T", 560, 192, { size: 120, cor: branco, t0: g + 0.05 });
  c.texto("Taiga Assist", 0, 0, { size: 130, cor: branco, caixa: [660, 120, 1100, 150], t0: g + 0.15, borda: bd, bordaW: 14 });
  const wx = W / 2, wy = 590;
  c.forma("rounded", wx, wy, 1100, 400, cream, { raio: 30, t0: g + 0.4, borda: bd, bordaW: 6 });
  c.forma("rounded", wx - 410, wy, 240, 360, "#e6dcc6", { raio: 22, t0: g + 0.5 });
  [["#ff5f56", -520], ["#ffbd2e", -485], ["#27c93f", -450]].forEach(([k, dx]) => c.forma("ellipse", wx + dx, wy - 170, 20, 20, k, { raio: 0, t0: g + 0.5 }));
  ["Pasta local", "Taiga US's", "Prestar conta"].forEach((m, i) => c.texto(m, wx - 410, wy - 70 + i * 66, { size: 32, cor: i === 0 ? azul : navy, t0: g + 0.6 + i * 0.1 }));
  for (let k = 0; k < 3; k++) c.forma("rounded", wx + 150, wy - 90 + k * 95, 640, 76, "#ffffff", { raio: 16, t0: g + 0.7 + k * 0.12, borda: "#d8cdb4", bordaW: 3 });
}
