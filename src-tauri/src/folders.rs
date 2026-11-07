//! Pastas locais cadastradas e descoberta dos repositórios git dentro delas.
//!
//! Regra (igual ao ci_cd): a própria pasta, se for um repositório; senão, seus filhos diretos
//! que são repositórios (ignora os que começam com ".").

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RepoInfo {
    /// "." (a própria pasta) ou o nome do filho direto.
    pub rel: String,
    pub nome: String,
}

#[derive(Debug, Serialize)]
pub struct FolderInfo {
    pub id: i64,
    pub caminho: String,
    /// `false` quando a pasta foi apagada/movida depois do cadastro.
    pub existe: bool,
    pub repos: Vec<RepoInfo>,
}

/// `.git` pode ser pasta (clone normal) ou arquivo (worktree/submódulo).
fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

pub fn find_repos(base: &Path) -> Vec<RepoInfo> {
    if is_repo(base) {
        let nome = base.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        return vec![RepoInfo { rel: ".".into(), nome }];
    }
    let Ok(entries) = std::fs::read_dir(base) else {
        return Vec::new();
    };
    let mut out: Vec<RepoInfo> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let nome = e.file_name().to_string_lossy().into_owned();
            (!nome.starts_with('.') && is_repo(&e.path())).then(|| RepoInfo { rel: nome.clone(), nome })
        })
        .collect();
    out.sort_by_key(|r| r.nome.to_lowercase());
    out
}

/// Caminho absoluto e limpo. No Windows o `canonicalize` devolve o prefixo `\\?\`, que o
/// usuário não deve ver nem gravar.
pub fn normalize(raw: &str) -> AppResult<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::msg("Informe o caminho da pasta."));
    }
    let path = std::fs::canonicalize(raw)
        .map_err(|_| AppError::msg(format!("A pasta não existe ou não pode ser lida: {raw}")))?;
    if !path.is_dir() {
        return Err(AppError::msg("O caminho informado não é uma pasta."));
    }
    let text = path.to_string_lossy();
    Ok(match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path,
    })
}

pub fn add(conn: &Connection, raw: &str) -> AppResult<i64> {
    let path = normalize(raw)?;
    let text = path.to_string_lossy().into_owned();
    let exists: bool = conn
        .query_row("SELECT 1 FROM local_folders WHERE caminho = ?1", [&text], |_| Ok(true))
        .unwrap_or(false);
    if exists {
        return Err(AppError::msg("Esta pasta já está cadastrada."));
    }
    conn.execute("INSERT INTO local_folders(caminho) VALUES (?1)", [&text])?;
    Ok(conn.last_insert_rowid())
}

pub fn remove(conn: &Connection, id: i64) -> AppResult<()> {
    let n = conn.execute("DELETE FROM local_folders WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(AppError::msg("Pasta não encontrada."));
    }
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<Vec<FolderInfo>> {
    let mut stmt = conn.prepare("SELECT id, caminho FROM local_folders ORDER BY caminho COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (id, caminho) = row?;
        let base = Path::new(&caminho);
        let existe = base.is_dir();
        let repos = if existe { find_repos(base) } else { Vec::new() };
        out.push(FolderInfo { id, caminho, existe, repos });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fake_repo(dir: &Path) {
        fs::create_dir_all(dir.join(".git")).unwrap();
    }

    #[test]
    fn a_propria_pasta_vale_quando_e_repositorio() {
        let t = tempfile::tempdir().unwrap();
        fake_repo(t.path());
        fake_repo(&t.path().join("filho")); // ignorado: a pasta já é um repositório
        let r = find_repos(t.path());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].rel, ".");
    }

    #[test]
    fn lista_filhos_diretos_ordenados_e_ignora_ocultos_e_comuns() {
        let t = tempfile::tempdir().unwrap();
        fake_repo(&t.path().join("zeta"));
        fake_repo(&t.path().join("Alfa"));
        fake_repo(&t.path().join(".oculto"));
        fs::create_dir_all(t.path().join("sem-git")).unwrap();
        fs::write(t.path().join("arquivo.txt"), "x").unwrap();
        fs::create_dir_all(t.path().join("neto/profundo")).unwrap();
        fake_repo(&t.path().join("neto/profundo")); // neto: não conta
        let nomes: Vec<_> = find_repos(t.path()).into_iter().map(|r| r.nome).collect();
        assert_eq!(nomes, ["Alfa", "zeta"]);
    }

    #[test]
    fn git_como_arquivo_tambem_e_repositorio() {
        let t = tempfile::tempdir().unwrap();
        let wt = t.path().join("worktree");
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join(".git"), "gitdir: /algum/lugar").unwrap();
        assert_eq!(find_repos(t.path()).len(), 1);
    }

    #[test]
    fn pasta_inexistente_ou_arquivo_e_recusada() {
        assert!(normalize("").is_err());
        assert!(normalize("/nao/existe/mesmo").is_err());
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("a.txt");
        fs::write(&f, "x").unwrap();
        assert!(normalize(f.to_str().unwrap()).is_err());
    }

    #[test]
    fn cadastrar_listar_duplicada_e_remover() {
        let conn = crate::db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        fake_repo(&t.path().join("proj"));
        let id = add(&conn, t.path().to_str().unwrap()).unwrap();
        assert!(add(&conn, t.path().to_str().unwrap()).is_err());
        let lista = list(&conn).unwrap();
        assert_eq!(lista.len(), 1);
        assert!(lista[0].existe);
        assert_eq!(lista[0].repos[0].nome, "proj");
        remove(&conn, id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
        assert!(remove(&conn, id).is_err());
    }

    #[test]
    fn pasta_apagada_depois_aparece_como_inexistente() {
        let conn = crate::db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        let sub = t.path().join("sumiu");
        fs::create_dir_all(&sub).unwrap();
        add(&conn, sub.to_str().unwrap()).unwrap();
        fs::remove_dir_all(&sub).unwrap();
        let lista = list(&conn).unwrap();
        assert!(!lista[0].existe);
        assert!(lista[0].repos.is_empty());
    }
}
