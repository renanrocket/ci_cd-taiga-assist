// Mini DSL: emite comandos do EffectCraft (JSONL). Cada elemento é uma camada com expressões de tempo.
export const COR = { navy:"#0b111f", navy2:"#1c2332", gold:"#f1d390", cream:"#f6efe2", laranja:"#f07a1a",
  verde:"#2fa364", vermelho:"#d9503f", azul:"#2f6fed", branco:"#ffffff", cinza:"#8a8f9c" };
export const hex = h => { const n = parseInt(h.slice(1), 16); return [(n>>16&255)/255, (n>>8&255)/255, (n&255)/255, 1]; };

export class Cena {
  constructor(nome, w, h, fps, dur) { this.cmds = []; this.n = 0; this.W = w; this.H = h;
    this.c("comp.new", { name: nome, width: w, height: h, frameRate: fps, duration: dur }); }
  c(command, params) { this.cmds.push({ command, params }); }
  nome(p) { return `${p}_${++this.n}`; }
  // visibilidade: aparece em t0, some em t1 (Infinity = fica); f = duração do fade
  vis(t0, t1 = Infinity, f = 0.3) {
    const a = `ease(time,${t0},${t0 + f},0,100)`;
    return t1 === Infinity ? a : `${a}-ease(time,${t1},${t1 + f},0,100)`;
  }
  expr(layer, path, expression) { this.c("prop.setExpression", { layer, path, expression }); }
  entrada(layer, t0, t1, { f = 0.3, de = 90, y = 0, x = 0, escala = true } = {}) {
    this.expr(layer, "transform/opacity", this.vis(t0, t1, f));
    if (escala) this.expr(layer, "transform/scale", `s=ease(time,${t0},${t0 + f},${de},100);[s,s,100]`);
  }
  fundo(cor) { const n = this.nome("fundo"); this.c("layer.newSolid", { name: n, color: cor }); return n; }
  forma(kind, cx, cy, w, h, cor, { t0 = 0, t1 = Infinity, f = 0.3, raio, borda, bordaW = 4, nome, anim = true, de = 92 } = {}) {
    const n = nome || this.nome(kind);
    this.c("layer.newShape", { kind, name: n, size: [w, h], ...(cor ? { fill: cor } : { fillType: "none" }), position: [cx, cy], ...(borda ? { stroke: borda, strokeWidth: bordaW } : {}) });
    if (kind === "rounded" && raio !== undefined) this.c("prop.set", { layer: n, path: "contents/group/contents/rect/roundness", value: raio });
    if (anim) this.entrada(n, t0, t1, { f, de });
    (this.pos ||= {})[n] = [cx, cy];
    return n;
  }
  // texto centrado em (cx,cy), ou à esquerda numa caixa [x,y,w,h] se `caixa`
  texto(txt, cx, cy, { size = 60, cor = COR.branco, fonte = "Inter", estilo = "Bold", borda, bordaW = 8, t0 = 0, t1 = Infinity,
    f = 0.3, caixa, justify, nome, anim = true, de = 92, leading } = {}) {
    const n = nome || this.nome("txt");
    this.c("layer.newText", { text: txt, name: n, font: fonte, style: estilo, size, fill: cor,
      ...(borda ? { stroke: borda, applyStroke: true, strokeWidth: bordaW, strokeOverFill: false } : {}),
      ...(caixa ? { box: caixa, justify: justify || "left" } : { position: [cx, cy + size * 0.25, ], justify: justify || "center" }),
      ...(leading ? { leading } : {}) });
    if (anim) this.entrada(n, t0, t1, { f, de });
    (this.pos ||= {})[n] = [cx, cy + size * 0.25];
    return n;
  }
  corPorTempo(layer, path, t, antes, depois, f = 0.3) {
    const A = hex(antes), D = hex(depois);
    const m = i => `${A[i]}+(${D[i]}-${A[i]})*k`;
    this.expr(layer, path, `k=ease(time,${t},${t + f},0,1);[${m(0)},${m(1)},${m(2)},1]`);
  }
  imagem(item, t0 = 0) { this.c("layer.addItem", { item, time: t0, position: [this.W / 2, this.H / 2] }); }
  // texto digitado: camadas com prefixos crescentes (expressões de texto não funcionam), esquerda fixa
  digitar(txt, caixa, { t0, t1, cps = 38, passo = 3, ...o }) {
    const n = Math.ceil(txt.length / passo);
    for (let k = 1; k <= n; k++) {
      const ini = t0 + (k * passo - passo) / cps, fim = k === n ? t1 : t0 + (k * passo) / cps;
      const nome = this.nome("dig");
      this.texto(txt.slice(0, k * passo), 0, 0, { ...o, caixa, nome, anim: false });
      this.expr(nome, "transform/opacity", `time>=${ini.toFixed(3)}&&time<${fim.toFixed(3)}?100:0`);
    }
  }
  // move camadas (de posição conhecida) por (dx,dy) entre a e b, com ease
  mover(nomes, dx, dy, a, b) {
    for (const n of nomes) { const [x, y] = this.pos[n];
      this.expr(n, "transform/position", `k=ease(time,${a},${b},0,1);[${x}+${dx}*k,${y}+${dy}*k,0]`); }
  }
  jsonl() { return this.cmds.map(x => JSON.stringify(x)).join("\n") + "\n"; }
}
