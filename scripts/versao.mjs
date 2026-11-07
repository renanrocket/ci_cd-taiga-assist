// Atualiza o número da versão nos três lugares onde ele existe. Uso: npm run versao -- 0.2.0
// Depois: commit, `git tag v0.2.0` e `git push origin prod v0.2.0` (o GitHub publica a versão).
import { readFileSync, writeFileSync } from "node:fs";

const nova = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(nova ?? "")) {
  console.error("Uso: npm run versao -- X.Y.Z   (ex.: 0.2.0)");
  process.exit(1);
}

const json = (arquivo) => {
  const dados = JSON.parse(readFileSync(arquivo, "utf8"));
  const anterior = dados.version;
  dados.version = nova;
  writeFileSync(arquivo, JSON.stringify(dados, null, 2) + "\n");
  return anterior;
};

const anterior = json("src-tauri/tauri.conf.json");
json("package.json");

const toml = readFileSync("src-tauri/Cargo.toml", "utf8");
const novoToml = toml.replace(/^version = ".*"$/m, `version = "${nova}"`);
if (novoToml === toml && anterior !== nova) {
  console.error("Não achei a linha 'version' em src-tauri/Cargo.toml.");
  process.exit(1);
}
writeFileSync("src-tauri/Cargo.toml", novoToml);

console.log(`Versão ${anterior} → ${nova} (tauri.conf.json, package.json, Cargo.toml).`);
console.log(`Próximos passos: commit, git tag v${nova}, git push origin prod v${nova}`);
