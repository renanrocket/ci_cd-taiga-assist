mod cards;
mod cmd_cards;
mod cmd_kiro;
mod cmd_local;
mod cmd_taiga;
mod db;
mod error;
mod export;
mod folders;
mod git;
mod gitcreds;
mod gitops;
mod kiro;
mod pipeline;
mod proc;
mod report;
mod secrets;
mod summary;
mod taiga;
mod taigadb;
mod tasks;
mod tools;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;

use crate::error::{AppError, AppResult};
use crate::tools::{Tool, ToolStatus};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub tasks: tasks::Tasks,
}

fn parse_tool(name: &str) -> AppResult<Tool> {
    match name {
        "git" => Ok(Tool::Git),
        "kiro" => Ok(Tool::Kiro),
        other => Err(AppError::msg(format!("ferramenta desconhecida: {other}"))),
    }
}

fn custom_path(state: &AppState, tool: Tool) -> AppResult<Option<String>> {
    let conn = state.db.lock().map_err(|_| AppError::msg("banco ocupado"))?;
    db::get_setting(&conn, tool.setting_key())
}

/// Pré-requisitos do wizard: git e kiro-cli encontrados? versão?
#[tauri::command]
async fn check_prerequisites(state: tauri::State<'_, AppState>) -> AppResult<Vec<ToolStatus>> {
    let mut out = Vec::new();
    for tool in [Tool::Git, Tool::Kiro] {
        let custom = custom_path(&state, tool)?;
        out.push(tools::status(tool, custom.as_deref()).await);
    }
    Ok(out)
}

/// Define (ou limpa, com texto vazio) o caminho de uma ferramenta e devolve o novo status.
#[tauri::command]
async fn set_tool_path(
    state: tauri::State<'_, AppState>,
    tool: String,
    path: String,
) -> AppResult<ToolStatus> {
    let tool = parse_tool(&tool)?;
    {
        let conn = state.db.lock().map_err(|_| AppError::msg("banco ocupado"))?;
        if path.trim().is_empty() {
            db::clear_setting(&conn, tool.setting_key())?;
        } else {
            db::set_setting(&conn, tool.setting_key(), path.trim())?;
        }
    }
    let custom = custom_path(&state, tool)?;
    Ok(tools::status(tool, custom.as_deref()).await)
}

pub(crate) fn with_db<T>(state: &AppState, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
    let conn = state.db.lock().map_err(|_| AppError::msg("banco ocupado"))?;
    f(&conn)
}

#[tauri::command]
fn list_folders(state: tauri::State<'_, AppState>) -> AppResult<Vec<folders::FolderInfo>> {
    with_db(&state, folders::list)
}

#[tauri::command]
fn add_folder(state: tauri::State<'_, AppState>, path: String) -> AppResult<Vec<folders::FolderInfo>> {
    with_db(&state, |c| {
        folders::add(c, &path)?;
        folders::list(c)
    })
}

#[tauri::command]
fn remove_folder(state: tauri::State<'_, AppState>, id: i64) -> AppResult<Vec<folders::FolderInfo>> {
    with_db(&state, |c| {
        folders::remove(c, id)?;
        folders::list(c)
    })
}

pub(crate) fn git_bin(state: &AppState) -> AppResult<std::path::PathBuf> {
    let custom = custom_path(state, Tool::Git)?;
    tools::locate(Tool::Git, custom.as_deref())
        .map(|(p, _)| p)
        .ok_or_else(|| AppError::msg("Git não encontrado. Volte ao passo 1 e informe o caminho."))
}

#[tauri::command]
fn list_git_credentials(state: tauri::State<'_, AppState>) -> AppResult<Vec<gitcreds::CredInfo>> {
    with_db(&state, gitcreds::list)
}

#[tauri::command]
fn add_git_credential(
    state: tauri::State<'_, AppState>,
    nome: String,
    tipo: String,
    usuario: Option<String>,
    segredo: Option<String>,
) -> AppResult<Vec<gitcreds::CredInfo>> {
    let kind = git::Kind::parse(&tipo)?;
    gitcreds::validate(&nome, kind, usuario.as_deref(), segredo.as_deref())?;
    // Segredo primeiro no chaveiro; se gravar no banco falhar, desfaz o chaveiro.
    let secret_ref = if kind == git::Kind::Sistema {
        None
    } else {
        let r = secrets::new_ref("git");
        secrets::set(&r, segredo.as_deref().unwrap_or_default().trim_end_matches(['\r', '\n']))?;
        Some(r)
    };
    let res = with_db(&state, |c| {
        gitcreds::insert(c, &nome, kind, usuario.as_deref(), secret_ref.as_deref())?;
        gitcreds::list(c)
    });
    if res.is_err() {
        if let Some(r) = &secret_ref {
            let _ = secrets::delete(r);
        }
    }
    res
}

#[tauri::command]
fn remove_git_credential(state: tauri::State<'_, AppState>, id: i64) -> AppResult<Vec<gitcreds::CredInfo>> {
    let secret_ref = with_db(&state, |c| gitcreds::delete(c, id))?;
    if let Some(r) = secret_ref {
        secrets::delete(&r)?;
    }
    with_db(&state, gitcreds::list)
}

#[tauri::command]
async fn list_repo_links(state: tauri::State<'_, AppState>) -> AppResult<Vec<gitcreds::RepoLink>> {
    let mut links = with_db(&state, gitcreds::links)?;
    if let Ok(git) = git_bin(&state) {
        for l in &mut links {
            let dir = std::path::Path::new(&l.folder);
            let dir = if l.rel == "." { dir.to_path_buf() } else { dir.join(&l.rel) };
            l.remote_url = git::remote_url(&git, &dir).await;
        }
    }
    Ok(links)
}

#[tauri::command]
fn set_repo_credential(
    state: tauri::State<'_, AppState>,
    folder_id: i64,
    rel: String,
    cred_id: Option<i64>,
) -> AppResult<()> {
    with_db(&state, |c| {
        gitcreds::repo_path(c, folder_id, &rel)?; // valida o repositório
        gitcreds::set_repo_credential(c, folder_id, &rel, cred_id)
    })
}

#[derive(serde::Serialize)]
pub(crate) struct TestResult {
    pub ok: bool,
    pub detail: String,
}

/// Testa o acesso ao remoto `origin` do repositório com a credencial vinculada a ele.
#[tauri::command]
async fn test_repo_access(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    folder_id: i64,
    rel: String,
) -> AppResult<TestResult> {
    let git = git_bin(&state)?;
    let (dir, cred) = with_db(&state, |c| {
        let dir = gitcreds::repo_path(c, folder_id, &rel)?;
        let cred_id: Option<i64> = c
            .query_row(
                "SELECT git_cred_id FROM local_repos WHERE folder_id = ?1 AND rel = ?2",
                rusqlite::params![folder_id, rel],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        let cred = cred_id.map(|id| gitcreds::get(c, id)).transpose()?;
        Ok((dir, cred))
    })?;
    let fail = |detail: &str| Ok(TestResult { ok: false, detail: detail.to_owned() });
    let Some(cred) = cred else {
        return fail("Escolha uma credencial para este repositório antes de testar.");
    };
    let Some(url) = git::remote_url(&git, &dir).await else {
        return fail("Este repositório não tem o remote 'origin'.");
    };
    let secret = match &cred.secret_ref {
        Some(r) => match secrets::get(r)? {
            Some(s) => Some(s),
            None => return fail("O segredo desta credencial não está mais no chaveiro. Cadastre-a de novo."),
        },
        None => None,
    };
    let known_hosts = git::known_hosts_path(&app.path().app_data_dir()?);
    let auth = git::build_auth(cred.kind, cred.usuario.as_deref(), secret.as_deref(), &url, &known_hosts)?;
    Ok(match git::test_remote(&git, &auth).await {
        Ok(detail) => TestResult { ok: true, detail },
        Err(detail) => TestResult { ok: false, detail },
    })
}

/// Como esta instalação recebe atualizações: `auto` (o app baixa e instala) ou `manual` (só avisa e
/// leva à página de download). No Linux, o atualizador só consegue trocar um AppImage; um `.deb`
/// instalado pelo apt não pode ser substituído pelo próprio app.
fn update_mode_for(os: &str, appimage_env: Option<&str>) -> &'static str {
    if os == "linux" && appimage_env.map_or(true, str::is_empty) {
        "manual"
    } else {
        "auto"
    }
}

#[tauri::command]
fn update_mode() -> &'static str {
    update_mode_for(std::env::consts::OS, std::env::var("APPIMAGE").ok().as_deref())
}

/// Pasta onde ficam os arquivos de log (para o usuário anexar num pedido de ajuda).
#[tauri::command]
fn logs_dir(app: tauri::AppHandle) -> AppResult<String> {
    Ok(app.path().app_log_dir()?.display().to_string())
}

#[tauri::command]
fn wizard_done(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    with_db(&state, |c| Ok(db::get_setting(c, "wizard.done")?.as_deref() == Some("1")))
}

#[tauri::command]
fn set_wizard_done(state: tauri::State<'_, AppState>, done: bool) -> AppResult<()> {
    with_db(&state, |c| if done { db::set_setting(c, "wizard.done", "1") } else { db::clear_setting(c, "wizard.done") })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Uma única janela por usuário (dois processos no mesmo SQLite dariam conflito). Deve ser o 1º plugin.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        // Log em arquivo (pasta de logs do sistema) para ajudar no suporte. Nunca logar segredos.
        .plugin(tauri_plugin_log::Builder::new().level(tauri_plugin_log::log::LevelFilter::Info).build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            tauri_plugin_log::log::info!("Taiga Assist {} iniciado ({})", env!("CARGO_PKG_VERSION"), std::env::consts::OS);
            let conn = db::open(&dir.join("taiga-assist.sqlite3"))?;
            app.manage(AppState { db: Mutex::new(conn), tasks: tasks::Tasks::new(app.handle().clone()) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_prerequisites,
            set_tool_path,
            list_folders,
            add_folder,
            remove_folder,
            list_git_credentials,
            add_git_credential,
            remove_git_credential,
            list_repo_links,
            set_repo_credential,
            test_repo_access,
            cmd_taiga::list_taiga_accounts,
            cmd_taiga::add_taiga_account,
            cmd_taiga::remove_taiga_account,
            cmd_taiga::fetch_remote_projects,
            cmd_taiga::import_taiga_projects,
            cmd_taiga::list_taiga_projects,
            cmd_taiga::remove_taiga_project,
            cmd_taiga::list_repo_branches,
            cmd_taiga::set_repo_taiga,
            cmd_taiga::test_repo_taiga,
            wizard_done,
            update_mode,
            logs_dir,
            set_wizard_done,
            cmd_kiro::kiro_status,
            cmd_kiro::save_kiro_api_key,
            cmd_kiro::clear_kiro_api_key,
            cmd_kiro::test_kiro,
            cmd_kiro::kiro_login_start,
            cmd_local::get_prazo_dias,
            cmd_local::set_prazo_dias,
            cmd_local::folder_overview,
            cmd_local::remote_states,
            cmd_local::update_branch,
            cmd_local::start_commit,
            cmd_local::start_resend,
            cmd_local::list_tasks,
            cmd_local::dismiss_task,
            cmd_cards::cards_list,
            cmd_cards::cards_prepare,
            cmd_cards::cards_review,
            cmd_cards::cards_close,
            cmd_cards::prestacao_list,
            cmd_cards::prestacao_pendentes,
            cmd_cards::prestar,
            cmd_cards::desfazer,
            cmd_cards::toggle_item
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o aplicativo");
}

#[cfg(test)]
mod tests {
    use super::update_mode_for;

    #[test]
    fn linux_so_atualiza_sozinho_como_appimage() {
        assert_eq!(update_mode_for("linux", Some("/tmp/app.AppImage")), "auto");
        assert_eq!(update_mode_for("linux", None), "manual", ".deb / execução avulsa");
        assert_eq!(update_mode_for("linux", Some("")), "manual");
        assert_eq!(update_mode_for("windows", None), "auto");
        assert_eq!(update_mode_for("macos", None), "auto");
    }
}
