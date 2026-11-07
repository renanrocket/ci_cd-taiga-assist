"""Gera, por fala, mp3 + tempos de palavras (edge-tts) e o words.json do capítulo.
Uso: video/.venv/bin/python video/scripts/tts.py [capitulo ...]"""
import asyncio, json, sys, pathlib
import edge_tts

RAIZ = pathlib.Path(__file__).resolve().parents[1]
ROT = json.loads((RAIZ / "roteiros/video.json").read_text())
PRON = {k: v for k, v in json.loads((RAIZ / "roteiros/pronuncia.json").read_text()).items() if not k.startswith("_")}
SAIDA = RAIZ / "audio/falas"; SAIDA.mkdir(parents=True, exist_ok=True)

import re
def falado(texto):
    """Troca palavras pela pronúncia; devolve (texto falado, nº de tokens falados por palavra original, palavras originais)."""
    orig, partes, qtd = [], [], []
    for tok in texto.split():
        m = re.match(r"^(\W*)(.*?)(\W*)$", tok, re.UNICODE); pre, nucleo, pos = m.groups()
        rep = PRON.get(nucleo.lower(), nucleo)
        orig.append(nucleo); partes.append(pre + rep + pos); qtd.append(len(rep.split()))
    return " ".join(partes), qtd, orig

async def fala(cap, i, texto, tent=0):
    dito, qtd, orig = falado(texto)
    com = edge_tts.Communicate(dito, ROT["voz"], boundary="WordBoundary")
    mp3 = SAIDA / f"c{cap}_{i:02d}.mp3"; palavras = []
    with open(mp3, "wb") as f:
        async for ev in com.stream():
            if ev["type"] == "audio": f.write(ev["data"])
            elif ev["type"] == "WordBoundary":
                palavras.append({"w": ev["text"], "t": ev["offset"] / 1e7, "d": ev["duration"] / 1e7})
    if len(palavras) != sum(qtd):
        if tent < 4: return await fala(cap, i, texto, tent + 1)
        raise SystemExit(f"alinhamento falhou em c{cap} fala {i}: {len(palavras)} tokens falados vs {sum(qtd)} esperados ({dito!r})")
    alinhadas, k = [], 0   # uma entrada por palavra ORIGINAL (legenda), com o tempo dos tokens falados
    for w, n in zip(orig, qtd):
        grupo = palavras[k:k + n]; k += n
        alinhadas.append({"w": w, "t": grupo[0]["t"], "d": grupo[-1]["t"] + grupo[-1]["d"] - grupo[0]["t"]})
    return {"cap": cap, "i": i, "texto": texto, "falado": dito, "mp3": str(mp3.relative_to(RAIZ)), "palavras": alinhadas}

async def main():
    alvo = {int(a) for a in sys.argv[1:]}
    for c in ROT["capitulos"]:
        if alvo and c["id"] not in alvo: continue
        res = [await fala(c["id"], i, t) for i, t in enumerate(c["falas"])]
        (SAIDA / f"c{c['id']}.words.json").write_text(json.dumps(res, ensure_ascii=False, indent=1))
        print("capítulo", c["id"], "ok")
asyncio.run(main())
