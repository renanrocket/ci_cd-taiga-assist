//! Pipeline "push → relatório (Kiro) → US no Taiga".
//!
//! Porte de `taiga.run_for_push / send_report / resend_pending`. Regras mantidas:
//! - idempotência por repositório + SHA curto + branch gatilho (tabela `reports`);
//! - valida login e projeto do Taiga ANTES de gastar uma execução do Kiro;
//! - se algo falha depois do push, o relatório fica em `.kiro/git-reports` para reenvio manual;
//! - nunca derruba o commit/push por causa do Taiga: devolve um `Outcome`.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

use crate::report::{self, Report};
use crate::taiga::Client;

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ok: bool,
    pub detail: String,
}

fn ok(detail: impl Into<String>) -> Outcome {
    Outcome { ok: true, detail: detail.into() }
}
fn fail(detail: impl Into<String>) -> Outcome {
    Outcome { ok: false, detail: detail.into() }
}

pub struct TaigaCfg {
    pub host: String,
    pub usuario: String,
    pub senha: String,
    pub slug: String,
}

pub struct KiroCfg {
    pub bin: PathBuf,
    pub api_key: Option<String>,
}

/// Tudo o que identifica o repositório no pipeline.
pub struct RepoCtx {
    pub git: PathBuf,
    pub dir: PathBuf,
    pub repo_id: i64,
    pub project_id: i64,
    pub label: String,
}

/// Cliente autenticado + projeto já carregado (login e leitura do projeto uma única vez).
struct Session {
    client: Client,
    project: Value,
}

async fn open_session(t: &TaigaCfg) -> Result<Session, String> {
    let mut client = Client::new(&t.host, &t.usuario, &t.senha).map_err(|e| e.message)?;
    client.login().await.map_err(|e| e.message)?;
    let project = client.project(&t.slug).await.map_err(|e| e.message)?;
    Ok(Session { client, project })
}

fn lock<T>(db: &Mutex<Connection>, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, String> {
    let c = db.lock().map_err(|_| "banco ocupado".to_string())?;
    f(&c).map_err(|e| e.to_string())
}

fn sha7(sha: &str) -> &str {
    &sha[..7.min(sha.len())]
}

fn already_sent(db: &Mutex<Connection>, repo_id: i64, sha: &str, branch: &str) -> Option<Option<i64>> {
    lock(db, |c| {
        c.query_row(
            "SELECT us_ref FROM reports WHERE repo_id = ?1 AND sha = ?2 AND branch = ?3",
            rusqlite::params![repo_id, sha7(sha), branch],
            |r| r.get::<_, Option<i64>>(0),
        )
        .optional()
    })
    .ok()
    .flatten()
}

fn ref_txt(r: Option<i64>) -> String {
    r.map(|n| format!("#{n}")).unwrap_or_else(|| "(sem número)".into())
}

/// Cria a US a partir do texto do relatório; apaga o arquivo se der certo.
#[allow(clippy::too_many_arguments)]
async fn send_with(
    db: &Mutex<Connection>,
    ctx: &RepoCtx,
    s: &Session,
    text: &str,
    name: &str,
    sha: &str,
    branch: &str,
    dias: i64,
    today: NaiveDate,
) -> Outcome {
    if let Some(r) = already_sent(db, ctx.repo_id, sha, branch) {
        report::delete_report_file(&ctx.dir, name);
        return ok(format!("US já criada antes ({}).", ref_txt(r)));
    }
    let kept = " Relatório mantido em .kiro/git-reports.";
    let rep: Report = match report::parse_report(text) {
        Ok(r) => r,
        Err(e) => return fail(format!("{e}{kept}")),
    };
    let Some(user_id) = s.client.user_id else { return fail(format!("Login sem usuário.{kept}")) };
    let footer = format!("Repositório: {} · branch: {} · commit: {}", ctx.label, branch, sha7(sha));
    let payload = match report::build_us_payload(&s.project, &rep, user_id, &footer, today, dias) {
        Ok(p) => p,
        Err(e) => return fail(format!("{e}{kept}")),
    };
    let us = match s.client.create_us(&payload).await {
        Ok(u) => u,
        Err(e) => return fail(format!("{}{kept}", e.message)),
    };
    let (us_id, us_ref) = (us.get("id").and_then(Value::as_i64), us.get("ref").and_then(Value::as_i64));
    let Some(us_id) = us_id else { return fail(format!("O Taiga não devolveu o id da história.{kept}")) };
    let saved = lock(db, |c| {
        c.execute(
            "INSERT INTO reports(repo_id, sha, branch, project_id, us_id, us_ref, titulo) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![ctx.repo_id, sha7(sha), branch, ctx.project_id, us_id, us_ref, rep.titulo],
        )
    });
    // A US já existe no Taiga: apaga o arquivo de qualquer forma (reenviar duplicaria a história).
    report::delete_report_file(&ctx.dir, name);
    match saved {
        Ok(_) => ok(format!("História {} criada no Taiga.", ref_txt(us_ref))),
        Err(e) => ok(format!("História {} criada no Taiga (não consegui registrar localmente: {e}).", ref_txt(us_ref))),
    }
}

/// Gera o relatório do push e cria a US. Nunca levanta.
#[allow(clippy::too_many_arguments)]
pub async fn run_for_push(
    db: &Mutex<Connection>,
    ctx: &RepoCtx,
    taiga: &TaigaCfg,
    kiro: &KiroCfg,
    branch: &str,
    old: &str,
    new: &str,
    dias: i64,
    today: NaiveDate,
    progress: &(dyn Fn(String) + Send + Sync),
) -> Outcome {
    if let Some(r) = already_sent(db, ctx.repo_id, new, branch) {
        return ok(format!("US deste commit já foi criada ({}).", ref_txt(r)));
    }
    progress(format!("{} · {branch}: conferindo o Taiga", ctx.label));
    let session = match open_session(taiga).await {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    progress(format!("{} · {branch}: gerando o relatório com o Kiro (pode levar alguns minutos)", ctx.label));
    let generated = match report::generate(&ctx.git, &kiro.bin, kiro.api_key.as_deref(), &ctx.dir, branch, old, new).await {
        Ok(g) => g,
        Err(e) => return fail(e),
    };
    progress(format!("{} · {branch}: criando a história no Taiga", ctx.label));
    send_with(db, ctx, &session, &generated.text, &generated.name, new, branch, dias, today).await
}

/// Reenvia os relatórios que ficaram em `.kiro/git-reports` (falharam antes).
pub async fn resend_pending(
    db: &Mutex<Connection>,
    ctx: &RepoCtx,
    taiga: &TaigaCfg,
    branch: &str,
    dias: i64,
    today: NaiveDate,
) -> Vec<(String, Outcome)> {
    let files = report::pending_files(&ctx.dir);
    if files.is_empty() {
        return Vec::new();
    }
    let session = match open_session(taiga).await {
        Ok(s) => s,
        Err(e) => return files.into_iter().map(|(n, _)| (n, fail(e.clone()))).collect(),
    };
    let mut out = Vec::new();
    for (name, sha) in files {
        let path = report::reports_dir(&ctx.dir).join(&name);
        let o = match std::fs::read_to_string(&path) {
            Ok(text) => send_with(db, ctx, &session, &text, &name, &sha, branch, dias, today).await,
            Err(_) => fail("Não consegui ler o arquivo."),
        };
        out.push((name, o));
    }
    out
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn sh(git: &Path, dir: &Path, args: &[&str]) -> String {
        let o = std::process::Command::new(git).arg("-C").arg(dir).args(args).output().unwrap();
        assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_owned()
    }

    const RELATORIO: &str = "Título\nAjusta o cadastro\n\nDescrição\nCorrige a validação do formulário.\n\nPontuação\nUX: 1\nDesign: 1\nFront: 2\nBack: 3\n";

    struct Fx {
        _t: tempfile::TempDir,
        git: PathBuf,
        work: PathBuf,
        kiro: PathBuf,
        db: Mutex<Connection>,
        repo_id: i64,
    }

    /// Repositório com 2 commits (o último é o "push"), Kiro simulado que devolve `RELATORIO`
    /// e banco com o vínculo repositório → projeto.
    fn fixture(reply_text: &str) -> Option<Fx> {
        let git = which::which("git").ok()?;
        let t = tempfile::tempdir().unwrap();
        let work = t.path().join("meu-repo");
        std::fs::create_dir_all(&work).unwrap();
        sh(&git, &work, &["init", "-q", "-b", "prod"]);
        sh(&git, &work, &["config", "user.name", "T"]);
        sh(&git, &work, &["config", "user.email", "t@t"]);
        std::fs::write(work.join("a.txt"), "1\n").unwrap();
        sh(&git, &work, &["add", "."]);
        sh(&git, &work, &["commit", "-q", "-m", "um"]);
        std::fs::write(work.join("a.txt"), "2\n").unwrap();
        sh(&git, &work, &["commit", "-q", "-am", "dois"]);

        let reply = json!({"type": "runFinished", "data": {"status": "success", "finalText": reply_text}});
        std::fs::write(t.path().join("reply.json"), format!("{reply}\n")).unwrap();
        let kiro = t.path().join("kiro-fake");
        std::fs::write(&kiro, format!("#!/bin/sh\ncat '{}'\n", t.path().join("reply.json").display())).unwrap();
        std::fs::set_permissions(&kiro, std::fs::Permissions::from_mode(0o755)).unwrap();

        let db = crate::db::open_in_memory().unwrap();
        db.execute("INSERT INTO taiga_accounts(id,nome,host,usuario,secret_ref) VALUES (1,'A','h','u','r')", []).unwrap();
        db.execute("INSERT INTO taiga_projects(id,account_id,slug,nome) VALUES (1,1,'proj','Proj')", []).unwrap();
        db.execute("INSERT INTO local_folders(id,caminho) VALUES (1,?1)", [t.path().to_str().unwrap()]).unwrap();
        db.execute("INSERT INTO local_repos(id,folder_id,rel,project_id,branch_gatilho) VALUES (1,1,'meu-repo',1,'prod')", []).unwrap();
        Some(Fx { _t: t, git, work, kiro, db: Mutex::new(db), repo_id: 1 })
    }

    fn mock_taiga(server: &MockServer) {
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/auth");
            then.status(200).json_body(json!({"auth_token": "T", "id": 42}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/projects/by_slug").query_param("slug", "proj");
            then.status(200).json_body(json!({
                "id": 77, "default_us_status": 3,
                "roles": [{"id": 10, "name": "UX"}, {"id": 11, "name": "Design"}, {"id": 12, "name": "Front"}, {"id": 13, "name": "Back"}],
                "points": [{"id": 2, "value": 1.0}, {"id": 3, "value": 2.0}, {"id": 4, "value": 3.0}],
                "us_statuses": [{"id": 3, "is_closed": false}]
            }));
        });
    }

    fn ctx(f: &Fx) -> RepoCtx {
        RepoCtx { git: f.git.clone(), dir: f.work.clone(), repo_id: f.repo_id, project_id: 1, label: "meu-repo".into() }
    }

    fn cfgs(f: &Fx, server: &MockServer) -> (TaigaCfg, KiroCfg) {
        (
            TaigaCfg { host: server.base_url(), usuario: "u".into(), senha: "s3nha".into(), slug: "proj".into() },
            KiroCfg { bin: f.kiro.clone(), api_key: None },
        )
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    fn noprog() -> impl Fn(String) + Send + Sync {
        |_| {}
    }

    #[tokio::test]
    async fn do_push_a_us_criada_e_idempotente() {
        let Some(f) = fixture(RELATORIO) else { return };
        let server = MockServer::start();
        mock_taiga(&server);
        let create = server.mock(|when, then| {
            when.method(POST).path("/api/v1/userstories").header("authorization", "Bearer T");
            then.status(201).json_body(json!({"id": 900, "ref": 350}));
        });
        let (t, k) = cfgs(&f, &server);
        let new = sh(&f.git, &f.work, &["rev-parse", "HEAD"]);
        let old = sh(&f.git, &f.work, &["rev-parse", "HEAD~1"]);

        let o = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", &old, &new, 15, today(), &noprog()).await;
        assert_eq!(o, ok("História #350 criada no Taiga."), "{o:?}");
        create.assert();

        // registrou, e não sobrou arquivo de relatório nem de contexto
        let (sha, us_ref): (String, i64) = f.db.lock().unwrap()
            .query_row("SELECT sha, us_ref FROM reports WHERE repo_id = 1", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((sha.as_str(), us_ref), (&new[..7], 350));
        let left: Vec<_> = std::fs::read_dir(report::reports_dir(&f.work)).map(|d| d.flatten().collect()).unwrap_or_default();
        assert!(left.is_empty(), "sobrou: {left:?}");

        // segunda execução do mesmo commit: não chama o Taiga de novo nem o Kiro
        let o2 = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", &old, &new, 15, today(), &noprog()).await;
        assert!(o2.ok && o2.detail.contains("já foi criada"), "{o2:?}");
        assert_eq!(create.hits(), 1);
    }

    #[tokio::test]
    async fn payload_enviado_ao_taiga_tem_titulo_pontos_prazo_e_rodape() {
        let Some(f) = fixture(RELATORIO) else { return };
        let server = MockServer::start();
        mock_taiga(&server);
        let create = server.mock(|when, then| {
            when.method(POST).path("/api/v1/userstories").json_body_includes(
                json!({
                    "project": 77, "subject": "Ajusta o cadastro", "assigned_to": 42, "status": 3,
                    "due_date": "2026-11-24",
                    "points": {"10": 2, "11": 2, "12": 3, "13": 4}
                })
                .to_string(),
            );
            then.status(201).json_body(json!({"id": 1, "ref": 1}));
        });
        let (t, k) = cfgs(&f, &server);
        let new = sh(&f.git, &f.work, &["rev-parse", "HEAD"]);
        let o = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", "", &new, 15, today(), &noprog()).await;
        assert!(o.ok, "{o:?}");
        create.assert();
    }

    #[tokio::test]
    async fn falha_do_taiga_mantem_o_relatorio_e_o_reenvio_cria_a_us() {
        let Some(f) = fixture(RELATORIO) else { return };
        let server = MockServer::start();
        mock_taiga(&server);
        let mut broken = server.mock(|when, then| {
            when.method(POST).path("/api/v1/userstories");
            then.status(500).json_body(json!({"_error_message": "boom"}));
        });
        let (t, k) = cfgs(&f, &server);
        let new = sh(&f.git, &f.work, &["rev-parse", "HEAD"]);

        let o = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", "", &new, 15, today(), &noprog()).await;
        assert!(!o.ok && o.detail.contains("Relatório mantido"), "{o:?}");
        let pend = report::pending_files(&f.work);
        assert_eq!(pend.len(), 1);
        assert_eq!(pend[0].1, new[..7]);
        let n: i64 = f.db.lock().unwrap().query_row("SELECT count(*) FROM reports", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "nada registrado quando a criação falha");

        // Taiga volta: reenvio cria a US e limpa o arquivo
        broken.delete();
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/userstories");
            then.status(201).json_body(json!({"id": 5, "ref": 12}));
        });
        let re = resend_pending(&f.db, &ctx(&f), &t, "prod", 15, today()).await;
        assert_eq!(re.len(), 1);
        assert!(re[0].1.ok, "{re:?}");
        assert!(report::pending_files(&f.work).is_empty());
        assert!(resend_pending(&f.db, &ctx(&f), &t, "prod", 15, today()).await.is_empty());
    }

    #[tokio::test]
    async fn taiga_inacessivel_nao_gasta_o_kiro() {
        let Some(f) = fixture(RELATORIO) else { return };
        let marker = f._t.path().join("kiro-rodou");
        std::fs::write(&f.kiro, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        let t = TaigaCfg { host: "http://127.0.0.1:1".into(), usuario: "u".into(), senha: "s3nha".into(), slug: "proj".into() };
        let k = KiroCfg { bin: f.kiro.clone(), api_key: None };
        let new = sh(&f.git, &f.work, &["rev-parse", "HEAD"]);
        let o = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", "", &new, 15, today(), &noprog()).await;
        assert!(!o.ok);
        assert!(!o.detail.contains("s3nha"));
        assert!(!marker.exists(), "o Kiro não pode rodar se o Taiga está fora");
    }

    #[tokio::test]
    async fn resposta_invalida_do_kiro_e_erro_sem_criar_us() {
        let Some(f) = fixture("Não consegui analisar as alterações.") else { return };
        let server = MockServer::start();
        mock_taiga(&server);
        let create = server.mock(|when, then| {
            when.method(POST).path("/api/v1/userstories");
            then.status(201).json_body(json!({"id": 1, "ref": 1}));
        });
        let (t, k) = cfgs(&f, &server);
        let new = sh(&f.git, &f.work, &["rev-parse", "HEAD"]);
        let o = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", "", &new, 15, today(), &noprog()).await;
        assert!(!o.ok && o.detail.contains("Título"), "{o:?}");
        assert_eq!(create.hits(), 0);
    }

    #[tokio::test]
    async fn pontuacao_invalida_mantem_o_arquivo_para_revisao() {
        let ruim = "Título\nX\nDescrição\nY\nPontuação\nUX: 4\nDesign: 1\nFront: 1\nBack: 1\n";
        let Some(f) = fixture(ruim) else { return };
        let server = MockServer::start();
        mock_taiga(&server);
        let (t, k) = cfgs(&f, &server);
        let new = sh(&f.git, &f.work, &["rev-parse", "HEAD"]);
        let o = run_for_push(&f.db, &ctx(&f), &t, &k, "prod", "", &new, 15, today(), &noprog()).await;
        assert!(!o.ok && o.detail.contains("Pontuação inválida"), "{o:?}");
        assert_eq!(report::pending_files(&f.work).len(), 1);
    }
}
