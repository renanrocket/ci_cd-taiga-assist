// Uso: node video/scripts/build.mjs <capitulo> <16x9|9x16>
import fs from "node:fs"; import path from "node:path"; import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { Cena, COR } from "./lib.mjs"; import { montarFundo } from "./fundo.mjs"; import { cap1 } from "./cap1.mjs"; import { cap2, cap3, cap4, cap5, cap6, cap7 } from "./caps.mjs";
const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const [cap = "1", fmt = "16x9"] = process.argv.slice(2);
const [W, H] = fmt === "16x9" ? [1920, 1080] : [1080, 1920];
const falas = JSON.parse(fs.readFileSync(`${RAIZ}/audio/falas/c${cap}.words.json`));
const dur = f => parseFloat(execSync(`ffprobe -v error -show_entries format=duration -of csv=p=0 ${RAIZ}/${f.mp3}`).toString());
const GAP = 0.35, inicio = []; let t = 0.5;
for (const f of falas) { inicio.push(t); t += dur(f) + GAP; }
const fim = t + 0.6;
const T = { inicio, fim, palavras: falas.map(f => f.palavras), falas, dur: fim };
fs.mkdirSync(`${RAIZ}/gerado`, { recursive: true });
// duração (s) dos capítulos anteriores = deslocamento da animação do fundo (continuidade entre capítulos)
let desloc = 0; for (let k = 1; k < Number(cap); k++) { const f = `${RAIZ}/gerado/cap${k}-tempos.json`; if (fs.existsSync(f)) desloc += Math.ceil(JSON.parse(fs.readFileSync(f)).fim); }
const c = new Cena(`Cap${cap}`, W, H, 30, Math.ceil(fim));
montarFundo(c, W, H, fmt, `${RAIZ}/arte`, desloc);
({ 1: cap1, 2: cap2, 3: cap3, 4: cap4, 5: cap5, 6: cap6, 7: cap7 })[cap](c, T);
// legendas: grupos de 2–3 palavras; palavra falada em laranja
const capY = fmt === "16x9" ? H * 0.8 : H * 0.62;
falas.forEach((f, i) => {
  const ps = f.palavras; let g = [];
  const grupos = [];
  ps.forEach((p, k) => { g.push(k); if (g.length === 3 || /[.,:;?!]$/.test(p.w) || k === ps.length - 1) { grupos.push(g); g = []; } });
  grupos.forEach(gr => gr.forEach((k, j) => {
    const txt = gr.map(x => ps[x].w).join(" ").toUpperCase();
    const ini = inicio[i] + ps[k].t - 0.02, fimw = j === gr.length - 1 ? Math.min(inicio[i] + ps[k].t + ps[k].d + 0.15, k + 1 < ps.length ? inicio[i] + ps[k + 1].t - 0.02 : 1e9) : inicio[i] + ps[gr[j + 1]].t - 0.02;
    const nome = `leg_${i}_${k}`;
    c.c("layer.newText", { text: txt, name: nome, font: "Inter", style: "Bold", size: fmt === "16x9" ? 78 : 92, fill: COR.branco,
      stroke: "#000000", applyStroke: true, strokeWidth: 10, strokeOverFill: false, position: [W / 2, capY], justify: "center" });
    const ant = gr.slice(0, j).map(x => ps[x].w.length + 1).reduce((a, b) => a + b, 0);
    c.c("layer.setText", { layer: nome, range: [ant, ant + ps[k].w.length], fill: COR.laranja });
    c.expr(nome, "transform/opacity", `time>=${ini.toFixed(3)}&&time<${fimw.toFixed(3)}?100:0`);
  }));
});
// vinheta de legibilidade já está no SVG; escreve e executa
const jl = `${RAIZ}/gerado/cap${cap}-${fmt}.jsonl`, proj = `${RAIZ}/gerado/cap${cap}-${fmt}.ecproj`;
fs.writeFileSync(jl, c.jsonl());
if (fs.existsSync(proj)) fs.unlinkSync(proj);
try { execSync(`effectcraft-cli --empty --save-as ${proj} run --commands ${jl} 2>&1`, { maxBuffer: 1 << 28 }); }
catch (e) { console.log(String(e.stdout).split("\n").filter(l => /effectcraft-cli run/.test(l)).join("\n")); process.exit(1); }
fs.writeFileSync(`${RAIZ}/gerado/cap${cap}-tempos.json`, JSON.stringify({ inicio, fim }));
console.log(`ok cap${cap} ${fmt}: ${c.cmds.length} comandos, duração ${fim.toFixed(1)} s`);
