"""Monta o áudio (voz + música) e, se pedido, o vídeo final.
Uso: python3 video/scripts/montar.py 16x9              → áudio + concatena os MP4 dos capítulos + muxa
     python3 video/scripts/montar.py 16x9 --sufixo=-v2 → grava out/taiga-assist-16x9-v2.mp4 (não sobrescreve a versão anterior)
     python3 video/scripts/montar.py 16x9 --so-audio   → só o áudio: video/audio/previa-audio.mp3 (para ouvir; não precisa dos MP4)"""
import json, math, subprocess, sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
R = pathlib.Path(__file__).resolve().parents[1]; fmt = next((a for a in sys.argv[1:] if not a.startswith("--")), "16x9"); SO_AUDIO = "--so-audio" in sys.argv
SUF = next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--sufixo=")), "")   # ex.: --sufixo=-v2 → taiga-assist-16x9-v2.mp4
run = lambda *a: subprocess.run(a, check=True, capture_output=True)
caps = range(1, 8); ins = []; filtros = []; seg = []; off = 0.0
# 1) voz: cada fala atrasada para (início do capítulo + início da fala)
args = ["ffmpeg", "-y", "-loglevel", "error"]; idx = 0; rotulos = []
for c in caps:
    t = json.load(open(R / f"gerado/cap{c}-tempos.json")); f = json.load(open(R / f"audio/falas/c{c}.words.json"))
    for i, fala in enumerate(f):
        args += ["-i", str(R / fala["mp3"])]; ms = int((off + t["inicio"][i]) * 1000)
        filtros.append(f"[{idx}:a]adelay={ms}|{ms},aresample=48000[v{idx}]"); rotulos.append(f"[v{idx}]"); idx += 1
    off += math.ceil(t["fim"])   # duração do capítulo = duração da composição (ceil do fim da última fala + cauda)
total = off
voz = R / "audio/voz.wav"
fc = ";".join(filtros) + ";" + "".join(rotulos) + f"amix=inputs={idx}:normalize=0:dropout_transition=0,apad=whole_dur={total},atrim=0:{total}[o]"
run(*args, "-filter_complex", fc, "-map", "[o]", "-ac", "2", str(voz))
# 2) música (opcional): loop com transição se o vídeo for maior; fade-out no fim; ducking sob a voz
import musica
mus = musica.achar(R); mix = R / "audio/mix.wav"
if mus:
    fundo = R / "audio/musica-ajustada.wav"
    n, dm = musica.preparar(mus, total, fundo)
    run("ffmpeg", "-y", "-loglevel", "error", "-i", str(voz), "-i", str(fundo), "-filter_complex",
        "[1:a][0:a]sidechaincompress=threshold=0.03:ratio=8:attack=20:release=400[md];[0:a][md]amix=inputs=2:normalize=0[o]",
        "-map", "[o]", str(mix))
else:
    mix = voz
# prévia só do áudio (mesma normalização do vídeo final) para ouvir antes do render
previa = R / "audio/previa-audio.mp3"
run("ffmpeg", "-y", "-loglevel", "error", "-i", str(mix), "-af", "loudnorm=I=-14:TP=-1.5:LRA=11", "-b:a", "192k", str(previa))
if SO_AUDIO:
    print("ok áudio", total, f"s | música: {mus.name} ({n} cópia(s), loop com transição)" if mus else "| sem música", "→", previa); sys.exit(0)
# 3) vídeo concatenado + áudio normalizado (~-14 LUFS) → MP4 +faststart
lista = R / "out/cap/lista.txt"; lista.write_text("".join(f"file 'cap{c}.mp4'\n" for c in caps))
run("ffmpeg", "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", str(lista), "-i", str(mix), "-map", "0:v", "-map", "1:a",
    "-c:v", "copy", "-af", "loudnorm=I=-14:TP=-1.5:LRA=11", "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart", "-shortest",
    str(R / f"out/taiga-assist-{fmt}{SUF}.mp4"))
print("ok", total, f"música: {mus.name} ({n} repetição(ões))" if mus else "sem música")
