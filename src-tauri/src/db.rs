//! SQLite local + migrações versionadas (`PRAGMA user_version`).
//!
//! Segredos NUNCA entram aqui: as tabelas guardam só a referência (`secret_ref`) a uma entrada
//! do chaveiro do SO (ver `secrets.rs`).

use std::path::Path;

use rusqlite::Connection;

use crate::error::AppResult;

/// Cada item é uma migração; o índice + 1 é o `user_version` depois de aplicá-la.
/// NUNCA edite uma migração já publicada: acrescente uma nova ao fim.
const MIGRATIONS: &[&str] = &[
    // 1 — esquema inicial (ver docs/spec.md §2.1)
    r#"
    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE taiga_accounts (
        id         INTEGER PRIMARY KEY,
        nome       TEXT NOT NULL,
        host       TEXT NOT NULL,           -- URL base, ex.: https://taiga.exemplo.com.br
        usuario    TEXT NOT NULL,
        secret_ref TEXT NOT NULL,           -- entrada do chaveiro com a senha
        criado_em  TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE taiga_projects (
        id         INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES taiga_accounts(id) ON DELETE CASCADE,
        slug       TEXT NOT NULL,
        nome       TEXT,                    -- cache do nome vindo da API
        UNIQUE (account_id, slug)
    );

    CREATE TABLE git_credentials (
        id         INTEGER PRIMARY KEY,
        nome       TEXT NOT NULL,
        tipo       TEXT NOT NULL CHECK (tipo IN ('sistema','token','userpass','ssh_key')),
        usuario    TEXT,
        secret_ref TEXT,                    -- NULL quando tipo = 'sistema'
        criado_em  TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE local_folders (
        id        INTEGER PRIMARY KEY,
        caminho   TEXT NOT NULL UNIQUE,
        criado_em TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE local_repos (
        id            INTEGER PRIMARY KEY,
        folder_id     INTEGER NOT NULL REFERENCES local_folders(id) ON DELETE CASCADE,
        rel           TEXT NOT NULL,        -- "." ou nome do repositório dentro da pasta
        git_cred_id   INTEGER REFERENCES git_credentials(id) ON DELETE SET NULL,
        project_id    INTEGER REFERENCES taiga_projects(id) ON DELETE SET NULL,
        branch_gatilho TEXT,                -- única branch que dispara relatório + US
        UNIQUE (folder_id, rel)
    );

    CREATE TABLE destinations (
        id         INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES taiga_accounts(id) ON DELETE CASCADE,
        rotulo     TEXT NOT NULL,
        url        TEXT NOT NULL            -- link da US de destino da prestação
    );

    CREATE TABLE reports (
        id         INTEGER PRIMARY KEY,
        repo_id    INTEGER REFERENCES local_repos(id) ON DELETE SET NULL,
        sha        TEXT NOT NULL,           -- SHA curto (7)
        branch     TEXT NOT NULL DEFAULT '',
        project_id INTEGER REFERENCES taiga_projects(id) ON DELETE SET NULL,
        us_id      INTEGER NOT NULL,
        us_ref     INTEGER,
        titulo     TEXT,
        resumo_hash TEXT,
        externo    INTEGER NOT NULL DEFAULT 0,
        criado_em  TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE (repo_id, sha, branch)
    );

    CREATE TABLE card_items (
        id          INTEGER PRIMARY KEY,
        report_id   INTEGER,                -- informativo, sem FK de propósito
        project_id  INTEGER,
        us_id       INTEGER NOT NULL,
        us_ref      INTEGER,
        titulo      TEXT NOT NULL DEFAULT '',
        ordem       INTEGER NOT NULL DEFAULT 0,
        texto       TEXT NOT NULL,
        tipo        TEXT NOT NULL DEFAULT 'feito' CHECK (tipo IN ('feito','pendencia')),
        data_ref    TEXT,
        prestado_em TEXT,                   -- NULL = pendente
        criado_em   TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX idx_card_items_us ON card_items(us_id);
    "#,
    // 2 — a US de destino da prestação é informada na hora de prestar conta (muda toda semana),
    //     então não há destinos cadastrados.
    "DROP TABLE IF EXISTS destinations;",
];

pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| crate::error::AppError::msg(e.to_string()))?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    Ok(())
}

fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
        tx.commit()?;
    }
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    Ok(match rows.next()? {
        Some(r) => Some(r.get(0)?),
        None => None,
    })
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

pub fn clear_setting(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migra_do_zero_e_e_idempotente() {
        let mut conn = open_in_memory().unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
        migrate(&mut conn).unwrap(); // rodar de novo não faz nada nem falha
        let n: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master WHERE name='local_repos'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn settings_gravar_ler_e_limpar() {
        let conn = open_in_memory().unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);
        set_setting(&conn, "k", "a").unwrap();
        set_setting(&conn, "k", "b").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap().as_deref(), Some("b"));
        clear_setting(&conn, "k").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);
    }

    #[test]
    fn apagar_conta_apaga_projetos_em_cascata() {
        let conn = open_in_memory().unwrap();
        conn.execute("INSERT INTO taiga_accounts(id,nome,host,usuario,secret_ref) VALUES (1,'a','h','u','r')", []).unwrap();
        conn.execute("INSERT INTO taiga_projects(account_id,slug) VALUES (1,'p')", []).unwrap();
        conn.execute("DELETE FROM taiga_accounts WHERE id=1", []).unwrap();
        let n: i64 = conn.query_row("SELECT count(*) FROM taiga_projects", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }
}
