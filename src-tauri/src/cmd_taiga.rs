//! Comandos Tauri do passo "Taiga": contas, projetos importados e vínculo repositório → projeto.

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::taiga::{Client, RemoteProject};
use crate::taigadb::{self, AccountInfo, ProjectInfo};
use crate::{git, gitcreds, secrets, with_db, AppState, TestResult};

/// Cliente já autenticado para uma conta cadastrada. A senha vem do chaveiro, só em memória.
async fn client_for(state: &AppState, account_id: i64) -> AppResult<Client> {
    let acc = with_db(state, |c| taigadb::get_account(c, account_id))?;
    let pass = secrets::get(&acc.secret_ref)?
        .ok_or_else(|| AppError::msg("A senha desta conta não está mais no chaveiro. Cadastre a conta de novo."))?;
    let mut client = Client::new(&acc.host, &acc.usuario, &pass)?;
    client.login().await?;
    Ok(client)
}

#[tauri::command]
pub fn list_taiga_accounts(state: tauri::State<'_, AppState>) -> AppResult<Vec<AccountInfo>> {
    with_db(&state, taigadb::list_accounts)
}

/// Só cadastra se o login funcionar: nunca guardamos uma conta que já nasce quebrada.
#[tauri::command]
pub async fn add_taiga_account(
    state: tauri::State<'_, AppState>,
    nome: String,
    host: String,
    usuario: String,
    senha: String,
) -> AppResult<Vec<AccountInfo>> {
    taigadb::validate_account(&nome, &host, &usuario, &senha)?;
    let mut client = Client::new(&host, usuario.trim(), &senha)?;
    client.login().await?;

    let secret_ref = secrets::new_ref("taiga");
    secrets::set(&secret_ref, &senha)?;
    let res = with_db(&state, |c| {
        taigadb::insert_account(c, &nome, &host, &usuario, &secret_ref)?;
        taigadb::list_accounts(c)
    });
    if res.is_err() {
        let _ = secrets::delete(&secret_ref);
    }
    res
}

#[tauri::command]
pub fn remove_taiga_account(state: tauri::State<'_, AppState>, id: i64) -> AppResult<Vec<AccountInfo>> {
    let secret_ref = with_db(&state, |c| taigadb::delete_account(c, id))?;
    secrets::delete(&secret_ref)?;
    with_db(&state, taigadb::list_accounts)
}

#[derive(Serialize)]
pub struct RemoteProjectSel {
    slug: String,
    nome: String,
    importado: bool,
}

/// Projetos de que o usuário é membro no Taiga, marcando os que já foram importados.
#[tauri::command]
pub async fn fetch_remote_projects(state: tauri::State<'_, AppState>, account_id: i64) -> AppResult<Vec<RemoteProjectSel>> {
    let remote = client_for(&state, account_id).await?.my_projects().await?;
    let have: Vec<String> = with_db(&state, taigadb::list_projects)?
        .into_iter()
        .filter(|p| p.account_id == account_id)
        .map(|p| p.slug)
        .collect();
    Ok(remote
        .into_iter()
        .map(|p| RemoteProjectSel { importado: have.contains(&p.slug), slug: p.slug, nome: p.nome })
        .collect())
}

/// Importa os slugs escolhidos. Os nomes são buscados de novo no Taiga (não confia na UI).
#[tauri::command]
pub async fn import_taiga_projects(
    state: tauri::State<'_, AppState>,
    account_id: i64,
    slugs: Vec<String>,
) -> AppResult<Vec<ProjectInfo>> {
    let remote: Vec<RemoteProject> = client_for(&state, account_id)
        .await?
        .my_projects()
        .await?
        .into_iter()
        .filter(|p| slugs.contains(&p.slug))
        .collect();
    with_db(&state, |c| {
        taigadb::add_projects(c, account_id, &remote)?;
        taigadb::list_projects(c)
    })
}

#[tauri::command]
pub fn list_taiga_projects(state: tauri::State<'_, AppState>) -> AppResult<Vec<ProjectInfo>> {
    with_db(&state, taigadb::list_projects)
}

#[tauri::command]
pub fn remove_taiga_project(state: tauri::State<'_, AppState>, id: i64) -> AppResult<Vec<ProjectInfo>> {
    with_db(&state, |c| {
        taigadb::remove_project(c, id)?;
        taigadb::list_projects(c)
    })
}

#[tauri::command]
pub async fn list_repo_branches(state: tauri::State<'_, AppState>, folder_id: i64, rel: String) -> AppResult<Vec<String>> {
    let git = crate::git_bin(&state)?;
    let dir = with_db(&state, |c| gitcreds::repo_path(c, folder_id, &rel))?;
    Ok(git::local_branches(&git, &dir).await)
}

#[tauri::command]
pub async fn set_repo_taiga(
    state: tauri::State<'_, AppState>,
    folder_id: i64,
    rel: String,
    project_id: Option<i64>,
    branch: Option<String>,
) -> AppResult<()> {
    let dir = with_db(&state, |c| gitcreds::repo_path(c, folder_id, &rel))?;
    if let Some(b) = branch.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
        let git = crate::git_bin(&state)?;
        if !git::local_branches(&git, &dir).await.iter().any(|x| x == b) {
            return Err(AppError::msg(format!("A branch '{b}' não existe neste repositório.")));
        }
    }
    with_db(&state, |c| taigadb::set_repo_taiga(c, folder_id, &rel, project_id, branch.as_deref()))
}

/// Login + leitura do projeto vinculado ao repositório (o mesmo "preflight" do ci_cd).
#[tauri::command]
pub async fn test_repo_taiga(state: tauri::State<'_, AppState>, folder_id: i64, rel: String) -> AppResult<TestResult> {
    let project_id: Option<i64> = with_db(&state, |c| {
        gitcreds::repo_path(c, folder_id, &rel)?;
        Ok(c.query_row(
            "SELECT project_id FROM local_repos WHERE folder_id = ?1 AND rel = ?2",
            rusqlite::params![folder_id, rel],
            |r| r.get::<_, Option<i64>>(0),
        )
        .ok()
        .flatten())
    })?;
    let Some(project_id) = project_id else {
        return Ok(TestResult { ok: false, detail: "Escolha o projeto Taiga deste repositório antes de testar.".into() });
    };
    let (account_id, slug) = with_db(&state, |c| taigadb::project_ref(c, project_id))?;
    let result = async {
        let client = client_for(&state, account_id).await?;
        let proj = client.project(&slug).await?;
        let nome = proj.get("name").and_then(|v| v.as_str()).unwrap_or(&slug).to_owned();
        Ok::<_, AppError>(format!("Projeto “{nome}” acessível."))
    }
    .await;
    Ok(match result {
        Ok(detail) => TestResult { ok: true, detail },
        Err(e) => TestResult { ok: false, detail: e.to_string() },
    })
}
