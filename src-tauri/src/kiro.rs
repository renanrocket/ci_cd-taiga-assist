//! Integração com o `kiro-cli` (login, API key opcional e execução headless).
//!
//! Formato confirmado na prática (Kiro CLI 2.22): `chat --no-interactive --output-format stream-json`
//! imprime um JSON por linha; o resultado vem no último evento `runFinished`
//! (`status`, `finalText`, `finalTextTruncated`). A API key (opcional) vai em `KIRO_API_KEY`,
//! nunca na linha de comando.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

pub const TIMEOUT_WHOAMI: Duration = Duration::from_secs(20);
pub const TIMEOUT_TEST: Duration = Duration::from_secs(120);

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WhoAmI {
    pub account_type: String,
    pub email: Option<String>,
    pub region: Option<String>,
}

/// `whoami --format json` imprime o JSON na 1ª linha e depois texto solto ("Profile: …").
pub fn parse_whoami(stdout: &str) -> Option<WhoAmI> {
    let line = stdout.lines().map(str::trim).find(|l| l.starts_with('{'))?;
    let v: Value = serde_json::from_str(line).ok()?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    Some(WhoAmI { account_type: s("accountType").unwrap_or_else(|| "desconhecido".into()), email: s("email"), region: s("region") })
}

#[derive(Debug, PartialEq)]
pub struct StreamResult {
    pub final_text: String,
    /// Créditos consumidos (soma de `meteringUsage`), quando informado.
    pub credits: Option<f64>,
}

/// Resultado de uma execução `stream-json`. Falha se não houver `runFinished`, se o status não for
/// `success` ou se o texto veio truncado (não aceitamos relatório cortado).
pub fn parse_stream(stdout: &str) -> Result<StreamResult, String> {
    let mut finished: Option<Value> = None;
    let mut credits = 0.0;
    let mut has_credits = false;
    for line in stdout.lines().map(str::trim).filter(|l| l.starts_with('{')) {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        match v.get("type").and_then(Value::as_str) {
            Some("runFinished") => finished = Some(v),
            Some("metadata") => {
                if let Some(arr) = v.pointer("/data/meteringUsage").and_then(Value::as_array) {
                    for m in arr {
                        if let Some(x) = m.get("value").and_then(Value::as_f64) {
                            credits += x;
                            has_credits = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let f = finished.ok_or("O kiro-cli não retornou o evento final.")?;
    let status = f.pointer("/data/status").and_then(Value::as_str).unwrap_or("desconhecido");
    if status != "success" {
        return Err(format!("O kiro-cli terminou com status: {status}."));
    }
    if f.pointer("/data/finalTextTruncated").and_then(Value::as_bool).unwrap_or(false) {
        return Err("A resposta do Kiro veio truncada. Tente de novo.".into());
    }
    let final_text = f.pointer("/data/finalText").and_then(Value::as_str).unwrap_or("").to_owned();
    Ok(StreamResult { final_text, credits: has_credits.then_some(credits) })
}

pub struct Output {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Executa o kiro-cli em `cwd` (use uma pasta neutra para o teste). Sanitiza a API key da saída.
pub async fn run(kiro: &Path, cwd: &Path, api_key: Option<&str>, args: &[&str], timeout: Duration) -> Result<Output, String> {
    let mut cmd = Command::new(kiro);
    cmd.args(args).current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    if let Some(k) = api_key.filter(|k| !k.is_empty()) {
        cmd.env("KIRO_API_KEY", k);
    }
    crate::proc::no_window(&mut cmd);
    let out = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| format!("Tempo esgotado ({}s) esperando o kiro-cli.", timeout.as_secs()))?
        .map_err(|e| format!("Não consegui executar o kiro-cli: {e}"))?;
    let secrets: Vec<String> = api_key.map(|k| vec![k.to_owned()]).unwrap_or_default();
    Ok(Output {
        ok: out.status.success(),
        stdout: crate::git::sanitize(&String::from_utf8_lossy(&out.stdout), &secrets),
        stderr: crate::git::sanitize(&String::from_utf8_lossy(&out.stderr), &secrets),
    })
}

/// Remove sequências ANSI (cores) de saídas de terminal.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}


fn pump<R: AsyncRead + Unpin + Send + 'static>(stream: R, tx: tokio::sync::mpsc::UnboundedSender<String>) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stream).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            let _ = tx.send(l);
        }
    });
}

/// Executa um comando de longa duração (ex.: `login`) repassando cada linha (stdout e stderr) em
/// tempo real. `Ok(true)` = terminou com sucesso. Mata o processo se passar de `timeout`.
pub async fn run_streaming(
    kiro: &Path,
    args: &[&str],
    on_line: &(dyn Fn(String) + Send + Sync),
    timeout: Duration,
) -> Result<bool, String> {
    let mut cmd = Command::new(kiro);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    crate::proc::no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| format!("Não consegui executar o kiro-cli: {e}"))?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    if let Some(o) = child.stdout.take() {
        pump(o, tx.clone());
    }
    if let Some(e) = child.stderr.take() {
        pump(e, tx.clone());
    }
    drop(tx);
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            line = rx.recv() => match line {
                Some(l) => {
                    let l = strip_ansi(&l);
                    let l = l.trim();
                    if !l.is_empty() { on_line(l.to_owned()); }
                }
                None => break,
            },
            _ = &mut deadline => {
                let _ = child.kill().await;
                return Err(format!("Tempo esgotado ({} min) esperando o login.", timeout.as_secs() / 60));
            }
        }
    }
    let status = child.wait().await.map_err(|e| e.to_string())?;
    Ok(status.success())
}

#[derive(Debug, Serialize)]
pub struct Login {
    pub logged_in: bool,
    pub who: Option<WhoAmI>,
    pub detail: String,
}

/// Estado do login (`kiro-cli whoami`).
pub async fn login_status(kiro: &Path, api_key: Option<&str>) -> Login {
    let tmp = std::env::temp_dir();
    match run(kiro, &tmp, api_key, &["whoami", "--format", "json"], TIMEOUT_WHOAMI).await {
        Err(e) => Login { logged_in: false, who: None, detail: e },
        Ok(o) => {
            let who = if o.ok { parse_whoami(&strip_ansi(&o.stdout)) } else { None };
            match who {
                Some(w) => Login { logged_in: true, who: Some(w), detail: String::new() },
                None => {
                    let msg = strip_ansi(if o.stderr.trim().is_empty() { &o.stdout } else { &o.stderr });
                    let msg = msg.trim();
                    Login {
                        logged_in: false,
                        who: None,
                        detail: if msg.is_empty() { "Não está logado no Kiro.".into() } else { msg.chars().take(300).collect() },
                    }
                }
            }
        }
    }
}

/// Teste ponta a ponta: pede uma resposta mínima e confere o `runFinished`.
pub async fn headless_test(kiro: &Path, api_key: Option<&str>) -> Result<String, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    let o = run(
        kiro,
        dir.path(),
        api_key,
        &["chat", "--agent-engine", "v2", "--no-interactive", "--output-format", "stream-json", "--wrap", "never", "--trust-tools=", "Responda apenas com a palavra OK."],
        TIMEOUT_TEST,
    )
    .await?;
    if !o.ok && !o.stdout.contains("runFinished") {
        let e = strip_ansi(&o.stderr);
        let e: String = e.trim().chars().rev().take(300).collect::<Vec<_>>().into_iter().rev().collect();
        return Err(if e.is_empty() { "O kiro-cli falhou sem mensagem.".into() } else { e });
    }
    let r = parse_stream(&o.stdout)?;
    if !r.final_text.to_uppercase().contains("OK") {
        return Err(format!("Resposta inesperada do Kiro: {}", r.final_text.chars().take(120).collect::<String>()));
    }
    let secs = started.elapsed().as_secs_f32();
    Ok(match r.credits {
        Some(c) => format!("Kiro respondeu em {secs:.1}s (≈{c:.2} crédito)."),
        None => format!("Kiro respondeu em {secs:.1}s."),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHOAMI: &str = "{\"accountType\":\"IamIdentityCenter\",\"email\":\"a@b.c\",\"region\":\"us-east-1\",\"startUrl\":\"https://x\"}\n\nProfile:\nKiroProfile-us-east-1\narn:aws:...\n";

    const STREAM: &str = r#"{"type":"runStarted","data":{"engine":"v2"}}
{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"O"}}}}
{"type":"metadata","data":{"meteringUsage":[{"value":0.05,"unit":"credit"}],"turnDurationMs":1612}}
{"type":"metadata","data":{"meteringUsage":[{"value":0.01,"unit":"credit"}]}}
{"type":"runFinished","data":{"status":"success","stopReason":"end_turn","finalText":"OK","finalTextTruncated":false}}
"#;

    #[test]
    fn whoami_le_o_json_da_primeira_linha() {
        let w = parse_whoami(WHOAMI).unwrap();
        assert_eq!(w.account_type, "IamIdentityCenter");
        assert_eq!(w.email.as_deref(), Some("a@b.c"));
        assert_eq!(w.region.as_deref(), Some("us-east-1"));
    }

    #[test]
    fn whoami_sem_json_nao_e_login() {
        assert!(parse_whoami("Not logged in\n").is_none());
        assert!(parse_whoami("{quebrado").is_none());
        assert!(parse_whoami("").is_none());
    }

    #[test]
    fn stream_pega_o_texto_final_e_soma_creditos() {
        let r = parse_stream(STREAM).unwrap();
        assert_eq!(r.final_text, "OK");
        assert!((r.credits.unwrap() - 0.06).abs() < 1e-9);
    }

    #[test]
    fn stream_usa_o_ultimo_run_finished_e_ignora_lixo() {
        let s = format!("aviso qualquer\n{{nao e json\n{STREAM}{{\"type\":\"runFinished\",\"data\":{{\"status\":\"success\",\"finalText\":\"SEGUNDO\"}}}}\n");
        assert_eq!(parse_stream(&s).unwrap().final_text, "SEGUNDO");
    }

    #[test]
    fn stream_falhas() {
        assert!(parse_stream("").unwrap_err().contains("evento final"));
        assert!(parse_stream("{\"type\":\"runStarted\",\"data\":{}}").is_err());
        let falha = r#"{"type":"runFinished","data":{"status":"error","finalText":"x"}}"#;
        assert!(parse_stream(falha).unwrap_err().contains("status: error"));
        let trunc = r#"{"type":"runFinished","data":{"status":"success","finalText":"corta","finalTextTruncated":true}}"#;
        assert!(parse_stream(trunc).unwrap_err().contains("truncada"));
    }

    #[test]
    fn stream_sem_creditos_informados() {
        let s = r#"{"type":"runFinished","data":{"status":"success","finalText":"OK"}}"#;
        assert_eq!(parse_stream(s).unwrap().credits, None);
    }

    #[test]
    fn strip_ansi_remove_cores() {
        assert_eq!(strip_ansi("\u{1b}[1;31mfalhou\u{1b}[0m ok"), "falhou ok");
    }

    /// Simula o kiro-cli com um script: confere que a API key vai por ambiente e é sanitizada.
    #[cfg(unix)]
    #[tokio::test]
    async fn api_key_vai_por_ambiente_e_some_da_saida() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let fake = t.path().join("kiro-fake");
        std::fs::write(
            &fake,
            "#!/bin/sh\nprintf '{\"type\":\"runFinished\",\"data\":{\"status\":\"success\",\"finalText\":\"OK chave=%s\"}}\\n' \"$KIRO_API_KEY\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let o = run(&fake, t.path(), Some("ksk_segredo"), &["x"], Duration::from_secs(10)).await.unwrap();
        assert!(o.ok);
        assert!(!o.stdout.contains("ksk_segredo"), "a chave não pode aparecer na saída");
        assert!(o.stdout.contains("***"));
        let r = headless_via(&fake, t.path()).await;
        assert!(r.is_ok(), "{r:?}");
    }

    #[cfg(unix)]
    async fn headless_via(fake: &Path, dir: &Path) -> Result<String, String> {
        let o = run(fake, dir, None, &[], Duration::from_secs(10)).await?;
        parse_stream(&o.stdout).map(|r| r.final_text)
    }

    #[tokio::test]
    async fn kiro_inexistente_da_erro_claro() {
        let l = login_status(Path::new("/nao/existe/kiro"), None).await;
        assert!(!l.logged_in);
        assert!(l.detail.contains("Não consegui executar"));
    }

    #[cfg(unix)]
    fn script(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn streaming_repassa_stdout_e_stderr_linha_a_linha() {
        let t = tempfile::tempdir().unwrap();
        let k = script(t.path(), "k", "echo 'Abra https://exemplo.com/device'; echo '\u{1b}[1mCódigo: ABCD-1234\u{1b}[0m' >&2; echo ''; exit 0");
        let got = std::sync::Mutex::new(Vec::<String>::new());
        let ok = run_streaming(&k, &["login"], &|l| got.lock().unwrap().push(l), Duration::from_secs(10)).await.unwrap();
        assert!(ok);
        let mut v = got.into_inner().unwrap();
        v.sort();
        assert_eq!(v, ["Abra https://exemplo.com/device", "Código: ABCD-1234"], "sem ANSI e sem linhas vazias");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn streaming_devolve_falso_quando_o_comando_falha_e_mata_no_timeout() {
        let t = tempfile::tempdir().unwrap();
        let falha = script(t.path(), "f", "echo 'erro'; exit 3");
        assert!(!run_streaming(&falha, &[], &|_| {}, Duration::from_secs(10)).await.unwrap());
        let lento = script(t.path(), "l", "echo 'esperando'; sleep 30");
        let e = run_streaming(&lento, &[], &|_| {}, Duration::from_millis(600)).await.unwrap_err();
        assert!(e.contains("Tempo esgotado"), "{e}");
    }
}
