// Vídeo só do fundo animado, para revisar o movimento. Uso: node video/scripts/fundo-preview.mjs [segundos=45]
import fs from "node:fs"; import path from "node:path"; import { execSync } from "node:child_process"; import { fileURLToPath } from "node:url";
import { Cena } from "./lib.mjs"; import { montarFundo } from "./fundo.mjs";
const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), ".."); const seg = Number(process.argv[2] || 45);
fs.mkdirSync(`${RAIZ}/gerado`, { recursive: true }); fs.mkdirSync(`${RAIZ}/out`, { recursive: true });
const c = new Cena("Fundo", 1920, 1080, 30, seg); montarFundo(c, 1920, 1080, "16x9", `${RAIZ}/arte`, 0);
const jl = `${RAIZ}/gerado/fundo-16x9.jsonl`, proj = `${RAIZ}/gerado/fundo-16x9.ecproj`;
fs.writeFileSync(jl, c.jsonl()); if (fs.existsSync(proj)) fs.unlinkSync(proj);
execSync(`effectcraft-cli --empty --save-as ${proj} run --commands ${jl} 2>&1`, { maxBuffer: 1 << 28 });
execSync(`effectcraft-cli --project ${proj} render --comp Fundo --out ${RAIZ}/out/fundo-animado-16x9.mp4 --format h264 --audio off 2>&1`, { maxBuffer: 1 << 28 });
console.log(`ok: ${RAIZ}/out/fundo-animado-16x9.mp4 (${seg} s)`);
