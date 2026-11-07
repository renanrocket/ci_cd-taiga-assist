//! Erro único da aplicação. Serializa como texto simples para a UI exibir.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("banco de dados: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("chaveiro do sistema: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("aplicativo: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("{0}")]
    Msg(String),
}

impl AppError {
    pub fn msg(text: impl Into<String>) -> Self {
        AppError::Msg(text.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
