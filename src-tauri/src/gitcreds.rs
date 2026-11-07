//! Credenciais git (metadados no SQLite, segredo no chaveiro) e o vínculo repositório → credencial.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::git::Kind;

#[derive(Debug, Serialize)]
pub struct CredInfo {
    pub id: i64,
    pub nome: String,
    pub tipo: String,
    pub usuario: Option<String>,
}

/// Credencial completa, só para uso interno (nunca serializada para a UI).
pub struct CredRow {
    pub kind: Kind,
    pub usuario: Option<String>,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepoLink {
    pub folder_id: i64,
    pub folder: String,
    pub rel: String,
    pub nome: String,
    pub cred_id: Option<i64>,
    pub project_id: Option<i64>,
    pub branch: Option<String>,
    pub remote_url: Option<String>,
}

pub fn list(conn: &Connection) -> AppResult<Vec<CredInfo>> {
    let mut stmt = conn.prepare("SELECT id, nome, tipo, usuario FROM git_credentials ORDER BY nome COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| {
        Ok(CredInfo { id: r.get(0)?, nome: r.get(1)?, tipo: r.get(2)?, usuario: r.get(3)? })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<CredRow> {
    conn.query_row("SELECT id, tipo, usuario, secret_ref FROM git_credentials WHERE id = ?1", [id], |r| {
        Ok((r.get::<_, String>(1)?, r.get(2)?, r.get(3)?))
    })
    .optional()?
      .map(|(tipo, usuario, secret_ref)| Ok(CredRow { kind: Kind::parse(&tipo)?, usuario, secret_ref }))
    .unwrap_or_else(|| Err(AppError::msg("Credencial não encontrada.")))
}

/// Valida os campos conforme o tipo (antes de tocar no chaveiro).
pub fn validate(nome: &str, kind: Kind, usuario: Option<&str>, segredo: Option<&str>) -> AppResult<()> {
    if nome.trim().is_empty() {
        return Err(AppError::msg("Informe um nome para a credencial."));
    }
    let has = |s: Option<&str>| s.map(|v| !v.trim().is_empty()).unwrap_or(false);
    match kind {
        Kind::Sistema => {}
        Kind::Token => {
            if !has(segredo) {
                return Err(AppError::msg("Informe o token."));
            }
        }
        Kind::Userpass => {
            if !has(usuario) || !has(segredo) {
                return Err(AppError::msg("Informe usuário e senha."));
            }
        }
        Kind::SshKey => {
            let k = segredo.unwrap_or("");
            if !k.contains("PRIVATE KEY") {
                return Err(AppError::msg("Cole a chave SSH PRIVADA completa (começa com -----BEGIN … PRIVATE KEY-----)."));
            }
        }
    }
    Ok(())
}

pub fn insert(conn: &Connection, nome: &str, kind: Kind, usuario: Option<&str>, secret_ref: Option<&str>) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO git_credentials(nome, tipo, usuario, secret_ref) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![nome.trim(), kind.as_str(), usuario.map(str::trim).filter(|u| !u.is_empty()), secret_ref],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Apaga a credencial e devolve a referência do chaveiro (para o chamador apagar o segredo).
pub fn delete(conn: &Connection, id: i64) -> AppResult<Option<String>> {
    let secret_ref: Option<Option<String>> = conn
        .query_row("SELECT secret_ref FROM git_credentials WHERE id = ?1", [id], |r| r.get(0))
        .optional()?;
    let Some(secret_ref) = secret_ref else {
        return Err(AppError::msg("Credencial não encontrada."));
    };
    conn.execute("DELETE FROM git_credentials WHERE id = ?1", [id])?; // ON DELETE SET NULL nos repos
    Ok(secret_ref)
}

/// Repositórios das pastas cadastradas, com a credencial vinculada (se houver).
pub fn links(conn: &Connection) -> AppResult<Vec<RepoLink>> {
    let mut out = Vec::new();
    let mut stmt = conn.prepare("SELECT id, caminho FROM local_folders ORDER BY caminho COLLATE NOCASE")?;
    let folders: Vec<(i64, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (folder_id, folder) in folders {
        for repo in crate::folders::find_repos(std::path::Path::new(&folder)) {
            let (cred_id, project_id, branch): (Option<i64>, Option<i64>, Option<String>) = conn
                .query_row(
                    "SELECT git_cred_id, project_id, branch_gatilho FROM local_repos WHERE folder_id = ?1 AND rel = ?2",
                    rusqlite::params![folder_id, repo.rel],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?
                .unwrap_or((None, None, None));
            out.push(RepoLink { folder_id, folder: folder.clone(), rel: repo.rel, nome: repo.nome, cred_id, project_id, branch, remote_url: None });
        }
    }
    Ok(out)
}

pub fn set_repo_credential(conn: &Connection, folder_id: i64, rel: &str, cred_id: Option<i64>) -> AppResult<()> {
    conn.execute(
        "INSERT INTO local_repos(folder_id, rel, git_cred_id) VALUES (?1, ?2, ?3)
         ON CONFLICT(folder_id, rel) DO UPDATE SET git_cred_id = excluded.git_cred_id",
        rusqlite::params![folder_id, rel, cred_id],
    )?;
    Ok(())
}

/// Caminho do repositório no disco, validando que `rel` é "." ou um repositório achado na pasta.
pub fn repo_path(conn: &Connection, folder_id: i64, rel: &str) -> AppResult<std::path::PathBuf> {
    let folder: String = conn
        .query_row("SELECT caminho FROM local_folders WHERE id = ?1", [folder_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::msg("Pasta não encontrada."))?;
    let base = std::path::PathBuf::from(&folder);
    if !crate::folders::find_repos(&base).iter().any(|r| r.rel == rel) {
        return Err(AppError::msg("Repositório não encontrado nesta pasta."));
    }
    Ok(if rel == "." { base } else { base.join(rel) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        crate::db::open_in_memory().unwrap()
    }

    #[test]
    fn validacao_por_tipo() {
        assert!(validate("", Kind::Sistema, None, None).is_err());
        assert!(validate("x", Kind::Sistema, None, None).is_ok());
        assert!(validate("x", Kind::Token, None, Some("  ")).is_err());
        assert!(validate("x", Kind::Token, None, Some("ghp")).is_ok());
        assert!(validate("x", Kind::Userpass, Some("ana"), None).is_err());
        assert!(validate("x", Kind::Userpass, Some("ana"), Some("s")).is_ok());
        assert!(validate("x", Kind::SshKey, None, Some("chave pública ssh-rsa AAA")).is_err());
        assert!(validate("x", Kind::SshKey, None, Some("-----BEGIN OPENSSH PRIVATE KEY-----\n..")).is_ok());
    }

    #[test]
    fn inserir_listar_e_apagar_devolve_a_referencia_do_segredo() {
        let c = db();
        let id = insert(&c, " Meu token ", Kind::Token, Some(" "), Some("ref-1")).unwrap();
        let l = list(&c).unwrap();
        assert_eq!(l[0].nome, "Meu token");
        assert_eq!(l[0].usuario, None); // vazio vira NULL
        assert_eq!(get(&c, id).unwrap().kind, Kind::Token);
        assert_eq!(delete(&c, id).unwrap().as_deref(), Some("ref-1"));
        assert!(get(&c, id).is_err());
        assert!(delete(&c, id).is_err());
    }

    #[test]
    fn apagar_credencial_desvincula_o_repositorio() {
        let c = db();
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("p/.git")).unwrap();
        let folder = crate::folders::add(&c, t.path().to_str().unwrap()).unwrap();
        let cred = insert(&c, "c", Kind::Sistema, None, None).unwrap();
        set_repo_credential(&c, folder, "p", Some(cred)).unwrap();
        assert_eq!(links(&c).unwrap()[0].cred_id, Some(cred));
        set_repo_credential(&c, folder, "p", Some(cred)).unwrap(); // idempotente
        delete(&c, cred).unwrap();
        assert_eq!(links(&c).unwrap()[0].cred_id, None);
    }

    #[test]
    fn repo_path_recusa_rel_fora_da_pasta() {
        let c = db();
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("p/.git")).unwrap();
        let folder = crate::folders::add(&c, t.path().to_str().unwrap()).unwrap();
        assert!(repo_path(&c, folder, "p").unwrap().ends_with("p"));
        assert!(repo_path(&c, folder, "..").is_err());
        assert!(repo_path(&c, folder, "../outro").is_err());
        assert!(repo_path(&c, 999, "p").is_err());
    }
}
