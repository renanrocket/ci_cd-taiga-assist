//! Segredos no chaveiro do SO (Keychain / Credential Manager / Secret Service).
//! O SQLite guarda só a referência (`secret_ref`). Nunca logar o valor.

use keyring::Entry;

use crate::error::AppResult;

const SERVICE: &str = "ci_cd-taiga-assist";

fn entry(secret_ref: &str) -> AppResult<Entry> {
    Ok(Entry::new(SERVICE, secret_ref)?)
}

pub fn set(secret_ref: &str, value: &str) -> AppResult<()> {
    entry(secret_ref)?.set_password(value)?;
    Ok(())
}

pub fn get(secret_ref: &str) -> AppResult<Option<String>> {
    match entry(secret_ref)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete(secret_ref: &str) -> AppResult<()> {
    match entry(secret_ref)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Referência nova e imprevisível para uma entrada do chaveiro.
pub fn new_ref(kind: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{kind}-{n:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Precisa de um chaveiro disponível (sessão gráfica); por isso é `#[ignore]` e roda com
    /// `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn ciclo_completo_no_chaveiro_real() {
        let r = new_ref("teste");
        assert_eq!(get(&r).unwrap(), None);
        set(&r, "segredo").unwrap();
        assert_eq!(get(&r).unwrap().as_deref(), Some("segredo"));
        delete(&r).unwrap();
        assert_eq!(get(&r).unwrap(), None);
    }

    #[test]
    fn refs_sao_distintas() {
        assert_ne!(new_ref("x"), new_ref("x"));
    }
}
