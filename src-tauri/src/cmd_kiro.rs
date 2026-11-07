//! Comandos Tauri do passo "Kiro": login, API key opcional e teste headless.

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::kiro;
use crate::tools::{self, Tool};
use crate::{secrets, with_db, AppState, TestResult};

/// Referência fixa: existe no máximo uma API key do Kiro por instalação.
pub(crate) const API_KEY_REF: &str = "kiro-api-key";

pub(crate) fn kiro_bin(state: &AppState) -> AppResult<std::path::PathBuf> {
    let custom = with_db(state, |c| crate::db::get_setting(c, Tool::Kiro.setting_key()))?;
    tools::locate(Tool::Kiro, custom.as_deref())
        .map(|(p, _)| p)
        .ok_or_else(|| AppError::msg("Kiro CLI não encontrado. Volte ao passo 1 e informe o caminho."))
}

#[derive(Serialize)]
pub struct KiroStatus {
    pub login: kiro::Login,
    pub api_key: bool,
}

async fn status(state: &AppState) -> AppResult<KiroStatus> {
    let bin = kiro_bin(state)?;
    let key = secrets::get(API_KEY_REF)?;
    let login = kiro::login_status(&bin, key.as_deref()).await;
    Ok(KiroStatus { login, api_key: key.is_some() })
}

#[tauri::command]
pub async fn kiro_status(state: tauri::State<'_, AppState>) -> AppResult<KiroStatus> {
    status(&state).await
}

#[tauri::command]
pub async fn save_kiro_api_key(state: tauri::State<'_, AppState>, key: String) -> AppResult<KiroStatus> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::msg("Cole a API key do Kiro."));
    }
    secrets::set(API_KEY_REF, key)?;
    status(&state).await
}

#[tauri::command]
pub async fn clear_kiro_api_key(state: tauri::State<'_, AppState>) -> AppResult<KiroStatus> {
    secrets::delete(API_KEY_REF)?;
    status(&state).await
}

/// Executa um prompt mínimo de ponta a ponta (gasta ~0,05 crédito).
#[tauri::command]
pub async fn test_kiro(state: tauri::State<'_, AppState>) -> AppResult<TestResult> {
    let bin = kiro_bin(&state)?;
    let key = secrets::get(API_KEY_REF)?;
    Ok(match kiro::headless_test(&bin, key.as_deref()).await {
        Ok(detail) => TestResult { ok: true, detail },
        Err(detail) => TestResult { ok: false, detail },
    })
}

/// Inicia o login do Kiro (código de dispositivo) como tarefa em segundo plano: cada linha que o
/// kiro-cli imprime vira uma etapa na caixa de notificações (a UI extrai o link e o código).
#[tauri::command]
pub async fn kiro_login_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    license: String,
    idp: Option<String>,
    region: Option<String>,
) -> AppResult<String> {
    use tauri::Manager;
    let bin = kiro_bin(&state)?;
    let mut args: Vec<String> = vec!["login".into(), "--license".into(), license.clone(), "--use-device-flow".into()];
    match license.as_str() {
        "free" => {}
        "pro" => {
            let idp = idp.unwrap_or_default().trim().to_owned();
            let region = region.unwrap_or_default().trim().to_owned();
            if !idp.starts_with("https://") {
                return Err(AppError::msg("Informe o endereço do portal da empresa (começa com https://)."));
            }
            if region.is_empty() || !region.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(AppError::msg("Informe a região (ex.: us-east-1)."));
            }
            args.extend(["--identity-provider".into(), idp, "--region".into(), region]);
        }
        _ => return Err(AppError::msg("Tipo de conta inválido.")),
    }
    let key = secrets::get(API_KEY_REF)?;
    if kiro::login_status(&bin, key.as_deref()).await.logged_in {
        return Err(AppError::msg("Você já está logado no Kiro."));
    }
    let id = state.tasks.start("kiro-login", "Login no Kiro").map_err(AppError::msg)?;
    let id2 = id.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        state.tasks.step(&id2, "Aguardando o login — abra a página indicada e confirme o código.");
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let on_line = |l: String| state.tasks.step(&id2, &l);
        let res = kiro::run_streaming(&bin, &refs, &on_line, std::time::Duration::from_secs(600)).await;
        let after = kiro::login_status(&bin, key.as_deref()).await;
        match (res, after.logged_in) {
            (_, true) => {
                let quem = after.who.and_then(|w| w.email).unwrap_or_else(|| "conta Kiro".into());
                state.tasks.finish(&id2, true, &format!("Login concluído: {quem}."), None)
            }
            (Err(e), false) => state.tasks.finish(&id2, false, &e, None),
            (Ok(_), false) => state.tasks.finish(&id2, false, "O login não foi concluído. Tente de novo.", None),
        }
    });
    Ok(id)
}
