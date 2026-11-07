// Capítulos 2 a 7 (16:9). Cada função recebe a Cena e os tempos (T.inicio por fala).
import { COR, } from "./lib.mjs";
import { tituloCap, appJanela, JAN, caixa, botao, pilula, campo, kanban, cartao, notificacao } from "./ui.mjs";
const { navy, navy2, gold, cream, laranja, verde, vermelho, azul, branco } = COR;
const bd = navy;
const ctx = (c, T) => { const S = T.inicio, E = i => S[i + 1] ?? T.fim;
  const pal = (i, re) => { const p = T.palavras[i].find(p => re.test(p.w)); return S[i] + (p ? p.t : 0); };
  return { S, E, pal, W: c.W, H: c.H }; };
const fim = T => T.fim + 1;

export function cap2(c, T) {
  const { S, E, pal } = ctx(c, T);
  tituloCap(c, "PASSO 1", "INSTALAR", 0.3, fim(T));
  // f0: três ícones + Baixar
  const sis = [["Windows", "⊞", 560], ["macOS", "", 960], ["Linux", "🐧", 1360]];
  const nomesSis = ["Windows", "macOS", "Linux"];
  sis.forEach(([n, g, x], i) => {
    const t = pal(0, new RegExp("^" + n, "i")) - 0.2;
    c.forma("rounded", x, 400, 250, 250, cream, { raio: 44, t0: Math.max(t, S[0]), t1: E(0) + 0.2, borda: bd, bordaW: 6 });
    c.texto(n, x, 400, { size: 44, cor: navy, t0: Math.max(t, S[0]) + 0.05, t1: E(0) + 0.2 });
  });
  botao(c, 960, 600, 360, 100, "⬇  Baixar", pal(0, /^Baixe/i) + 0.2, E(0) + 0.2, { cor: laranja, size: 44 });
  // f1: aviso do sistema
  const a = S[1] - 0.1, b = E(1) + 0.2;
  c.forma("rounded", 960, 460, 980, 420, cream, { raio: 30, t0: a, t1: b, borda: bd, bordaW: 6 });
  c.forma("ellipse", 560, 380, 110, 110, "#f5b942", { t0: a, t1: b, borda: bd, bordaW: 5 });
  c.texto("!", 560, 380, { size: 80, cor: navy, t0: a, t1: b });
  c.texto("Não foi possível verificar o desenvolvedor", 0, 0, { size: 38, cor: navy, caixa: [650, 330, 740, 100], t0: a, t1: b });
  c.texto("\"Taiga Assist\" não é assinado.", 0, 0, { size: 30, cor: "#5b6272", caixa: [650, 420, 740, 60], t0: a, t1: b, estilo: "Medium" });
  botao(c, 780, 580, 300, 80, "Cancelar", a, b, { cor: "#d8cdb4", corTxt: navy });
  botao(c, 1150, 580, 400, 80, "Abrir mesmo assim", a, b, { cor: azul, destaque: pal(1, /^confirmar|^É$/i) });
  // f2: pré-requisitos
  const g = S[2] - 0.1;
  pilula(c, 700, 440, 480, "✓  GIT", g, fim(T), { cor: verde, borda: bd });
  pilula(c, 1220, 440, 520, "✓  KIRO CLI", pal(2, /^Kiro/i) - 0.2, fim(T), { cor: verde, borda: bd });
}

export function cap3(c, T) {
  const { S, E, pal } = ctx(c, T);
  tituloCap(c, "PASSO 2", "CONFIGURAR", 0.3, fim(T));
  const passos = ["1 Pré-requisitos", "2 Pasta", "3 Credencial", "4 Taiga", "5 Kiro"];
  // pílulas pequenas no topo direito; estado por tempo
  passos.forEach((p, i) => {
    const x = 330 + i * 315, y = 235, t0 = S[0] + i * 0.15;
    const n = c.forma("rounded", x, y, 295, 66, navy2, { raio: 33, t0, borda: "#3b4660", bordaW: 3 });
    const a = i === 0 ? S[1] : S[i + 1] - 0.05, b = S[i + 2] - 0.05 || 1e9;
    const hx = h => { const v = parseInt(h.slice(1), 16); return [(v >> 16 & 255) / 255, (v >> 8 & 255) / 255, (v & 255) / 255]; };
    const N = hx(navy2), O = hx(laranja), G = hx(verde);
    const comp = j => `${N[j]}+(${O[j]}-${N[j]})*k1+(${G[j]}-${O[j]})*k2*k1`;
    c.expr(n, "contents/group/contents/fill/color", `k1=ease(time,${a},${a + 0.3},0,1);k2=ease(time,${b},${b + 0.3},0,1);[${comp(0)},${comp(1)},${comp(2)},1]`);
    c.texto(p, x, y, { size: 26, cor: branco, t0: t0 + 0.05 });
  });
  // painel do app (um por passo)
  const { cx, cy, w, h } = { cx: 960, cy: 530, w: 1240, h: 470 };
  const jan = (t0, t1, tit) => { c.forma("rounded", cx, cy, w, h, cream, { raio: 30, t0, t1, borda: bd, bordaW: 6 });
    c.texto(tit, 0, 0, { size: 38, cor: azul, caixa: [cx - w / 2 + 50, cy - h / 2 + 30, 1000, 60], t0, t1 }); };
  const X = cx - w / 2 + 60, Y = cy - h / 2 + 110;
  let t0 = S[0] + 0.3, t1 = S[1] - 0.05;
  jan(t0, E(0) + 0.1, "Assistente de configuração");
  c.texto("Cinco passos rápidos", 0, 0, { size: 34, cor: navy, caixa: [X, Y, 900, 60], t0: t0 + 0.2, t1: E(0) + 0.1, estilo: "Medium" });
  // passo 1
  t0 = S[1] - 0.1; t1 = E(1) + 0.1; jan(t0, t1, "1 · Pré-requisitos");
  ["Git  — encontrado", "Kiro CLI  — encontrado"].forEach((r, i) => { caixa(c, X + 20, Y + 30 + i * 90, r, t0 + 0.2 + i * 0.2, t1, t0 + 0.7 + i * 0.4, { size: 36 }); });
  // passo 2
  t0 = S[2] - 0.1; t1 = E(2) + 0.1; jan(t0, t1, "2 · Pasta dos projetos");
  campo(c, X, Y + 50, 760, "~/Projetos", t0 + 0.2, t1, { rotulo: "Pasta" });
  ["portal-eventos", "api-agenda", "site-institucional"].forEach((r, i) => c.texto("▸ " + r, 0, 0, { size: 30, cor: navy, caixa: [X + 20, Y + 105 + i * 50, 700, 50], t0: pal(2, /^pasta/i) + i * 0.25, t1, estilo: "Medium" }));
  // passo 3
  t0 = S[3] - 0.1; t1 = E(3) + 0.1; jan(t0, t1, "3 · Acesso aos repositórios");
  ["Token de acesso", "Chave SSH", "Git do sistema"].forEach((r, i) => caixa(c, X + 20, Y + 20 + i * 70, r, t0 + 0.2 + i * 0.15, t1, i === 2 ? t0 + 1 : undefined, { size: 34 }));
  botao(c, X + 720, Y + 90, 220, 76, "Testar", t0 + 0.4, t1, { cor: azul });
  c.texto("✓ funcionou", X + 720, Y + 168, { size: 32, cor: verde, fonte: "DejaVu Sans", t0: pal(3, /^cadastre/i) + 1.2, t1 });
  // passo 4
  t0 = S[4] - 0.1; t1 = E(4) + 0.1; jan(t0, t1, "4 · Conta do Taiga");
  campo(c, X, Y + 50, 520, "ana.souza@exemplo.com", t0 + 0.2, t1, { rotulo: "Usuária" });
  caixa(c, X + 20, Y + 150, "Selecionar todos", pal(4, /^importe/i) - 0.3, t1, pal(4, /^importe/i) + 0.5, { size: 32 });
  ["Portal de Eventos", "Agenda Online"].forEach((r, i) => caixa(c, X + 20, Y + 210 + i * 56, r, pal(4, /^importe/i) - 0.2, t1, pal(4, /^importe/i) + 0.6 + i * 0.2, { size: 30 }));
  const lx = X + 620, lt = pal(4, /^ligue/i) - 0.2;
  campo(c, lx, Y + 50, 520, "portal-eventos", lt, t1, { rotulo: "Repositório" });
  campo(c, lx, Y + 150, 520, "Portal de Eventos", lt + 0.3, t1, { rotulo: "Projeto" });
  campo(c, lx, Y + 250, 520, "main", pal(4, /^branch/i) - 0.3, t1, { rotulo: "Branch" });
  // passo 5
  t0 = S[5] - 0.1; t1 = E(5) + 0.1; jan(t0, t1, "5 · Kiro");
  c.texto("Conectado como ana.souza@exemplo.com", 0, 0, { size: 36, cor: navy, caixa: [X, Y + 30, 1000, 60], t0: t0 + 0.2, t1 });
  c.texto("✓ logado", 0, 0, { size: 44, cor: verde, fonte: "DejaVu Sans", caixa: [X, Y + 110, 600, 70], t0: t0 + 1, t1 });
  // fala 6: tudo certo + menu Configurações
  t0 = S[6] - 0.1; jan(t0, fim(T), "Tudo pronto");
  ["Pré-requisitos", "Pasta", "Credencial", "Taiga", "Kiro"].forEach((r, i) => c.texto("✓  " + r, 0, 0, { size: 34, cor: verde, fonte: "DejaVu Sans", caixa: [X, Y + i * 52, 500, 50], t0: t0 + 0.2 + i * 0.12, t1: fim(T) }));
  c.forma("rounded", X + 760, Y + 120, 340, 80, azul, { raio: 40, t0: pal(6, /^Configura/i) - 0.5, t1: fim(T), borda: bd, bordaW: 4 });
  c.texto("Configurações", X + 760, Y + 120, { size: 34, cor: branco, t0: pal(6, /^Configura/i) - 0.45 });
  c.forma("rounded", X + 760, Y + 120, 380, 120, null, { raio: 60, t0: pal(6, /^Configura/i), t1: fim(T), borda: gold, bordaW: 6, de: 105 });
}

export function cap4(c, T) {
  const { S, E, pal } = ctx(c, T);
  tituloCap(c, "NO DIA A DIA", "COMMIT E PUSH", 0.3, fim(T));
  const A = appJanela(c, 0.4, S[3] - 0.15, "Pasta local");
  const X = A.x0, Y = A.y0;
  c.texto("portal-eventos", 0, 0, { size: 34, cor: azul, caixa: [X, Y - 8, 700, 50], t0: S[0] + 0.5, t1: S[3] - 0.15 });
  ["main", "feature/inscricao", "feature/agenda"].forEach((b, i) => {
    const tm = pal(1, /^Marque/i) + 0.5 + i * 0.3;
    caixa(c, X + 20, Y + 70 + i * 62, b, S[0] + 0.8 + i * 0.2, S[3] - 0.15, i > 0 ? tm : undefined, { size: 30 });
  });
  campo(c, X, Y + 290, 520, "", S[1] - 0.1, S[3] - 0.15, { rotulo: "Mensagem do commit" });
  c.digitar("feat: inscrição com confirmação", [X + 18, Y + 270, 500, 50], { t0: pal(1, /^mensagem/i), t1: S[3] - 0.15, size: 28, cor: navy, estilo: "Medium", cps: 22 });
  // opções + botão (fala 2)
  const ox = X + 400;
  caixa(c, ox, Y + 70, "Fazer push", S[2], S[3] - 0.15, S[2] + 0.4, { size: 30 });
  caixa(c, ox, Y + 130, "Criar US no Taiga", S[2] + 0.2, S[3] - 0.15, pal(2, /^Taiga/i) + 0.3, { size: 30 });
  botao(c, ox + 295, Y + 290, 300, 84, "Commitar", S[2], S[3] - 0.15, { cor: azul, destaque: pal(2, /^Commitar/i) - 0.2, size: 36 });
  // fala 3: notificação com etapas
  const n0 = S[3] - 0.1, n1 = S[4] - 0.1;
  notificacao(c, 960, 520, 900, 440, n0, n1);
  c.texto("Enviando…", 0, 0, { size: 40, cor: gold, caixa: [520, 330, 600, 60], t0: n0, t1: n1 });
  ["Commit feito", "Push enviado", "Kiro escrevendo o relatório", "US criada no Taiga"].forEach((r, i) => {
    const t = S[3] + 0.3 + i * (E(3) - S[3]) / 4.4;
    c.texto("✓  " + r, 0, 0, { size: 36, cor: i === 2 ? "#9ad1ff" : branco, fonte: "DejaVu Sans", caixa: [540, 400 + i * 62, 800, 60], t0: t, t1: n1 });
  });
  // fala 4: Taiga fictício com cartão novo
  const k0 = S[4] - 0.1;
  c.texto("Taiga · Portal de Eventos", 0, 0, { size: 34, cor: gold, caixa: [440, 205, 800, 50], t0: k0, t1: fim(T), borda: bd, bordaW: 8 });
  const K = kanban(c, 960, 520, 1300, 520, k0, fim(T), { cartoes: [[1, 0, 305, "Login social"], [2, 0, 298, "Filtro de eventos"], [3, 0, 290, "Layout mobile"]] });
  cartao(c, K.xs[0], K.topo, K.cw - 30, 312, "Inscrição com\nconfirmação", pal(4, /^hist/i) , fim(T), { realce: laranja });
  c.texto("novo!", K.xs[0], K.topo + 105, { size: 26, cor: laranja, t0: pal(4, /^hist/i) + 0.3, t1: fim(T) });
}

export function cap5(c, T) {
  const { S, E, pal } = ctx(c, T);
  tituloCap(c, "ENTREGOU?", "REVISAR E FECHAR", 0.3, fim(T));
  const t1a = S[3] - 0.15;
  const t1a2 = S[2] - 0.15;
  const A = appJanela(c, 0.4, t1a2, "Taiga US");
  const X = A.x0, Y = A.y0;
  [["312", "Inscrição com confirmação"], ["305", "Login social"], ["298", "Filtro de eventos"]].forEach(([n, t], i) => {
    const y = Y + 40 + i * 78, marca = i === 0 ? pal(1, /^Marque/i) + 0.4 : undefined;
    caixa(c, X + 20, y, `#${n}  ${t}`, S[0] + 0.4 + i * 0.15, t1a2, marca, { size: 30 });
  });
  botao(c, X + 728, Y + 40, 276, 76, "Revisar e fechar", S[1], t1a2, { cor: azul, destaque: pal(1, /^clique/i), size: 32 });
  const r0 = pal(1, /^Kiro/i) - 0.3;
  c.forma("rounded", X + 440, Y + 330, 800, 190, "#ffffff", { raio: 20, t0: r0, t1: t1a2, borda: "#cfc4aa", bordaW: 3 });
  c.texto("Feito", 0, 0, { size: 30, cor: verde, caixa: [X + 70, Y + 255, 400, 40], t0: r0 + 0.2, t1: t1a2 });
  c.texto("• Inscrição com e-mail de confirmação", 0, 0, { size: 26, cor: navy, caixa: [X + 70, Y + 295, 700, 40], t0: r0 + 0.5, t1: t1a2, estilo: "Medium" });
  c.texto("Pendências", 0, 0, { size: 30, cor: vermelho, caixa: [X + 70, Y + 345, 400, 40], t0: r0 + 0.9, t1: t1a2 });
  c.texto("• Texto do e-mail aguarda revisão", 0, 0, { size: 26, cor: navy, caixa: [X + 70, Y + 385, 700, 40], t0: r0 + 1.1, t1: t1a2, estilo: "Medium" });
  // fala 2: popup de confirmação
  const p0 = S[2] - 0.1, p1 = S[3] - 0.1;
  c.forma("rectangle" === "x" ? "rounded" : "rounded", 960, 520, 900, 380, cream, { raio: 30, t0: p0, t1: p1, borda: bd, bordaW: 6 });
  c.texto("Fechar US no Taiga", 960, 400, { size: 44, cor: navy, t0: p0, t1: p1 });
  c.texto("#312 · pontos: Front 3 · Back 5", 960, 480, { size: 32, cor: "#5b6272", t0: p0, t1: p1, estilo: "Medium" });
  botao(c, 780, 620, 260, 80, "Cancelar", p0, p1, { cor: "#d8cdb4", corTxt: navy });
  botao(c, 1140, 620, 340, 80, "Confirmar", p0, p1, { cor: verde, destaque: pal(2, /^confirme/i) - 0.3 });
  // fala 3: Kanban; cartão desliza para Fechado
  const k0 = S[3] - 0.1;
  const K = kanban(c, 960, 520, 1300, 520, k0, fim(T), { cartoes: [[0, 0, 305, "Login social"], [2, 0, 298, "Filtro de eventos"]] });
  const card = cartao(c, K.xs[1], K.topo, K.cw - 30, 312, "Inscrição com\nconfirmação", k0, fim(T), { realce: laranja });
  c.mover(card, K.xs[3] - K.xs[1], 0, pal(3, /^vai/i), pal(3, /^vai/i) + 1.4);
  c.texto("✓ fechada", K.xs[3], K.topo + 105, { size: 30, cor: verde, fonte: "DejaVu Sans", t0: pal(3, /^vai/i) + 1.4, t1: fim(T) });
}

export function cap6(c, T) {
  const { S, E, pal } = ctx(c, T);
  // f0: título
  const g0 = 0.3, g1 = S[1] - 0.1;
  c.texto("TODA SEMANA", 960, 440, { size: 54, cor: gold, borda: bd, bordaW: 10, t0: g0, t1: g1 });
  c.texto("DIA DA REUNIÃO", 960, 560, { size: 150, cor: branco, borda: bd, bordaW: 16, t0: g0 + 0.1, t1: g1 });
  // f1-2: app, aba Prestar conta
  const a0 = S[1] - 0.2, a1 = S[3] - 0.2, p0 = pal(2, /^O$|^app/i) + 0.1;
  tituloCap(c, "REUNIÃO", "PRESTAR CONTA", a0, fim(T));
  const A = appJanela(c, a0, p0, "Prestar");
  const X = A.x0, Y = A.y0;
  ["#312 Inscrição com confirmação", "#305 Login social", "#298 Filtro de eventos", "#290 Layout mobile"].forEach((r, i) =>
    caixa(c, X + 20, Y + 40 + i * 62, r, a0 + 0.3 + i * 0.12, p0, i < 3 ? pal(1, /^marque/i) + 0.3 + i * 0.4 : undefined, { size: 30 }));
  botao(c, X + 560, Y + 340, 480, 84, "Prestar conta e exportar…", S[2] - 0.1, p0, { cor: azul, destaque: pal(2, /^Clique/i), size: 30 });
  // popup pedindo link
  const p1 = a1;
  c.forma("rounded", 960, 520, 900, 340, cream, { raio: 30, t0: p0, t1: p1, borda: bd, bordaW: 6 });
  c.texto("Link da US da reunião", 960, 430, { size: 40, cor: navy, t0: p0, t1: p1 });
  campo(c, 560, 520, 800, "", p0, p1);
  // f3: Alt+Tab → navegador com Kanban das reuniões
  const b0 = S[3] - 0.1, b1 = S[4] - 0.1;
  c.forma("rounded", 960, 500, 1300, 560, cream, { raio: 28, t0: b0, t1: b1, borda: bd, bordaW: 6 });
  [["#ff5f56", 0], ["#ffbd2e", 1], ["#27c93f", 2]].forEach(([k, i]) => c.forma("ellipse", 340 + i * 34, 262, 20, 20, k, { t0: b0, t1: b1 }));
  c.forma("rounded", 1000, 262, 1000, 46, "#ffffff", { raio: 23, t0: b0, t1: b1, borda: "#cfc4aa", bordaW: 3 });
  c.texto("taiga.exemplo.com.br/project/reunioes/us/87", 0, 0, { size: 24, cor: "#5b6272", caixa: [520, 247, 960, 36], t0: b0, t1: b1, estilo: "Medium" });
  c.texto("Alt + Tab", 960, 200, { size: 40, cor: gold, borda: bd, bordaW: 8, t0: b0, t1: b0 + 1.6 });
  const xs = [460, 800, 1140, 1480];
  const cart = [[0, "Reunião semanal · 05/11"], [1, "Reunião semanal · 12/11 · Ana Souza"], [2, "Reunião semanal · 19/11"]];
  c.forma("rounded", 960, 540, 1240, 420, "#e9e1cf", { raio: 20, t0: b0 + 0.1, t1: b1 });
  cart.forEach(([i, t]) => { const alvo = i === 1; const x = [560, 960, 1360][i];
    c.forma("rounded", x, 480, 360, 150, "#ffffff", { raio: 16, t0: b0 + 0.3 + i * 0.15, t1: b1, borda: alvo ? laranja : "#b9ae95", bordaW: alvo ? 8 : 3 });
    c.texto(t, 0, 0, { size: 26, cor: navy, caixa: [x - 160, 440, 330, 100], t0: b0 + 0.35 + i * 0.15, t1: b1 }); });
  c.texto("copiar link ⧉", 960, 640, { size: 36, cor: azul, fonte: "DejaVu Sans", t0: pal(3, /^copie/i) - 0.5, t1: b1 });
  // f4: volta ao app, cola, exporta
  const c0 = S[4] - 0.1, c1 = S[5] - 0.1;
  appJanela(c, c0, c1, "Prestar");
  c.forma("rounded", 960, 520, 900, 340, cream, { raio: 30, t0: c0 + 0.1, t1: c1, borda: bd, bordaW: 6 });
  c.texto("Link da US da reunião", 960, 430, { size: 40, cor: navy, t0: c0 + 0.1, t1: c1 });
  campo(c, 560, 520, 800, "taiga.exemplo.com.br/project/reunioes/us/87", pal(4, /^cole/i) - 0.3, c1);
  botao(c, 960, 620, 300, 76, "Exportar", c0 + 0.1, c1, { cor: verde, destaque: pal(4, /^exporte/i) - 0.2 });
  // f5: US da reunião com descrição
  const d0 = S[5] - 0.1;
  c.forma("rounded", 960, 500, 1240, 540, cream, { raio: 30, t0: d0, t1: fim(T), borda: bd, bordaW: 6 });
  c.texto("Reunião semanal · 12/11 · Ana Souza", 0, 0, { size: 38, cor: azul, caixa: [380, 250, 1100, 60], t0: d0, t1: fim(T) });
  c.texto("Feito", 0, 0, { size: 34, cor: verde, caixa: [380, 330, 500, 50], t0: d0 + 0.4, t1: fim(T) });
  [["#312 Inscrição com confirmação", "  • e-mail de confirmação"], ["#305 Login social", "  • login com Google"], ["#298 Filtro de eventos", "  • filtro por data"]].forEach(([a, b], i) => {
    c.texto(a, 0, 0, { size: 30, cor: navy, caixa: [400, 385 + i * 100, 1000, 45], t0: pal(5, /^descri/i) - 0.2 + i * 0.5, t1: fim(T) });
    c.texto(b, 0, 0, { size: 26, cor: "#5b6272", caixa: [400, 425 + i * 100, 1000, 40], t0: pal(5, /^descri/i) - 0.1 + i * 0.5, t1: fim(T), estilo: "Medium" }); });
}

export function cap7(c, T) {
  const { S, E, pal } = ctx(c, T);
  // f0-1: faixa de atualização sobre o app
  const a1 = S[2] - 0.2;
  const A = appJanela(c, 0.3, a1, "Pasta local");
  const b0 = S[0] + 0.3;
  c.forma("rounded", 960, 180, 1160, 100, azul, { raio: 24, t0: b0, t1: a1, borda: bd, bordaW: 5 });
  c.texto("Nova versão disponível", 0, 0, { size: 34, cor: branco, caixa: [440, 154, 560, 50], t0: b0, t1: a1 });
  botao(c, 1130, 180, 250, 70, "Atualizar agora", pal(1, /^Voc/i) - 0.5, a1, { cor: verde, size: 26, destaque: pal(1, /^atualizar/i) - 0.2 });
  botao(c, 1390, 180, 210, 70, "Agora não", pal(1, /^Voc/i) - 0.5, a1, { cor: "#d8cdb4", corTxt: navy, size: 26, destaque: pal(1, /^depois/i) - 0.5 });
  // f2: cartão final
  const f0 = S[2] - 0.1;
  c.forma("rounded", 960, 520, 1300, 600, cream, { raio: 40, t0: f0, borda: bd, bordaW: 8 });
  c.forma("rounded", 960, 340, 150, 150, azul, { raio: 38, t0: f0 + 0.1, borda: bd, bordaW: 6 });
  c.texto("T", 960, 342, { size: 120, cor: branco, t0: f0 + 0.15 });
  c.texto("Taiga Assist", 960, 480, { size: 100, cor: navy, t0: f0 + 0.2 });
  c.texto("Menos relatório, mais código.", 960, 580, { size: 44, cor: "#5b6272", t0: pal(2, /^Menos/i) - 0.2, estilo: "Medium" });
  c.texto("NTI Sistemas", 960, 670, { size: 70, cor: branco, borda: "#000000", bordaW: 12, t0: pal(2, /^Menos/i) + 0.3 });
  c.texto("github.com/renanrocket/ci_cd-taiga-assist/releases", 960, 750, { size: 30, cor: azul, t0: pal(2, /^Baixe/i) - 0.3, estilo: "Medium" });
}
