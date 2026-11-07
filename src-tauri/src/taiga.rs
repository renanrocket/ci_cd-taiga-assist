//! Cliente da API REST do Taiga.
//!
//! Segurança: a senha só existe em memória no `Client` e nunca entra em mensagem de erro, log
//! ou URL. Mensagens de erro são seguras para exibir na UI.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde_json::{json, Value};

use crate::error::AppError;

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct TaigaError {
    pub message: String,
    /// Status HTTP do Taiga, quando houve resposta (404, 409…).
    pub status: Option<u16>,
}

impl std::fmt::Display for TaigaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<TaigaError> for AppError {
    fn from(e: TaigaError) -> Self {
        AppError::Msg(e.message)
    }
}

fn err(message: impl Into<String>) -> TaigaError {
    TaigaError { message: message.into(), status: None }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RemoteProject {
    pub slug: String,
    pub nome: String,
}

/// `meu.taiga.com` / `https://meu.taiga.com/` → `https://meu.taiga.com/api/v1`.
pub fn normalize_base(host: &str) -> Result<String, TaigaError> {
    let mut base = host.trim().trim_end_matches('/').to_owned();
    if base.is_empty() {
        return Err(err("Informe a URL do Taiga."));
    }
    if !base.starts_with("http://") && !base.starts_with("https://") {
        base = format!("https://{base}");
    }
    if !base.ends_with("/api/v1") {
        base.push_str("/api/v1");
    }
    Ok(base)
}


/// Host (minúsculo, sem porta) de uma URL ou de um host digitado sem esquema.
pub fn host_of(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let with_scheme = if raw.contains("://") { raw.to_owned() } else { format!("https://{raw}") };
    reqwest::Url::parse(&with_scheme).ok()?.host_str().map(str::to_lowercase)
}

/// Link de uma US (`https://taiga.x.com/project/meu-projeto/us/350`) → (host, slug, ref).
pub fn parse_us_url(url: &str) -> Result<(String, String, i64), TaigaError> {
    let bad = || err("URL inválida. Use o link de uma US, ex.: https://taiga.x.com/project/meu-projeto/us/350");
    let u = reqwest::Url::parse(url.trim()).map_err(|_| bad())?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return Err(bad());
    }
    let host = u.host_str().ok_or_else(bad)?.to_lowercase();
    let segs: Vec<&str> = u.path().trim_matches('/').split('/').collect();
    match segs.as_slice() {
        ["project", slug, "us", r] if !slug.is_empty() => {
            let r: i64 = r.parse().map_err(|_| bad())?;
            Ok((host, (*slug).to_owned(), r))
        }
        _ => Err(bad()),
    }
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base: String,
    user: String,
    pass: String,
    token: Option<String>,
    pub user_id: Option<i64>,
}

impl Client {
    pub fn new(host: &str, user: &str, pass: &str) -> Result<Self, TaigaError> {
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|_| err("Não consegui iniciar o cliente HTTP."))?;
        Ok(Client {
            http,
            base: normalize_base(host)?,
            user: user.to_owned(),
            pass: pass.to_owned(),
            token: None,
            user_id: None,
        })
    }

    async fn req(&self, method: Method, path: &str, query: &[(&str, &str)], all: bool, body: Option<&Value>) -> Result<Value, TaigaError> {
        let mut rb = self.http.request(method, format!("{}{}", self.base, path)).query(query);
        if let Some(b) = body {
            rb = rb.json(b);
        }
        if let Some(t) = &self.token {
            rb = rb.bearer_auth(t);
        }
        if all {
            rb = rb.header("x-disable-pagination", "True");
        }
        let resp = rb.send().await.map_err(|e| {
            err(if e.is_timeout() {
                "Taiga demorou demais para responder."
            } else if e.is_connect() {
                "Taiga inacessível (verifique a URL e a rede)."
            } else {
                "Falha de comunicação com o Taiga."
            })
        })?;
        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if status.is_client_error() || status.is_server_error() {
            let detail = body
                .get("_error_message")
                .and_then(Value::as_str)
                .map(|m| m.chars().take(200).collect::<String>())
                .filter(|m| !m.is_empty());
            let message = match (status, detail) {
                (StatusCode::UNAUTHORIZED, _) => "Taiga recusou o acesso (401). Confira usuário e senha.".to_owned(),
                (s, Some(d)) => format!("Taiga respondeu {}: {d}", s.as_u16()),
                (s, None) => format!("Taiga respondeu {}.", s.as_u16()),
            };
            return Err(TaigaError { message, status: Some(status.as_u16()) });
        }
        Ok(body)
    }

    pub async fn login(&mut self) -> Result<(), TaigaError> {
        let resp = self
            .http
            .post(format!("{}/auth", self.base))
            .json(&json!({"type": "normal", "username": self.user, "password": self.pass}))
            .send()
            .await
            .map_err(|e| {
                err(if e.is_timeout() {
                    "Taiga demorou demais para responder."
                } else if e.is_connect() {
                    "Taiga inacessível (verifique a URL e a rede)."
                } else {
                    "Falha de comunicação com o Taiga."
                })
            })?;
        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            let detail = body.get("_error_message").and_then(Value::as_str).unwrap_or("");
            let message = if status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED {
                "Login recusado pelo Taiga. Confira usuário e senha.".to_owned()
            } else if detail.is_empty() {
                format!("Taiga respondeu {} no login.", status.as_u16())
            } else {
                format!("Taiga respondeu {}: {}", status.as_u16(), detail.chars().take(200).collect::<String>())
            };
            return Err(TaigaError { message, status: Some(status.as_u16()) });
        }
        let token = body.get("auth_token").and_then(Value::as_str).map(str::to_owned);
        let id = body.get("id").and_then(Value::as_i64);
        match (token, id) {
            (Some(t), Some(i)) => {
                self.token = Some(t);
                self.user_id = Some(i);
                Ok(())
            }
            _ => Err(err("O login no Taiga não devolveu token.")),
        }
    }

    /// Projetos de que o usuário é membro.
    pub async fn my_projects(&self) -> Result<Vec<RemoteProject>, TaigaError> {
        let id = self.user_id.ok_or_else(|| err("Faça login antes."))?;
        let data = self.req(Method::GET, "/projects", &[("member", &id.to_string()), ("order_by", "name")], true, None).await?;
        let mut out: Vec<RemoteProject> = data
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        let slug = p.get("slug")?.as_str()?.to_owned();
                        let nome = p.get("name").and_then(Value::as_str).unwrap_or(&slug).to_owned();
                        Some(RemoteProject { slug, nome })
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by_key(|p| p.nome.to_lowercase());
        Ok(out)
    }

    /// História pela referência (`#350`) dentro de um projeto.
        pub async fn us_by_ref(&self, project_id: i64, r: i64) -> Result<Value, TaigaError> {
        self.req(Method::GET, "/userstories/by_ref", &[("ref", &r.to_string()), ("project", &project_id.to_string())], false, None).await
    }

    pub async fn get_us(&self, id: i64) -> Result<Value, TaigaError> {
        self.req(Method::GET, &format!("/userstories/{id}"), &[], false, None).await
    }

    pub async fn patch_us(&self, id: i64, payload: &Value) -> Result<Value, TaigaError> {
        self.req(Method::PATCH, &format!("/userstories/{id}"), &[], false, Some(payload)).await
    }

    /// Todas as histórias do projeto (sem paginação) — uma chamada por projeto.
    pub async fn userstories(&self, project_id: i64) -> Result<Vec<Value>, TaigaError> {
        let v = self.req(Method::GET, "/userstories", &[("project", &project_id.to_string())], true, None).await?;
        Ok(v.as_array().cloned().unwrap_or_default())
    }

    /// Cria uma história de usuário.
    pub async fn create_us(&self, payload: &Value) -> Result<Value, TaigaError> {
        self.req(Method::POST, "/userstories", &[], false, Some(payload)).await
    }

    pub async fn project(&self, slug: &str) -> Result<Value, TaigaError> {
        self.req(Method::GET, "/projects/by_slug", &[("slug", slug)], false, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn normaliza_a_url_base() {
        assert_eq!(normalize_base("taiga.x.com").unwrap(), "https://taiga.x.com/api/v1");
        assert_eq!(normalize_base(" https://taiga.x.com/ ").unwrap(), "https://taiga.x.com/api/v1");
        assert_eq!(normalize_base("http://localhost:9000/api/v1").unwrap(), "http://localhost:9000/api/v1");
        assert!(normalize_base("   ").is_err());
    }

    fn login_mock(server: &MockServer) {
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/v1/auth")
                .json_body(json!({"type": "normal", "username": "ana", "password": "s3nha"}));
            then.status(200).json_body(json!({"auth_token": "TOK", "id": 42}));
        });
    }

    #[tokio::test]
    async fn login_guarda_token_e_lista_projetos_do_membro() {
        let server = MockServer::start();
        login_mock(&server);
        let lista = server.mock(|when, then| {
            when.method(GET)
                .path("/api/v1/projects")
                .query_param("member", "42")
                .header("authorization", "Bearer TOK")
                .header("x-disable-pagination", "True");
            then.status(200).json_body(json!([
                {"slug": "zeta", "name": "Zeta"},
                {"slug": "alfa", "name": "alfa projeto"},
                {"id": 1}
            ]));
        });
        let mut c = Client::new(&server.base_url(), "ana", "s3nha").unwrap();
        c.login().await.unwrap();
        assert_eq!(c.user_id, Some(42));
        let p = c.my_projects().await.unwrap();
        assert_eq!(p.iter().map(|x| x.slug.as_str()).collect::<Vec<_>>(), ["alfa", "zeta"]);
        lista.assert();
    }

    #[tokio::test]
    async fn login_recusado_da_mensagem_segura_sem_senha() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/auth");
            then.status(400).json_body(json!({"_error_message": "No active account s3nha"}));
        });
        let mut c = Client::new(&server.base_url(), "ana", "s3nha").unwrap();
        let e = c.login().await.unwrap_err();
        assert_eq!(e.status, Some(400));
        assert!(e.message.contains("Confira usuário e senha"));
        assert!(!e.message.contains("s3nha"));
    }

    #[tokio::test]
    async fn projeto_inexistente_devolve_status_404() {
        let server = MockServer::start();
        login_mock(&server);
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/projects/by_slug").query_param("slug", "nao-existe");
            then.status(404).json_body(json!({"_error_message": "Not found"}));
        });
        let mut c = Client::new(&server.base_url(), "ana", "s3nha").unwrap();
        c.login().await.unwrap();
        let e = c.project("nao-existe").await.unwrap_err();
        assert_eq!(e.status, Some(404));
    }

    #[tokio::test]
    async fn servidor_fora_do_ar_nao_vaza_a_senha() {
        // porta quase certamente fechada
        let mut c = Client::new("http://127.0.0.1:1", "ana", "s3nha").unwrap();
        let e = c.login().await.unwrap_err();
        assert!(e.message.contains("inacessível") || e.message.contains("Falha"));
        assert!(!e.message.contains("s3nha"));
    }

    #[tokio::test]
    async fn listar_projetos_sem_login_pede_login() {
        let c = Client::new("http://127.0.0.1:1", "a", "b").unwrap();
        assert!(c.my_projects().await.is_err());
    }
}

#[cfg(test)]
mod tests_url {
    use super::*;

    #[test]
    fn parse_us_url_valida() {
        assert_eq!(
            parse_us_url("https://Taiga.X.com/project/meu-proj/us/350").unwrap(),
            ("taiga.x.com".into(), "meu-proj".into(), 350)
        );
        assert_eq!(parse_us_url(" http://t.x/project/a_b.c/us/7/?x=1#h ").unwrap().2, 7);
    }

    #[test]
    fn parse_us_url_invalida() {
        for u in [
            "", "taiga.x.com/project/a/us/1", "ftp://t.x/project/a/us/1", "https://t.x/project/a/us/abc",
            "https://t.x/project/a/task/1", "https://t.x/project//us/1", "https://t.x/project/a/us/1/extra",
        ] {
            assert!(parse_us_url(u).is_err(), "deveria recusar: {u}");
        }
    }

    #[test]
    fn host_of_aceita_com_e_sem_esquema() {
        assert_eq!(host_of("Taiga.X.com").as_deref(), Some("taiga.x.com"));
        assert_eq!(host_of("https://taiga.x.com:8443/api/v1").as_deref(), Some("taiga.x.com"));
        assert_eq!(host_of("  ").as_deref(), None);
    }

    #[tokio::test]
    async fn us_by_ref_consulta_projeto_e_ref() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/auth");
            then.status(200).json_body(serde_json::json!({"auth_token": "T", "id": 1}));
        });
        let m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/by_ref").query_param("ref", "350").query_param("project", "9");
            then.status(200).json_body(serde_json::json!({"id": 5, "subject": "Reunião"}));
        });
        let mut c = Client::new(&server.base_url(), "a", "b").unwrap();
        c.login().await.unwrap();
        let us = c.us_by_ref(9, 350).await.unwrap();
        assert_eq!(us["subject"], "Reunião");
        m.assert();
    }
}
