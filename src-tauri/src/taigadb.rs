//! Contas Taiga (senha no chaveiro), projetos importados e o vínculo repositório → projeto + branch.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::taiga::RemoteProject;

#[derive(Debug, Serialize)]
pub struct AccountInfo {
    pub id: i64,
    pub nome: String,
    pub host: String,
    pub usuario: String,
}

pub struct AccountRow {
    pub host: String,
    pub usuario: String,
    pub secret_ref: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectInfo {
    pub id: i64,
    pub account_id: i64,
    pub account_nome: String,
    pub slug: String,
    pub nome: String,
}

pub fn list_accounts(conn: &Connection) -> AppResult<Vec<AccountInfo>> {
    let mut stmt = conn.prepare("SELECT id, nome, host, usuario FROM taiga_accounts ORDER BY nome COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| Ok(AccountInfo { id: r.get(0)?, nome: r.get(1)?, host: r.get(2)?, usuario: r.get(3)? }))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_account(conn: &Connection, id: i64) -> AppResult<AccountRow> {
    conn.query_row("SELECT host, usuario, secret_ref FROM taiga_accounts WHERE id = ?1", [id], |r| {
        Ok(AccountRow { host: r.get(0)?, usuario: r.get(1)?, secret_ref: r.get(2)? })
    })
    .optional()?
    .ok_or_else(|| AppError::msg("Conta Taiga não encontrada."))
}

pub fn validate_account(nome: &str, host: &str, usuario: &str, senha: &str) -> AppResult<()> {
    if nome.trim().is_empty() || host.trim().is_empty() || usuario.trim().is_empty() || senha.is_empty() {
        return Err(AppError::msg("Preencha nome, URL, usuário e senha."));
    }
    Ok(())
}

pub fn insert_account(conn: &Connection, nome: &str, host: &str, usuario: &str, secret_ref: &str) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO taiga_accounts(nome, host, usuario, secret_ref) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![nome.trim(), host.trim(), usuario.trim(), secret_ref],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Apaga a conta (projetos em cascata; repositórios ficam sem projeto) e devolve o `secret_ref`.
pub fn delete_account(conn: &Connection, id: i64) -> AppResult<String> {
    let r = get_account(conn, id)?.secret_ref;
    conn.execute("DELETE FROM taiga_accounts WHERE id = ?1", [id])?;
    Ok(r)
}

pub fn list_projects(conn: &Connection) -> AppResult<Vec<ProjectInfo>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.account_id, a.nome, p.slug, COALESCE(p.nome, p.slug)
           FROM taiga_projects p JOIN taiga_accounts a ON a.id = p.account_id
          ORDER BY a.nome COLLATE NOCASE, COALESCE(p.nome, p.slug) COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ProjectInfo { id: r.get(0)?, account_id: r.get(1)?, account_nome: r.get(2)?, slug: r.get(3)?, nome: r.get(4)? })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Importa projetos (idempotente): quem já existe só tem o nome atualizado.
pub fn add_projects(conn: &Connection, account_id: i64, projects: &[RemoteProject]) -> AppResult<()> {
    get_account(conn, account_id)?;
    for p in projects {
        conn.execute(
            "INSERT INTO taiga_projects(account_id, slug, nome) VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, slug) DO UPDATE SET nome = excluded.nome",
            rusqlite::params![account_id, p.slug, p.nome],
        )?;
    }
    Ok(())
}

pub fn remove_project(conn: &Connection, id: i64) -> AppResult<()> {
    if conn.execute("DELETE FROM taiga_projects WHERE id = ?1", [id])? == 0 {
        return Err(AppError::msg("Projeto não encontrado."));
    }
    Ok(())
}

/// Slug + conta de um projeto importado.
pub fn project_ref(conn: &Connection, id: i64) -> AppResult<(i64, String)> {
    conn.query_row("SELECT account_id, slug FROM taiga_projects WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or_else(|| AppError::msg("Projeto não encontrado."))
}

/// Define projeto Taiga + branch gatilho do repositório (sem mexer na credencial git).
pub fn set_repo_taiga(conn: &Connection, folder_id: i64, rel: &str, project_id: Option<i64>, branch: Option<&str>) -> AppResult<()> {
    if let Some(p) = project_id {
        project_ref(conn, p)?;
    }
    conn.execute(
        "INSERT INTO local_repos(folder_id, rel, project_id, branch_gatilho) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(folder_id, rel) DO UPDATE SET project_id = excluded.project_id, branch_gatilho = excluded.branch_gatilho",
        rusqlite::params![folder_id, rel, project_id, branch.map(str::trim).filter(|b| !b.is_empty())],
    )?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        crate::db::open_in_memory().unwrap()
    }

    fn rp(slug: &str, nome: &str) -> RemoteProject {
        RemoteProject { slug: slug.into(), nome: nome.into() }
    }

    #[test]
    fn conta_projetos_importacao_idempotente_e_nome_atualizado() {
        let c = db();
        let a = insert_account(&c, " Trabalho ", " https://t.x ", " ana ", "ref").unwrap();
        add_projects(&c, a, &[rp("p1", "Um"), rp("p2", "Dois")]).unwrap();
        add_projects(&c, a, &[rp("p1", "Um (renomeado)")]).unwrap();
        let l = list_projects(&c).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l.iter().find(|p| p.slug == "p1").unwrap().nome, "Um (renomeado)");
        assert_eq!(list_accounts(&c).unwrap()[0].usuario, "ana");
        assert!(add_projects(&c, 999, &[rp("x", "x")]).is_err());
    }

    #[test]
    fn duas_contas_podem_ter_o_mesmo_slug() {
        let c = db();
        let a = insert_account(&c, "A", "h1", "u", "r1").unwrap();
        let b = insert_account(&c, "B", "h2", "u", "r2").unwrap();
        add_projects(&c, a, &[rp("site", "Site A")]).unwrap();
        add_projects(&c, b, &[rp("site", "Site B")]).unwrap();
        assert_eq!(list_projects(&c).unwrap().len(), 2);
    }

    #[test]
    fn apagar_conta_remove_projetos_e_desvincula_repositorios() {
        let c = db();
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("r/.git")).unwrap();
        let f = crate::folders::add(&c, t.path().to_str().unwrap()).unwrap();
        let a = insert_account(&c, "A", "h", "u", "ref-a").unwrap();
        add_projects(&c, a, &[rp("p", "P")]).unwrap();
        let pid = list_projects(&c).unwrap()[0].id;
        set_repo_taiga(&c, f, "r", Some(pid), Some(" prod ")).unwrap();
        let l = crate::gitcreds::links(&c).unwrap();
        assert_eq!((l[0].project_id, l[0].branch.as_deref()), (Some(pid), Some("prod")));
        assert_eq!(delete_account(&c, a).unwrap(), "ref-a");
        assert!(list_projects(&c).unwrap().is_empty());
        assert_eq!(crate::gitcreds::links(&c).unwrap()[0].project_id, None);
        assert!(delete_account(&c, a).is_err());
    }

    #[test]
    fn vinculo_taiga_nao_apaga_a_credencial_git() {
        let c = db();
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("r/.git")).unwrap();
        let f = crate::folders::add(&c, t.path().to_str().unwrap()).unwrap();
        let cred = crate::gitcreds::insert(&c, "c", crate::git::Kind::Sistema, None, None).unwrap();
        crate::gitcreds::set_repo_credential(&c, f, "r", Some(cred)).unwrap();
        set_repo_taiga(&c, f, "r", None, Some("prod")).unwrap();
        assert_eq!(crate::gitcreds::links(&c).unwrap()[0].cred_id, Some(cred));
        // e o inverso
        crate::gitcreds::set_repo_credential(&c, f, "r", Some(cred)).unwrap();
        assert_eq!(crate::gitcreds::links(&c).unwrap()[0].branch.as_deref(), Some("prod"));
    }

    #[test]
    fn projeto_inexistente_e_recusado_no_vinculo() {
        let c = db();
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("r/.git")).unwrap();
        let f = crate::folders::add(&c, t.path().to_str().unwrap()).unwrap();
        assert!(set_repo_taiga(&c, f, "r", Some(123), None).is_err());
    }

    #[test]
    fn validacao_da_conta() {
        assert!(validate_account("", "h", "u", "s").is_err());
        assert!(validate_account("n", "h", "u", "").is_err());
        assert!(validate_account("n", "h", "u", "s").is_ok());
    }
}
