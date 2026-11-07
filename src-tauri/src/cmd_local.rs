//! Comandos Tauri da tela "Pasta local": status dos repositórios, commit/push em lote
//! (com relatório → US), reenvio de relatórios pendentes e tarefas em segundo plano.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::pipeline::{self, KiroCfg, Outcome, RepoCtx, TaigaCfg};
use crate::tasks::TaskInfo;
use crate::{git, gitcreds, gitops, report, secrets, taigadb, with_db, AppState};

#[derive(Serialize)]
pub struct BranchInfo {
    nome: String,
    atual: bool,
    /// Outras branches locais que estão à frente desta (ela será avançada até lá).
    atras_de: Vec<String>,
}

#[derive(Serialize)]
pub struct RepoStatus {
    rel: String,
    nome: String,
    atual: Option<String>,
    sujo: usize,
    branches: Vec<BranchInfo>,
    cred_id: Option<i64>,
    cred_nome: Option<String>,
    projeto: Option<String>,
    branch_gatilho: Option<String>,
    pendentes: usize,
    remote_url: Option<String>,
}

#[derive(Default)]
struct RepoCfg {
    repo_id: Option<i64>,
    cred_id: Option<i64>,
    project_id: Option<i64>,
    branch: Option<String>,
}

fn repo_cfg(conn: &Connection, folder_id: i64, rel: &str) -> AppResult<RepoCfg> {
    Ok(conn
        .query_row(
            "SELECT id, git_cred_id, project_id, branch_gatilho FROM local_repos WHERE folder_id = ?1 AND rel = ?2",
            rusqlite::params![folder_id, rel],
            |r| Ok(RepoCfg { repo_id: r.get(0)?, cred_id: r.get(1)?, project_id: r.get(2)?, branch: r.get(3)? }),
        )
        .optional()?
        .unwrap_or_default())
}

fn load_cred(app: &AppHandle, state: &AppState, cred_id: Option<i64>) -> AppResult<Option<gitops::RemoteCred>> {
    let Some(id) = cred_id else { return Ok(None) };
    let row = with_db(state, |c| gitcreds::get(c, id))?;
    let secret = match &row.secret_ref {
        Some(r) => Some(secrets::get(r)?.ok_or_else(|| AppError::msg("O segredo da credencial não está mais no chaveiro. Cadastre-a de novo."))?),
        None => None,
    };
    Ok(Some(gitops::RemoteCred {
        kind: row.kind,
        usuario: row.usuario,
        secret,
        known_hosts: git::known_hosts_path(&app.path().app_data_dir()?),
    }))
}

fn taiga_cfg(state: &AppState, project_id: i64) -> AppResult<TaigaCfg> {
    let (account_id, slug) = with_db(state, |c| taigadb::project_ref(c, project_id))?;
    let acc = with_db(state, |c| taigadb::get_account(c, account_id))?;
    let senha = secrets::get(&acc.secret_ref)?
        .ok_or_else(|| AppError::msg("A senha da conta Taiga não está mais no chaveiro. Cadastre a conta de novo."))?;
    Ok(TaigaCfg { host: acc.host, usuario: acc.usuario, senha, slug })
}

fn kiro_cfg(state: &AppState) -> AppResult<KiroCfg> {
    Ok(KiroCfg { bin: crate::cmd_kiro::kiro_bin(state)?, api_key: secrets::get(crate::cmd_kiro::API_KEY_REF)? })
}

fn prazo_dias(state: &AppState) -> i64 {
    with_db(state, |c| crate::db::get_setting(c, "prazo_dias"))
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|d| (0..=365).contains(d))
        .unwrap_or(report::PRAZO_DIAS_PADRAO)
}

fn folder_path(state: &AppState, folder_id: i64) -> AppResult<std::path::PathBuf> {
    with_db(state, |c| {
        c.query_row("SELECT caminho FROM local_folders WHERE id = ?1", [folder_id], |r| r.get::<_, String>(0))
            .optional()?
            .map(std::path::PathBuf::from)
            .ok_or_else(|| AppError::msg("Pasta não encontrada."))
    })
}

/// Visão local (sem rede) dos repositórios de uma pasta.
#[tauri::command]
pub async fn folder_overview(state: tauri::State<'_, AppState>, folder_id: i64) -> AppResult<Vec<RepoStatus>> {
    let git = crate::git_bin(&state)?;
    let base = folder_path(&state, folder_id)?;
    if !base.is_dir() {
        return Err(AppError::msg("A pasta cadastrada não existe mais no disco."));
    }
    let creds = with_db(&state, gitcreds::list)?;
    let projects = with_db(&state, taigadb::list_projects)?;
    let mut out = Vec::new();
    for repo in crate::folders::find_repos(&base) {
        let dir = if repo.rel == "." { base.clone() } else { base.join(&repo.rel) };
        let cfg = with_db(&state, |c| repo_cfg(c, folder_id, &repo.rel))?;
        let names = git::local_branches(&git, &dir).await;
        let atual = gitops::current_branch(&git, &dir).await;
        let irmas = gitops::behind_siblings(&git, &dir, &names).await;
        let branches = names
            .iter()
            .map(|n| BranchInfo { nome: n.clone(), atual: atual.as_deref() == Some(n.as_str()), atras_de: irmas.get(n).cloned().unwrap_or_default() })
            .collect();
        out.push(RepoStatus {
            nome: repo.nome.clone(),
            sujo: gitops::dirty_count(&git, &dir).await,
            pendentes: report::pending_files(&dir).len(),
            remote_url: git::remote_url(&git, &dir).await,
            cred_nome: cfg.cred_id.and_then(|id| creds.iter().find(|c| c.id == id)).map(|c| c.nome.clone()),
            projeto: cfg.project_id.and_then(|id| projects.iter().find(|p| p.id == id)).map(|p| format!("{} ({})", p.nome, p.account_nome)),
            cred_id: cfg.cred_id,
            branch_gatilho: cfg.branch,
            rel: repo.rel,
            atual,
            branches,
        });
    }
    Ok(out)
}

/// Estado de cada branch frente ao remoto (1 `ls-remote`; carregado depois, sem travar a tela).
#[tauri::command]
pub async fn remote_states(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    folder_id: i64,
    rel: String,
) -> AppResult<HashMap<String, String>> {
    let git = crate::git_bin(&state)?;
    let dir = with_db(&state, |c| gitcreds::repo_path(c, folder_id, &rel))?;
    let cfg = with_db(&state, |c| repo_cfg(c, folder_id, &rel))?;
    let cred = load_cred(&app, &state, cfg.cred_id)?.ok_or_else(|| AppError::msg("Sem credencial para consultar o remoto."))?;
    let states = gitops::branch_states(&git, &dir, &cred).await.map_err(AppError::msg)?;
    Ok(states.into_iter().map(|(k, v)| (k, v.to_owned())).collect())
}

/// "Atualizar" uma branch a partir do remoto (git pull só fast-forward). Rápido: roda direto, sem tarefa em segundo plano.
#[tauri::command]
pub async fn update_branch(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    folder_id: i64,
    rel: String,
    branch: String,
) -> AppResult<String> {
    let git = crate::git_bin(&state)?;
    let dir = with_db(&state, |c| gitcreds::repo_path(c, folder_id, &rel))?;
    let cfg = with_db(&state, |c| repo_cfg(c, folder_id, &rel))?;
    let cred = load_cred(&app, &state, cfg.cred_id)?.ok_or_else(|| AppError::msg("Sem credencial para consultar o remoto."))?;
    gitops::pull_branch(&git, &dir, &cred, &branch).await.map_err(AppError::msg)
}

#[derive(Deserialize)]
pub struct CommitItem {
    pub rel: String,
    pub branches: Vec<String>,
    /// Gerar relatório e US no Taiga (se o repositório tiver projeto + branch gatilho).
    pub taiga: bool,
}

#[tauri::command]
pub async fn start_commit(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    folder_id: i64,
    items: Vec<CommitItem>,
    message: String,
    push: bool,
) -> AppResult<String> {
    let items: Vec<CommitItem> = items.into_iter().filter(|i| !i.branches.is_empty()).collect();
    if items.is_empty() {
        return Err(AppError::msg("Selecione ao menos uma branch."));
    }
    let message = message.trim().to_owned();
    let git = crate::git_bin(&state)?;
    for it in &items {
        let dir = with_db(&state, |c| gitcreds::repo_path(c, folder_id, &it.rel))?;
        // A mensagem só é obrigatória quando há alterações pendentes na branch marcada.
        if message.is_empty() {
            let atual = gitops::current_branch(&git, &dir).await;
            if gitops::dirty_count(&git, &dir).await > 0 && atual.is_some_and(|a| it.branches.contains(&a)) {
                return Err(AppError::msg("Informe a mensagem do commit (há alterações pendentes)."));
            }
        }
        if push && with_db(&state, |c| repo_cfg(c, folder_id, &it.rel))?.cred_id.is_none() {
            return Err(AppError::msg(format!("“{}” não tem credencial para o push. Vincule uma em Configurações.", it.rel)));
        }
    }
    let pasta = folder_path(&state, folder_id)?.display().to_string();
    let id = state.tasks.start(&format!("commit:{folder_id}"), &format!("Commit em {pasta}")).map_err(AppError::msg)?;
    let (app2, id2) = (app.clone(), id.clone());
    tauri::async_runtime::spawn(async move { commit_task(app2, id2, folder_id, items, message, push).await });
    Ok(id)
}

async fn commit_task(app: AppHandle, tid: String, folder_id: i64, items: Vec<CommitItem>, message: String, push: bool) {
    let state = app.state::<AppState>();
    let tasks = &state.tasks;
    let progress = |m: String| tasks.step(&tid, &m);
    let git = match crate::git_bin(&state) {
        Ok(g) => g,
        Err(e) => return tasks.finish(&tid, false, &e.to_string(), None),
    };
    let dias = prazo_dias(&state);
    let today = chrono::Local::now().date_naive();
    let mut results: Vec<gitops::RepoResult> = Vec::new();

    for it in items {
        let dir = match with_db(&state, |c| gitcreds::repo_path(c, folder_id, &it.rel)) {
            Ok(d) => d,
            Err(e) => {
                results.push(gitops::RepoResult { name: it.rel.clone(), error: e.to_string(), ..Default::default() });
                continue;
            }
        };
        let cfg = with_db(&state, |c| repo_cfg(c, folder_id, &it.rel)).unwrap_or_default();
        let cred = match load_cred(&app, &state, cfg.cred_id) {
            Ok(c) => c,
            Err(e) => {
                let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                results.push(gitops::RepoResult { name, error: e.to_string(), ..Default::default() });
                continue;
            }
        };
        let mut res = gitops::commit_branches(&git, &dir, &it.branches, &message, push, cred.as_ref(), &progress).await;

        if let (true, Some(project_id), Some(gatilho), Some(repo_id)) = (it.taiga, cfg.project_id, cfg.branch.clone(), cfg.repo_id) {
            for br in res.branches.iter_mut() {
                // Só a branch gatilho, e só se o push foi ok e trouxe commit novo.
                if br.branch != gatilho || br.pushed != Some(true) || br.new_sha.is_empty() || br.new_sha == br.old_sha {
                    continue;
                }
                let ctx = RepoCtx { git: git.clone(), dir: dir.clone(), repo_id, project_id, label: res.name.clone() };
                let out: Outcome = match (taiga_cfg(&state, project_id), kiro_cfg(&state)) {
                    (Ok(t), Ok(k)) => pipeline::run_for_push(&state.db, &ctx, &t, &k, &br.branch, &br.old_sha, &br.new_sha, dias, today, &progress).await,
                    (Err(e), _) | (_, Err(e)) => Outcome { ok: false, detail: e.to_string() },
                };
                br.taiga = out.detail;
                br.taiga_ok = Some(out.ok);
            }
        }
        results.push(res);
    }

    let bad = results
        .iter()
        .filter(|r| !r.error.is_empty() || r.branches.iter().any(|b| b.status == "erro" || b.pushed == Some(false) || b.taiga_ok == Some(false)))
        .count();
    let n: usize = results.iter().map(|r| r.branches.len()).sum();
    let msg = if bad > 0 { format!("{bad} repositório(s) com problema; veja o resultado.") } else { format!("{n} branch(es) processada(s).") };
    tasks.finish(&tid, bad == 0, &msg, serde_json::to_value(&results).ok());
}

/// Reenvia ao Taiga os relatórios que ficaram em `.kiro/git-reports` (falhas anteriores).
#[tauri::command]
pub async fn start_resend(app: AppHandle, state: tauri::State<'_, AppState>, folder_id: i64, rel: String) -> AppResult<String> {
    let dir = with_db(&state, |c| gitcreds::repo_path(c, folder_id, &rel))?;
    let cfg = with_db(&state, |c| repo_cfg(c, folder_id, &rel))?;
    let (Some(project_id), Some(branch), Some(repo_id)) = (cfg.project_id, cfg.branch, cfg.repo_id) else {
        return Err(AppError::msg("Este repositório não tem projeto Taiga e branch gatilho configurados."));
    };
    let git = crate::git_bin(&state)?;
    let nome = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let id = state.tasks.start(&format!("resend:{folder_id}:{rel}"), &format!("Reenvio ao Taiga · {nome}")).map_err(AppError::msg)?;
    let (app2, id2) = (app.clone(), id.clone());
    tauri::async_runtime::spawn(async move {
        let state = app2.state::<AppState>();
        state.tasks.step(&id2, "Reenviando relatórios pendentes");
        let ctx = RepoCtx { git, dir, repo_id, project_id, label: nome };
        let (dias, today) = (prazo_dias(&state), chrono::Local::now().date_naive());
        let done = match taiga_cfg(&state, project_id) {
            Ok(t) => pipeline::resend_pending(&state.db, &ctx, &t, &branch, dias, today).await,
            Err(e) => return state.tasks.finish(&id2, false, &e.to_string(), None),
        };
        if done.is_empty() {
            return state.tasks.finish(&id2, true, "Nenhum relatório pendente em .kiro/git-reports.", None);
        }
        let all_ok = done.iter().all(|(_, o)| o.ok);
        let msg = done.iter().map(|(n, o)| format!("{n}: {}", o.detail)).collect::<Vec<_>>().join(" · ");
        let result = serde_json::json!(done.iter().map(|(n, o)| serde_json::json!({"arquivo": n, "ok": o.ok, "detail": o.detail})).collect::<Vec<_>>());
        state.tasks.finish(&id2, all_ok, &msg, Some(result));
    });
    Ok(id)
}

#[tauri::command]
pub fn list_tasks(state: tauri::State<'_, AppState>) -> Vec<TaskInfo> {
    state.tasks.list()
}

#[tauri::command]
pub fn dismiss_task(state: tauri::State<'_, AppState>, id: String) {
    state.tasks.dismiss(&id);
}

/// Prazo padrão (dias corridos) da US criada a partir do relatório.
#[tauri::command]
pub fn get_prazo_dias(state: tauri::State<'_, AppState>) -> i64 {
    prazo_dias(&state)
}

#[tauri::command]
pub fn set_prazo_dias(state: tauri::State<'_, AppState>, dias: i64) -> AppResult<i64> {
    if !(0..=365).contains(&dias) {
        return Err(AppError::msg("O prazo precisa estar entre 0 e 365 dias."));
    }
    with_db(&state, |c| crate::db::set_setting(c, "prazo_dias", &dias.to_string()))?;
    Ok(dias)
}
