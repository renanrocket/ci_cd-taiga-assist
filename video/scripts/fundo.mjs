// Fundo: quadro Kanban visto de cima, com UMA IMAGEM (SVG) POR COLUNA, para as colunas poderem se mover
// (uma sobe enquanto a vizinha desce, bem devagar), e uma camada de sombra + vinheta por cima.
// Os SVGs ficam em video/arte/ (versionados) e são regravados a cada build.
import fs from "node:fs"; import path from "node:path";

export const COLUNAS = 5, EXT = 140;                // EXT: sobra vertical (px) acima/abaixo, para o movimento nunca mostrar a ponta
const AMPLITUDE = [34, 26, 38, 26, 34];             // px de sobe-e-desce de cada coluna
const VELOCIDADE = 0.30;                            // rad/s (período ≈ 21 s)

export function gerarSvgs(W, H, fmt, pasta) {
  let seed = 7; const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
  const gap = 36, cw = (W - gap * (COLUNAS + 1)) / COLUNAS, HH = H + 2 * EXT, arquivos = [];
  for (let i = 0; i < COLUNAS; i++) {
    const x = gap + i * (cw + gap);
    let s = `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${HH}" viewBox="0 0 ${W} ${HH}">` +
      `<rect x="${x}" y="0" width="${cw}" height="${HH}" rx="26" fill="#5d5d57"/>` +
      `<rect x="${x + 24}" y="${EXT + 56}" width="${cw * 0.45}" height="22" rx="11" fill="#828073"/>`;
    for (let y = EXT + 110; y < HH - 60;) {
      const h = 90 + Math.floor(rnd() * 90);
      s += `<rect x="${x + 20}" y="${y}" width="${cw - 40}" height="${h}" rx="18" fill="#76746a"/>` +
        `<rect x="${x + 40}" y="${y + 22}" width="${(cw - 80) * (0.5 + rnd() * 0.45)}" height="16" rx="8" fill="#a39f8d"/>` +
        `<rect x="${x + 40}" y="${y + 52}" width="${(cw - 80) * (0.3 + rnd() * 0.3)}" height="14" rx="7" fill="#8c8977"/>`;
      y += h + 22;
    }
    const f = path.join(pasta, `fundo-coluna-${i + 1}-${fmt}.svg`); fs.writeFileSync(f, s + "</svg>"); arquivos.push(f);
  }
  const sombra = path.join(pasta, `fundo-sombra-${fmt}.svg`);
  fs.writeFileSync(sombra, `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}"><defs>` +
    `<radialGradient id="v" cx="50%" cy="50%" r="72%"><stop offset="35%" stop-color="#05080f" stop-opacity="0"/><stop offset="100%" stop-color="#05080f" stop-opacity="0.92"/></radialGradient></defs>` +
    `<rect width="${W}" height="${H}" fill="#0b111f" opacity="0.62"/><rect width="${W}" height="${H}" fill="url(#v)"/></svg>`);
  return { colunas: arquivos, sombra };
}

// Camadas (de baixo para cima): base lisa → 5 colunas animadas → sombra/vinheta. `deslocamento` = segundos de vídeo já
// passados antes deste trecho, para o movimento continuar sem salto entre capítulos.
export function montarFundo(c, W, H, fmt, pasta, deslocamento = 0) {
  const { colunas, sombra } = gerarSvgs(W, H, fmt, pasta);
  c.fundo("#4e4e4a");
  c.c("file.import", { paths: [...colunas, sombra] });
  colunas.forEach((f, i) => {
    const nome = path.basename(f); c.imagem(nome);
    const sinal = i % 2 ? 1 : -1;                    // pares sobem quando ímpares descem, e vice-versa
    c.expr(nome, "transform/position", `[${W / 2},${H / 2}+${sinal * AMPLITUDE[i]}*Math.sin((time+${deslocamento.toFixed(3)})*${VELOCIDADE}),0]`);
  });
  c.imagem(path.basename(sombra));
}
