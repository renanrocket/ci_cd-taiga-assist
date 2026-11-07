"""Transforma o log do cargo (teste ou build) em anotações do GitHub, legíveis na página do run e pela API pública.
Uso: python anotar.py <log> <rótulo>"""
import re, sys
def esc(t): return t.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
def prop(t): return esc(t).replace(":", "%3A").replace(",", "%2C")
log = open(sys.argv[1], encoding="utf-8", errors="replace").read().replace("\r\n", "\n"); rot = sys.argv[2]
n = 0
# 1) testes que falharam (corpo do pânico)
for nome, corpo in re.findall(r"---- (\S+) stdout ----\n(.*?)(?=\n---- |\nfailures:\n|\Z)", log, re.S):
    print(f"::error title={prop(rot + ' - teste falhou - ' + nome)}::{esc(corpo.strip()[:3500])}"); n += 1
# 2) erros de compilação do rustc: "error[E0xxx]: msg\n  --> arquivo:linha:coluna"
for msg, arq, lin in re.findall(r"^(error(?:\[E\d+\])?: [^\n]+)\n\s*--> ([^\n:]+(?::[^\n:]+)?):(\d+):\d+", log, re.M):
    print(f"::error title={prop(rot + ' - erro de compilação')},file={prop(arq)},line={lin}::{esc(msg)}"); n += 1
# 3) sempre: resumo + final do log (para nunca ficar sem pista)
resumo = [l for l in log.splitlines() if l.startswith("test result:") or l.startswith("error") or "FAILED" in l][:25]
print(f"::notice title={prop(rot + ' - resumo')}::{esc(chr(10).join(resumo)[:3000] or '(sem linhas de resumo)')}")
print(f"::notice title={prop(rot + ' - final do log')}::{esc(chr(10).join(log.splitlines()[-45:])[:3800])}")
print(f"{rot}: {n} anotação(ões) de erro")
