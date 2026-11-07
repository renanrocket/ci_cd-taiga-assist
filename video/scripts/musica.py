"""Prepara a trilha: usa o 1º .mp3 de video/audio/musica (principal.mp3 tem prioridade).
Se o vídeo for maior que a faixa, repete em loop com transição suave (acrossfade);
no fim sempre esmaece (fade-out). Volume baixo (a voz é mixada por cima, com ducking)."""
import math, pathlib, subprocess

CF = 3.0      # s de transição entre repetições
FADE_IN, FADE_OUT = 1.0, 3.0
VOLUME = 0.13

def achar(raiz: pathlib.Path):
    pasta = raiz / "audio/musica"
    p = pasta / "principal.mp3"
    if p.exists(): return p
    outros = sorted(pasta.glob("*.mp3"))
    return outros[0] if outros else None

def duracao(f):
    return float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(f)]))

def preparar(mus, total, saida):
    d = duracao(mus)
    n = 1 if d >= total else math.ceil((total - CF) / (d - CF))   # nº de cópias para cobrir o vídeo
    args = ["ffmpeg", "-y", "-loglevel", "error"]
    for _ in range(n): args += ["-i", str(mus)]
    cadeia = [f"[{i}:a]aresample=48000,aformat=channel_layouts=stereo[a{i}]" for i in range(n)]
    ult = "a0"
    for i in range(1, n):
        cadeia.append(f"[{ult}][a{i}]acrossfade=d={CF}:c1=tri:c2=tri[x{i}]"); ult = f"x{i}"
    cadeia.append(f"[{ult}]atrim=0:{total},volume={VOLUME},afade=t=in:d={FADE_IN},afade=t=out:st={total - FADE_OUT}:d={FADE_OUT}[m]")
    subprocess.run(args + ["-filter_complex", ";".join(cadeia), "-map", "[m]", str(saida)], check=True, capture_output=True)
    return n, d

if __name__ == "__main__":   # teste: python3 musica.py <segundos> <saida.wav>
    import sys
    r = pathlib.Path(__file__).resolve().parents[1]
    print(preparar(achar(r), float(sys.argv[1]), sys.argv[2]))
