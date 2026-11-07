//! Operações git do fluxo "Pasta local": status, commit em lote em várias branches e push.
//!
//! Porte de `localgit.py` do ci_cd. Princípios (iguais ao original):
//! - nunca reescreve histórico: só fast-forward, cherry-pick e commit;
//! - em qualquer conflito aborta e deixa a branch como estava;
//! - alterações pendentes que não foram selecionadas são guardadas (stash) e restauradas no fim;
//! - a branch em que o usuário estava é sempre restaurada.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::error::AppResult;
use crate::git::{self, last_chars, Auth, Kind, Output};

const T_LOCAL: Duration = git::TIMEOUT_LOCAL;
const T_NET: Duration = git::TIMEOUT_REMOTE;
const T_LONG: Duration = Duration::from_secs(120);

/// Credencial para falar com o remoto (segredo já lido do chaveiro).
pub struct RemoteCred {
    pub kind: Kind,
    pub usuario: Option<String>,
    pub secret: Option<String>,
    pub known_hosts: PathBuf,
}

impl RemoteCred {
    pub fn auth(&self, url: &str) -> AppResult<Auth> {
        git::build_auth(self.kind, self.usuario.as_deref(), self.secret.as_deref(), url, &self.known_hosts)
    }
}

async fn g(git: &Path, dir: &Path, auth: &Auth, args: &[&str], t: Duration) -> Output {
    git::run(git, Some(dir), auth, args, t)
        .await
        .unwrap_or_else(|e| Output { ok: false, stdout: String::new(), stderr: e })
}

async fn gl(git: &Path, dir: &Path, args: &[&str]) -> Output {
    g(git, dir, &git::plain_auth(), args, T_LOCAL).await
}

fn msg(o: &Output) -> String {
    let t = if o.stderr.trim().is_empty() { o.stdout.trim() } else { o.stderr.trim() };
    last_chars(t, 600)
}

// --------------------------------------------------------------------------- //
// Leitura (sem alterar nada)
// --------------------------------------------------------------------------- //

pub async fn current_branch(git: &Path, dir: &Path) -> Option<String> {
    let o = gl(git, dir, &["symbolic-ref", "--short", "-q", "HEAD"]).await;
    let b = o.stdout.trim().to_owned();
    (o.ok && !b.is_empty()).then_some(b)
}

pub async fn dirty_count(git: &Path, dir: &Path) -> usize {
    let o = gl(git, dir, &["status", "--porcelain"]).await;
    if !o.ok {
        return 0;
    }
    o.stdout.lines().filter(|l| !l.trim().is_empty()).count()
}

async fn rev(git: &Path, dir: &Path, refname: &str) -> Option<String> {
    let o = gl(git, dir, &["rev-parse", "--verify", "-q", refname]).await;
    let s = o.stdout.trim().to_owned();
    (o.ok && !s.is_empty()).then_some(s)
}

async fn is_ancestor(git: &Path, dir: &Path, a: &str, b: &str) -> bool {
    gl(git, dir, &["merge-base", "--is-ancestor", a, b]).await.ok
}

/// `sem_remoto` | `ok` | `adiantado` (há commits a enviar) | `desatualizado` (remoto à frente/divergente).
pub async fn classify(git: &Path, dir: &Path, local: &str, remote: Option<&str>) -> &'static str {
    match remote {
        None => "sem_remoto",
        Some(r) if r == local => "ok",
        Some(r) => {
            if is_ancestor(git, dir, r, local).await {
                "adiantado"
            } else {
                "desatualizado"
            }
        }
    }
}

/// `ls-remote --heads`: branch → SHA.
pub async fn remote_heads(git: &Path, dir: &Path, cred: &RemoteCred, url: &str) -> Result<HashMap<String, String>, String> {
    let auth = cred.auth(url).map_err(|e| e.to_string())?;
    let o = git::run(git, Some(dir), &auth, &["ls-remote", "--heads", &auth.url], T_NET).await?;
    if !o.ok {
        return Err(last_chars(&o.stderr, 400));
    }
    Ok(o.stdout
        .lines()
        .filter_map(|l| {
            let (sha, r) = l.split_once('\t')?;
            Some((r.strip_prefix("refs/heads/")?.to_owned(), sha.trim().to_owned()))
        })
        .collect())
}

/// Estado de cada branch local frente ao remoto (um único `ls-remote`).
pub async fn branch_states(git: &Path, dir: &Path, cred: &RemoteCred) -> Result<HashMap<String, &'static str>, String> {
    let url = git::remote_url(git, dir).await.ok_or("Repositório sem remote 'origin'.")?;
    let heads = remote_heads(git, dir, cred, &url).await?;
    let mut out = HashMap::new();
    for br in git::local_branches(git, dir).await {
        if let Some(local) = rev(git, dir, &format!("refs/heads/{br}")).await {
            out.insert(br.clone(), classify(git, dir, &local, heads.get(&br).map(String::as_str)).await);
        }
    }
    Ok(out)
}

/// Para cada branch, as OUTRAS que estão à frente dela (ancestral estrita). Só local.
pub async fn behind_siblings(git: &Path, dir: &Path, branches: &[String]) -> HashMap<String, Vec<String>> {
    let mut shas: Vec<(String, String)> = Vec::new();
    for b in branches.iter().take(30) {
        if let Some(s) = rev(git, dir, &format!("refs/heads/{b}")).await {
            shas.push((b.clone(), s));
        }
    }
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (x, sx) in &shas {
        for (y, sy) in &shas {
            if x != y && sx != sy && is_ancestor(git, dir, sx, sy).await {
                out.entry(x.clone()).or_default().push(y.clone());
            }
        }
    }
    out
}

pub struct Plan {
    pub target: Option<String>,
    pub count: i64,
    pub problem: String,
}

/// Alvo do fast-forward de `br`: a branch local mais avançada que contém `br`.
pub async fn plan_update(git: &Path, dir: &Path, br: &str) -> Plan {
    let none = Plan { target: None, count: 0, problem: String::new() };
    let names = git::local_branches(git, dir).await;
    let mut sha_of: Vec<(String, String)> = Vec::new();
    for n in names.iter().take(50) {
        if let Some(s) = rev(git, dir, &format!("refs/heads/{n}")).await {
            sha_of.push((n.clone(), s));
        }
    }
    let Some(me) = sha_of.iter().find(|(n, _)| n == br).map(|(_, s)| s.clone()) else { return none };
    let mut cands: Vec<&(String, String)> = Vec::new();
    for c in &sha_of {
        if c.0 != br && c.1 != me && is_ancestor(git, dir, &me, &c.1).await {
            cands.push(c);
        }
    }
    if cands.is_empty() {
        return none;
    }
    let mut top: Vec<&(String, String)> = Vec::new();
    for n in &cands {
        let mut all = true;
        for m in &cands {
            if m.0 != n.0 && !is_ancestor(git, dir, &m.1, &n.1).await {
                all = false;
                break;
            }
        }
        if all {
            top.push(n);
        }
    }
    if top.is_empty() {
        let mut names: Vec<&str> = cands.iter().map(|c| c.0.as_str()).collect();
        names.sort();
        return Plan { target: None, count: 0, problem: format!("várias branches à frente sem relação entre si ({})", names.join(", ")) };
    }
    top.sort_by(|a, b| a.0.cmp(&b.0));
    let t = top[0];
    let n = gl(git, dir, &["rev-list", "--count", &format!("{me}..{}", t.1)]).await;
    Plan { target: Some(t.0.clone()), count: n.stdout.trim().parse().unwrap_or(0), problem: String::new() }
}

// --------------------------------------------------------------------------- //
// Escrita
// --------------------------------------------------------------------------- //

/// Avança `br` (em checkout) até a branch local que está à frente. `Ok("")` = nada a fazer.
async fn fast_forward(git: &Path, dir: &Path, env: &Auth, br: &str) -> Result<String, String> {
    let plan = plan_update(git, dir, br).await;
    if !plan.problem.is_empty() {
        return Err(format!("Não foi possível atualizar: {}.", plan.problem));
    }
    let Some(target) = plan.target else { return Ok(String::new()) };
    let o = g(git, dir, env, &["merge", "--ff-only", &target], T_LONG).await;
    if !o.ok {
        return Err(format!("Não consegui avançar até {target}: {}", msg(&o)));
    }
    Ok(format!("Avançada até {target} (+{} commit(s)).", plan.count))
}

/// Se o remoto tem commits que a branch local não tem, traz por fast-forward.
async fn sync_remote(git: &Path, dir: &Path, cred: Option<&RemoteCred>, env: &Auth, br: &str) -> Result<String, String> {
    let Some(cred) = cred else { return Ok(String::new()) };
    let Some(url) = git::remote_url(git, dir).await else { return Ok(String::new()) };
    let auth = cred.auth(&url).map_err(|e| e.to_string())?;
    let lr = git::run(git, Some(dir), &auth, &["ls-remote", &auth.url, &format!("refs/heads/{br}")], T_NET)
        .await
        .map_err(|_| "Falha ao consultar o remoto (tempo esgotado ou credencial).".to_string())?;
    let remote = if lr.ok { lr.stdout.split_whitespace().next().map(str::to_owned) } else { None };
    let local = rev(git, dir, &format!("refs/heads/{br}")).await.unwrap_or_default();
    let Some(remote) = remote.filter(|r| *r != local) else { return Ok(String::new()) };
    if is_ancestor(git, dir, &remote, &local).await {
        return Ok(String::new()); // local à frente: o push resolve
    }
    let f = git::run(git, Some(dir), &auth, &["fetch", &auth.url, &format!("refs/heads/{br}")], T_NET)
        .await
        .map_err(|e| format!("Falha ao buscar o remoto: {e}"))?;
    if !f.ok {
        return Err(format!("Falha ao buscar o remoto: {}", msg(&f)));
    }
    let fetched = gl(git, dir, &["rev-parse", "FETCH_HEAD"]).await.stdout.trim().to_owned();
    if !is_ancestor(git, dir, &local, &fetched).await {
        return Err("A branch divergiu do remoto (há commits dos dois lados): resolva manualmente com merge/rebase.".into());
    }
    let n = gl(git, dir, &["rev-list", "--count", &format!("{local}..{fetched}")]).await.stdout.trim().to_owned();
    let o = g(git, dir, env, &["merge", "--ff-only", &fetched], T_LONG).await;
    if !o.ok {
        return Err(format!("Não consegui atualizar com o remoto: {}", msg(&o)));
    }
    Ok(format!("Atualizada com o remoto (+{n} commit(s))."))
}

/// Atualiza `br` a partir do remoto **só por fast-forward** (o `git pull --ff-only` daquela branch), sem trocar de branch.
/// Branch em checkout: `merge --ff-only` (o git recusa se alterações locais seriam sobrescritas). Outra branch: só move a
/// referência. Se divergiu do remoto, recusa e orienta a resolver fora do app. Devolve a mensagem de resultado.
pub async fn pull_branch(git: &Path, dir: &Path, cred: &RemoteCred, br: &str) -> Result<String, String> {
    let url = git::remote_url(git, dir).await.ok_or("O repositório não tem remoto configurado.")?;
    let auth = cred.auth(&url).map_err(|e| e.to_string())?;
    let local = rev(git, dir, &format!("refs/heads/{br}")).await.ok_or_else(|| format!("A branch {br} não existe localmente."))?;
    let f = git::run(git, Some(dir), &auth, &["fetch", &auth.url, &format!("refs/heads/{br}")], T_NET)
        .await
        .map_err(|_| "Falha ao buscar o remoto (tempo esgotado ou credencial).".to_string())?;
    if !f.ok {
        return Err(format!("Falha ao buscar o remoto: {}", msg(&f)));
    }
    let fetched = gl(git, dir, &["rev-parse", "FETCH_HEAD"]).await.stdout.trim().to_owned();
    if fetched == local || is_ancestor(git, dir, &fetched, &local).await {
        let a_frente = fetched != local;
        return Ok(if a_frente { "Nada a atualizar: a branch local está à frente do remoto (o push resolve).".into() } else { "Já está em dia com o remoto.".into() });
    }
    if !is_ancestor(git, dir, &local, &fetched).await {
        return Err("A branch divergiu do remoto (há commits dos dois lados): resolva manualmente com merge ou rebase.".into());
    }
    let n = gl(git, dir, &["rev-list", "--count", &format!("{local}..{fetched}")]).await.stdout.trim().to_owned();
    if current_branch(git, dir).await.as_deref() == Some(br) {
        let o = g(git, dir, &auth, &["merge", "--ff-only", &fetched], T_LONG).await;
        if !o.ok {
            return Err(format!("Não consegui atualizar (alterações locais podem conflitar): {}", msg(&o)));
        }
    } else {
        let o = gl(git, dir, &["update-ref", "-m", "pull --ff-only", &format!("refs/heads/{br}"), &fetched, &local]).await;
        if !o.ok {
            return Err(format!("Não consegui atualizar a branch: {}", msg(&o)));
        }
    }
    Ok(format!("Atualizada com o remoto (+{n} commit(s))."))
}

enum Rep {
    JaTem(String),
    Ok(String),
    Vazio(String),
    Erro(String),
}

/// Leva o commit `source_sha` (de `source_br`) para a branch em checkout. Em conflito aborta.
async fn replicate(git: &Path, dir: &Path, env: &Auth, source_sha: &str, source_br: &str) -> Rep {
    if is_ancestor(git, dir, source_sha, "HEAD").await {
        return Rep::JaTem(format!("Já contém o commit de {source_br}."));
    }
    let p = g(git, dir, env, &["cherry-pick", source_sha], T_LONG).await;
    if p.ok {
        return Rep::Ok(format!("Replicado de {source_br} ({}).", &source_sha[..7.min(source_sha.len())]));
    }
    let conflitos = gl(git, dir, &["ls-files", "-u"]).await.stdout.trim().to_owned();
    let vazio = conflitos.is_empty() && gl(git, dir, &["diff", "--cached", "--quiet"]).await.ok;
    let m = msg(&p);
    g(git, dir, env, &["cherry-pick", "--abort"], T_LOCAL).await;
    if vazio {
        return Rep::Vazio(format!("Já tinha as alterações de {source_br}."));
    }
    Rep::Erro(format!(
        "Conflito ao replicar {} de {source_br} nesta branch (branch mantida como estava; resolva manualmente). {m}",
        &source_sha[..7.min(source_sha.len())]
    ))
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct BranchResult {
    pub branch: String,
    /// commitado | atualizado | sem_alteracoes | erro
    pub status: String,
    pub detail: String,
    pub pushed: Option<bool>,
    pub push_detail: String,
    /// SHA remoto da branch antes do push ("" = branch nova).
    pub old_sha: String,
    pub new_sha: String,
    pub taiga: String,
    pub taiga_ok: Option<bool>,
}

#[derive(Debug, Serialize, Default)]
pub struct RepoResult {
    pub name: String,
    pub branches: Vec<BranchResult>,
    pub error: String,
    pub note: String,
}

/// Push da branch; devolve (ok, detalhe, SHA remoto anterior).
pub async fn push(git: &Path, dir: &Path, branch: &str, cred: &RemoteCred) -> (bool, String, String) {
    let Some(url) = git::remote_url(git, dir).await else {
        return (false, "Repositório sem remote 'origin'.".into(), String::new());
    };
    let auth = match cred.auth(&url) {
        Ok(a) => a,
        Err(e) => return (false, e.to_string(), String::new()),
    };
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let old = git::run(git, Some(dir), &auth, &["ls-remote", &auth.url, &format!("refs/heads/{branch}")], T_NET)
        .await
        .ok()
        .filter(|o| o.ok)
        .and_then(|o| o.stdout.split_whitespace().next().map(str::to_owned))
        .unwrap_or_default();
    match git::run(git, Some(dir), &auth, &["push", &auth.url, &refspec], T_NET).await {
        Ok(o) if o.ok => (true, { let m = msg(&o); if m.is_empty() { "ok".into() } else { m } }, old),
        Ok(o) => (false, msg(&o), old),
        Err(e) => (false, e, old),
    }
}

/// Impede que `git add .` leve relatórios pendentes: ignora `.kiro/git-reports/` só neste clone.
pub fn ensure_reports_ignored(dir: &Path) {
    let info = dir.join(".git").join("info");
    if !dir.join(".git").is_dir() {
        return;
    }
    let _ = std::fs::create_dir_all(&info);
    let file = info.join("exclude");
    let current = std::fs::read_to_string(&file).unwrap_or_default();
    if !current.lines().any(|l| l.trim() == ".kiro/git-reports/") {
        let sep = if current.is_empty() || current.ends_with('\n') { "" } else { "\n" };
        let _ = std::fs::write(&file, format!("{current}{sep}.kiro/git-reports/\n"));
    }
}

struct Run {
    source_sha: String,
    source_br: String,
    stashed: bool,
}

/// `git add .` + commit em cada branch selecionada (+ push). Restaura a branch original.
/// `progress` recebe uma frase por etapa (para a caixa de notificação).
pub async fn commit_branches(
    git: &Path,
    dir: &Path,
    branches: &[String],
    message: &str,
    push_after: bool,
    cred: Option<&RemoteCred>,
    progress: &(dyn Fn(String) + Send + Sync),
) -> RepoResult {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut result = RepoResult { name: name.clone(), ..Default::default() };
    let valid = git::local_branches(git, dir).await;
    let Some(orig) = current_branch(git, dir).await else {
        result.error = "HEAD destacado (detached): faça checkout de uma branch antes.".into();
        return result;
    };
    if push_after && cred.is_none() {
        result.error = "Sem credencial cadastrada para o push.".into();
        return result;
    }
    ensure_reports_ignored(dir);
    let env = git::plain_auth();

    // Branch atual primeiro: as alterações pendentes pertencem a ela.
    let mut ordered: Vec<String> = Vec::new();
    for b in branches {
        if !ordered.contains(b) {
            ordered.push(b.clone());
        }
    }
    ordered.sort_by_key(|b| (*b != orig, b.to_lowercase()));

    let mut st = Run { source_sha: String::new(), source_br: String::new(), stashed: false };
    for br in &ordered {
        progress(format!("{name} · {br}: processando"));
        let res = process_branch(git, dir, &env, &valid, br, message, push_after, cred, &mut st, progress, &name).await;
        result.branches.push(res);
    }

    if current_branch(git, dir).await.as_deref() != Some(orig.as_str()) {
        g(git, dir, &env, &["checkout", &orig, "--"], T_LONG).await;
    }
    if st.stashed {
        let pop = g(git, dir, &env, &["stash", "pop"], T_LONG).await;
        if pop.ok {
            result.note = format!(
                "Alterações pendentes de {orig} não foram commitadas (a branch não estava marcada); foram preservadas e restauradas."
            );
        } else {
            result.error = format!(
                "Não consegui restaurar as alterações pendentes; elas continuam guardadas em `git stash list`. {}",
                msg(&pop)
            );
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn process_branch(
    git: &Path,
    dir: &Path,
    env: &Auth,
    valid: &[String],
    br: &str,
    message: &str,
    push_after: bool,
    cred: Option<&RemoteCred>,
    st: &mut Run,
    progress: &(dyn Fn(String) + Send + Sync),
    name: &str,
) -> BranchResult {
    let mut res = BranchResult { branch: br.to_owned(), status: "erro".into(), ..Default::default() };
    if !valid.iter().any(|v| v == br) {
        res.detail = "Branch não existe localmente.".into();
        return res;
    }
    if current_branch(git, dir).await.as_deref() != Some(br) {
        if !st.stashed && dirty_count(git, dir).await > 0 {
            let sp = g(git, dir, env, &["stash", "push", "-u", "-m", "taiga-assist: auto-stash"], T_LONG).await;
            if !sp.ok {
                res.detail = format!("não consegui guardar as alterações pendentes: {}", msg(&sp));
                return res;
            }
            st.stashed = true;
        }
        let co = g(git, dir, env, &["checkout", br, "--"], T_LONG).await;
        if !co.ok {
            res.detail = format!("checkout falhou: {}", msg(&co));
            return res;
        }
    }

    let mut acts: Vec<String> = Vec::new();
    // 1) em dia com o remoto  2) em dia com a branch à frente — só fast-forward.
    match sync_remote(git, dir, cred, env, br).await {
        Ok(m) if !m.is_empty() => acts.push(m),
        Ok(_) => {}
        Err(m) => {
            res.detail = m;
            return res;
        }
    }
    match fast_forward(git, dir, env, br).await {
        Ok(m) if !m.is_empty() => acts.push(m),
        Ok(_) => {}
        Err(m) => {
            res.detail = m;
            return res;
        }
    }

    let add = g(git, dir, env, &["add", "."], T_LONG).await;
    if !add.ok {
        res.detail = format!("git add falhou: {}", msg(&add));
        return res;
    }
    let mut committed = false;
    let has_changes = !g(git, dir, env, &["diff", "--cached", "--quiet"], T_LOCAL).await.ok;
    if has_changes {
        let c = g(git, dir, env, &["commit", "-m", message], T_LONG).await;
        if !c.ok {
            res.detail = format!("commit falhou: {}", msg(&c));
            return res;
        }
        committed = true;
        acts.push(format!("Commit: {}", c.stdout.lines().next().unwrap_or("ok").trim()));
        if st.source_sha.is_empty() {
            st.source_sha = gl(git, dir, &["rev-parse", "HEAD"]).await.stdout.trim().to_owned();
            st.source_br = br.to_owned();
        }
    } else if !st.source_sha.is_empty() {
        let (sha, from) = (st.source_sha.clone(), st.source_br.clone());
        match replicate(git, dir, env, &sha, &from).await {
            Rep::Erro(m) => {
                res.detail = m;
                return res;
            }
            Rep::Ok(m) => {
                committed = true;
                acts.push(m);
            }
            Rep::JaTem(m) if acts.is_empty() => acts.push(m),
            Rep::JaTem(_) | Rep::Vazio(_) => {}
        }
    }
    res.status = if committed {
        "commitado"
    } else if acts.iter().any(|a| a.starts_with("Avançada") || a.starts_with("Atualizada")) {
        "atualizado"
    } else {
        "sem_alteracoes"
    }
    .into();
    res.detail = if acts.is_empty() { "Já está em dia; nada a commitar.".into() } else { acts.join(" ") };

    if push_after {
        if let Some(cred) = cred {
            progress(format!("{name} · {br}: enviando (push)"));
            let (ok, detail, old) = push(git, dir, br, cred).await;
            res.pushed = Some(ok);
            res.push_detail = detail;
            res.old_sha = old;
            res.new_sha = rev(git, dir, &format!("refs/heads/{br}")).await.unwrap_or_default();
            if !ok {
                res.status = "erro".into();
            }
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(git: &Path, dir: &Path, args: &[&str]) -> String {
        let o = std::process::Command::new(git).arg("-C").arg(dir).args(args).output().unwrap();
        assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_owned()
    }

    struct Fx {
        _t: tempfile::TempDir,
        git: PathBuf,
        work: PathBuf,
        bare: PathBuf,
        cred: RemoteCred,
    }

    /// Remoto `bare` com `prod` e `develop` no mesmo commit; clone de trabalho em `develop`.
    fn fixture() -> Option<Fx> {
        let git = which::which("git").ok()?;
        let t = tempfile::tempdir().unwrap();
        let bare = t.path().join("remoto.git");
        let work = t.path().join("trab");
        std::fs::create_dir_all(&bare).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        sh(&git, &bare, &["init", "--bare", "-q"]);
        sh(&git, &work, &["init", "-q", "-b", "prod"]);
        sh(&git, &work, &["config", "user.name", "Teste"]);
        sh(&git, &work, &["config", "user.email", "t@t.com"]);
        std::fs::write(work.join("a.txt"), "um\n").unwrap();
        sh(&git, &work, &["add", "."]);
        sh(&git, &work, &["commit", "-q", "-m", "inicial"]);
        sh(&git, &work, &["branch", "develop"]);
        sh(&git, &work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        sh(&git, &work, &["push", "-q", "origin", "prod", "develop"]);
        sh(&git, &work, &["checkout", "-q", "develop"]);
        let cred = RemoteCred { kind: Kind::Sistema, usuario: None, secret: None, known_hosts: t.path().join("kh") };
        Some(Fx { _t: t, git, work, bare, cred })
    }

    fn noprog() -> impl Fn(String) + Send + Sync {
        |_| {}
    }

    /// Faz um commit novo direto no remoto (por um segundo clone), simulando um colega que deu push.
    fn colega_envia(f: &Fx, branch: &str, arquivo: &str, texto: &str) {
        let outro = f._t.path().join(format!("colega-{branch}-{arquivo}"));
        sh(&f.git, f._t.path(), &["clone", "-q", f.bare.to_str().unwrap(), outro.to_str().unwrap()]);
        sh(&f.git, &outro, &["config", "user.name", "Colega"]);
        sh(&f.git, &outro, &["config", "user.email", "c@c.com"]);
        sh(&f.git, &outro, &["checkout", "-q", branch]);
        std::fs::write(outro.join(arquivo), texto).unwrap();
        sh(&f.git, &outro, &["add", "."]);
        sh(&f.git, &outro, &["commit", "-q", "-m", "do colega"]);
        sh(&f.git, &outro, &["push", "-q", "origin", branch]);
    }

    #[tokio::test]
    async fn pull_atualiza_a_branch_atual_por_fast_forward() {
        let Some(f) = fixture() else { return };
        colega_envia(&f, "develop", "novo.txt", "do colega\n");
        let r = pull_branch(&f.git, &f.work, &f.cred, "develop").await;
        assert!(r.as_ref().is_ok_and(|m| m.contains("+1 commit")), "{r:?}");
        assert_eq!(std::fs::read_to_string(f.work.join("novo.txt")).unwrap().replace("\r\n", "\n"), "do colega\n");
        assert_eq!(current_branch(&f.git, &f.work).await.as_deref(), Some("develop"));
    }

    #[tokio::test]
    async fn pull_atualiza_outra_branch_sem_trocar_de_branch() {
        let Some(f) = fixture() else { return };
        std::fs::write(f.work.join("a.txt"), "sujo\n").unwrap(); // alteração local não atrapalha outra branch
        colega_envia(&f, "prod", "p.txt", "p\n");
        let antes = sh(&f.git, &f.work, &["rev-parse", "prod"]);
        let r = pull_branch(&f.git, &f.work, &f.cred, "prod").await;
        assert!(r.as_ref().is_ok_and(|m| m.contains("+1 commit")), "{r:?}");
        assert_ne!(sh(&f.git, &f.work, &["rev-parse", "prod"]), antes);
        assert_eq!(sh(&f.git, &f.work, &["rev-parse", "prod"]), sh(&f.git, &f.bare, &["rev-parse", "prod"]));
        assert_eq!(current_branch(&f.git, &f.work).await.as_deref(), Some("develop"));
        assert_eq!(std::fs::read_to_string(f.work.join("a.txt")).unwrap(), "sujo\n");
    }

    #[tokio::test]
    async fn pull_recusa_quando_divergiu_e_nao_mexe_na_branch() {
        let Some(f) = fixture() else { return };
        colega_envia(&f, "develop", "x.txt", "remoto\n");
        std::fs::write(f.work.join("y.txt"), "local\n").unwrap();
        sh(&f.git, &f.work, &["add", "."]);
        sh(&f.git, &f.work, &["commit", "-q", "-m", "local"]);
        let antes = sh(&f.git, &f.work, &["rev-parse", "develop"]);
        let r = pull_branch(&f.git, &f.work, &f.cred, "develop").await;
        assert!(r.as_ref().is_err_and(|m| m.contains("divergiu")), "{r:?}");
        assert_eq!(sh(&f.git, &f.work, &["rev-parse", "develop"]), antes);
    }

    #[tokio::test]
    async fn pull_em_dia_ou_a_frente_nao_faz_nada() {
        let Some(f) = fixture() else { return };
        let r = pull_branch(&f.git, &f.work, &f.cred, "develop").await;
        assert!(r.as_ref().is_ok_and(|m| m.contains("em dia")), "{r:?}");
        std::fs::write(f.work.join("z.txt"), "z\n").unwrap();
        sh(&f.git, &f.work, &["add", "."]);
        sh(&f.git, &f.work, &["commit", "-q", "-m", "local"]);
        let r = pull_branch(&f.git, &f.work, &f.cred, "develop").await;
        assert!(r.as_ref().is_ok_and(|m| m.contains("à frente")), "{r:?}");
    }

    #[tokio::test]
    async fn commita_na_atual_avanca_a_que_esta_atras_e_envia_as_duas() {
        let Some(f) = fixture() else { return };
        std::fs::write(f.work.join("a.txt"), "dois\n").unwrap();
        let r = commit_branches(&f.git, &f.work, &["develop".into(), "prod".into()], "mudança", true, Some(&f.cred), &noprog()).await;
        assert_eq!(r.error, "");
        let by = |n: &str| r.branches.iter().find(|b| b.branch == n).unwrap().clone();
        let (dev, prod) = (by("develop"), by("prod"));
        assert_eq!((dev.status.as_str(), dev.pushed), ("commitado", Some(true)), "{dev:?}");
        assert_eq!((prod.status.as_str(), prod.pushed), ("atualizado", Some(true)), "{prod:?}");
        assert!(prod.detail.contains("Avançada até develop"));
        assert!(!dev.old_sha.is_empty() && dev.old_sha != dev.new_sha);
        // remoto e local iguais, e o usuário continua em develop
        let remote_dev = sh(&f.git, &f.bare, &["rev-parse", "develop"]);
        assert_eq!(remote_dev, dev.new_sha);
        assert_eq!(sh(&f.git, &f.bare, &["rev-parse", "prod"]), remote_dev);
        assert_eq!(current_branch(&f.git, &f.work).await.as_deref(), Some("develop"));
    }

    #[tokio::test]
    async fn pendencias_de_branch_nao_marcada_sao_guardadas_e_restauradas() {
        let Some(f) = fixture() else { return };
        std::fs::write(f.work.join("a.txt"), "sujo\n").unwrap();
        std::fs::write(f.work.join("novo.txt"), "novo\n").unwrap();
        let r = commit_branches(&f.git, &f.work, &["prod".into()], "x", false, None, &noprog()).await;
        assert_eq!(r.error, "");
        assert!(r.note.contains("preservadas"), "{r:?}");
        assert_eq!(r.branches[0].status, "sem_alteracoes");
        // voltou para develop com os arquivos sujos intactos
        assert_eq!(current_branch(&f.git, &f.work).await.as_deref(), Some("develop"));
        // o git do Windows pode converter LF em CRLF (core.autocrlf); o conteúdo é o mesmo
        assert_eq!(std::fs::read_to_string(f.work.join("a.txt")).unwrap().replace("\r\n", "\n"), "sujo\n");
        assert!(f.work.join("novo.txt").exists());
        assert_eq!(dirty_count(&f.git, &f.work).await, 2);
    }

    #[tokio::test]
    async fn replica_o_commit_em_outra_branch_sem_relacao() {
        let Some(f) = fixture() else { return };
        // prod e develop ganham commits próprios => divergem (sem ancestralidade)
        sh(&f.git, &f.work, &["checkout", "-q", "prod"]);
        std::fs::write(f.work.join("so-prod.txt"), "p\n").unwrap();
        sh(&f.git, &f.work, &["add", "."]);
        sh(&f.git, &f.work, &["commit", "-q", "-m", "so prod"]);
        sh(&f.git, &f.work, &["push", "-q", "origin", "prod"]);
        sh(&f.git, &f.work, &["checkout", "-q", "develop"]);
        std::fs::write(f.work.join("so-develop.txt"), "d\n").unwrap();
        sh(&f.git, &f.work, &["add", "."]);
        sh(&f.git, &f.work, &["commit", "-q", "-m", "so develop"]);
        sh(&f.git, &f.work, &["push", "-q", "origin", "develop"]);
        std::fs::write(f.work.join("b.txt"), "b\n").unwrap();
        let r = commit_branches(&f.git, &f.work, &["develop".into(), "prod".into()], "feat b", true, Some(&f.cred), &noprog()).await;
        let prod = r.branches.iter().find(|b| b.branch == "prod").unwrap();
        assert_eq!(prod.status, "commitado", "{prod:?}");
        assert!(prod.detail.contains("Replicado de develop"), "{prod:?}");
        assert_eq!(prod.pushed, Some(true));
    }

    #[tokio::test]
    async fn remoto_a_frente_e_trazido_por_fast_forward() {
        let Some(f) = fixture() else { return };
        // outro clone empurra um commit para develop
        let outro = f._t.path().join("outro");
        sh(&f.git, f._t.path(), &["clone", "-q", f.bare.to_str().unwrap(), outro.to_str().unwrap()]);
        sh(&f.git, &outro, &["config", "user.name", "O"]);
        sh(&f.git, &outro, &["config", "user.email", "o@o"]);
        sh(&f.git, &outro, &["checkout", "-q", "develop"]);
        std::fs::write(outro.join("remoto.txt"), "r\n").unwrap();
        sh(&f.git, &outro, &["add", "."]);
        sh(&f.git, &outro, &["commit", "-q", "-m", "do outro"]);
        sh(&f.git, &outro, &["push", "-q", "origin", "develop"]);

        std::fs::write(f.work.join("meu.txt"), "m\n").unwrap();
        let r = commit_branches(&f.git, &f.work, &["develop".into()], "meu", true, Some(&f.cred), &noprog()).await;
        let b = &r.branches[0];
        assert_eq!(b.status, "commitado", "{b:?}");
        assert!(b.detail.contains("Atualizada com o remoto"), "{b:?}");
        assert!(f.work.join("remoto.txt").exists(), "trouxe o commit do outro");
        assert_eq!(b.pushed, Some(true));
    }

    #[tokio::test]
    async fn divergencia_com_o_remoto_e_erro_sem_reescrever_historico() {
        let Some(f) = fixture() else { return };
        let outro = f._t.path().join("outro");
        sh(&f.git, f._t.path(), &["clone", "-q", f.bare.to_str().unwrap(), outro.to_str().unwrap()]);
        sh(&f.git, &outro, &["config", "user.name", "O"]);
        sh(&f.git, &outro, &["config", "user.email", "o@o"]);
        sh(&f.git, &outro, &["checkout", "-q", "develop"]);
        std::fs::write(outro.join("x.txt"), "x\n").unwrap();
        sh(&f.git, &outro, &["add", "."]);
        sh(&f.git, &outro, &["commit", "-q", "-m", "remoto"]);
        sh(&f.git, &outro, &["push", "-q", "origin", "develop"]);
        // commit local divergente
        std::fs::write(f.work.join("y.txt"), "y\n").unwrap();
        sh(&f.git, &f.work, &["add", "."]);
        sh(&f.git, &f.work, &["commit", "-q", "-m", "local"]);
        let antes = sh(&f.git, &f.work, &["rev-parse", "develop"]);
        let r = commit_branches(&f.git, &f.work, &["develop".into()], "z", true, Some(&f.cred), &noprog()).await;
        let b = &r.branches[0];
        assert_eq!(b.status, "erro");
        assert!(b.detail.contains("divergiu"), "{b:?}");
        assert_eq!(sh(&f.git, &f.work, &["rev-parse", "develop"]), antes, "histórico intacto");
    }

    #[tokio::test]
    async fn push_sem_credencial_e_erro_claro_e_head_destacado_tambem() {
        let Some(f) = fixture() else { return };
        let r = commit_branches(&f.git, &f.work, &["develop".into()], "x", true, None, &noprog()).await;
        assert!(r.error.contains("Sem credencial"));
        sh(&f.git, &f.work, &["checkout", "-q", "--detach"]);
        let r = commit_branches(&f.git, &f.work, &["develop".into()], "x", false, None, &noprog()).await;
        assert!(r.error.contains("detached"));
    }

    #[tokio::test]
    async fn estados_das_branches_frente_ao_remoto_e_irmas_atras() {
        let Some(f) = fixture() else { return };
        std::fs::write(f.work.join("n.txt"), "n\n").unwrap();
        sh(&f.git, &f.work, &["add", "."]);
        sh(&f.git, &f.work, &["commit", "-q", "-m", "mais um"]); // develop à frente do remoto e de prod
        let st = branch_states(&f.git, &f.work, &f.cred).await.unwrap();
        assert_eq!(st["develop"], "adiantado");
        assert_eq!(st["prod"], "ok");
        let br = git::local_branches(&f.git, &f.work).await;
        let irmas = behind_siblings(&f.git, &f.work, &br).await;
        assert_eq!(irmas["prod"], ["develop"]);
        assert!(!irmas.contains_key("develop"));
    }

    #[test]
    fn exclude_do_relatorio_e_idempotente() {
        let Some(f) = fixture() else { return };
        ensure_reports_ignored(&f.work);
        ensure_reports_ignored(&f.work);
        let txt = std::fs::read_to_string(f.work.join(".git/info/exclude")).unwrap();
        assert_eq!(txt.matches(".kiro/git-reports/").count(), 1);
    }
}
