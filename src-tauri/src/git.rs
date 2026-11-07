//! Execução do `git` do sistema com autenticação por credencial cadastrada.
//!
//! Segredos NUNCA vão na URL nem na linha de comando (aparecem na lista de processos):
//! - token / usuário+senha: *credential helper* inline que lê variáveis de ambiente;
//! - chave SSH: arquivo temporário (0600 no Unix) apontado por `GIT_SSH_COMMAND`;
//! - "sistema": nada é injetado; vale o que o git/ssh do usuário já tem configurado.
//!
//! Toda saída que volta para a UI passa por `sanitize` (remove os segredos usados).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::process::Command;

use crate::error::{AppError, AppResult};

pub const TIMEOUT_REMOTE: Duration = Duration::from_secs(40);
pub const TIMEOUT_LOCAL: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Sistema,
    Token,
    Userpass,
    SshKey,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Sistema => "sistema",
            Kind::Token => "token",
            Kind::Userpass => "userpass",
            Kind::SshKey => "ssh_key",
        }
    }

    pub fn parse(s: &str) -> AppResult<Kind> {
        match s {
            "sistema" => Ok(Kind::Sistema),
            "token" => Ok(Kind::Token),
            "userpass" => Ok(Kind::Userpass),
            "ssh_key" => Ok(Kind::SshKey),
            other => Err(AppError::msg(format!("tipo de credencial desconhecido: {other}"))),
        }
    }
}

/// Tudo o que é preciso para rodar um comando git autenticado. Mantém vivo o arquivo temporário
/// da chave SSH até o fim do comando (o `Drop` apaga).
pub struct Auth {
    pub url: String,
    pub config_args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub secrets: Vec<String>,
    _key_file: Option<NamedTempFile>,
}

/// `git@host:org/repo.git` / `ssh://git@host:22/org/repo.git` → `https://host/org/repo.git`.
/// URLs http(s) passam como estão. Necessário para token e usuário+senha (basic auth HTTPS).
pub fn to_https(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_owned();
    }
    let strip_user = |s: &str| s.rsplit_once('@').map_or(s.to_owned(), |(_, r)| r.to_owned());
    if let Some((_, rest)) = url.split_once("://") {
        // ssh://[user@]host[:porta]/caminho
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host = strip_user(authority);
        let host = host.split(':').next().unwrap_or(&host).to_owned();
        return format!("https://{host}/{path}");
    }
    // scp-like: [user@]host:caminho (sem "://")
    if let Some((authority, path)) = url.split_once(':') {
        let host = strip_user(authority);
        return format!("https://{host}/{}", path.trim_start_matches('/'));
    }
    url.to_owned()
}

const HELPER: &str =
    r#"credential.helper=!f() { echo "username=${CICD_GIT_USER}"; echo "password=${CICD_GIT_SECRET}"; }; f"#;

pub fn build_auth(
    kind: Kind,
    usuario: Option<&str>,
    secret: Option<&str>,
    url: &str,
    known_hosts: &Path,
) -> AppResult<Auth> {
    let mut auth = Auth {
        url: url.trim().to_owned(),
        config_args: Vec::new(),
        env: vec![("GIT_TERMINAL_PROMPT".into(), "0".into())],
        secrets: Vec::new(),
        _key_file: None,
    };
    let need_secret = || {
        secret
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::msg("A credencial não tem segredo salvo no chaveiro."))
    };
    match kind {
        Kind::Sistema => {
            // ssh sem terminal: nunca trava pedindo senha/confirmação.
            auth.env.push(("GIT_SSH_COMMAND".into(), "ssh -o BatchMode=yes".into()));
        }
        Kind::Token | Kind::Userpass => {
            let secret = need_secret()?;
            let user = match kind {
                Kind::Token => usuario.filter(|u| !u.is_empty()).unwrap_or("x-access-token"),
                _ => usuario
                    .filter(|u| !u.is_empty())
                    .ok_or_else(|| AppError::msg("Informe o usuário da credencial."))?,
            };
            auth.url = to_https(url);
            auth.config_args = vec!["-c".into(), "credential.helper=".into(), "-c".into(), HELPER.into()];
            auth.env.push(("CICD_GIT_USER".into(), user.to_owned()));
            auth.env.push(("CICD_GIT_SECRET".into(), secret.to_owned()));
            auth.secrets.push(secret.to_owned());
        }
        Kind::SshKey => {
            let key = need_secret()?;
            let mut file = NamedTempFile::new().map_err(|e| AppError::msg(e.to_string()))?;
            {
                use std::io::Write;
                let text = if key.ends_with('\n') { key.to_owned() } else { format!("{key}\n") };
                file.write_all(text.replace("\r\n", "\n").as_bytes())
                    .map_err(|e| AppError::msg(e.to_string()))?;
            }
            let key_path = file.path().to_string_lossy().replace('\\', "/");
            let kh = known_hosts.to_string_lossy().replace('\\', "/");
            let user_opt = usuario
                .filter(|u| !u.is_empty() && !url.contains('@'))
                .map(|u| format!("-o User={u} "))
                .unwrap_or_default();
            auth.env.push((
                "GIT_SSH_COMMAND".into(),
                format!(
                    "ssh -i \"{key_path}\" {user_opt}-o UserKnownHostsFile=\"{kh}\" \
                     -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=accept-new"
                ),
            ));
            auth.secrets.push(key.to_owned());
            auth._key_file = Some(file);
        }
    }
    Ok(auth)
}

/// Remove segredos (e a URL com credencial embutida, se houver) de um texto de saída do git.
pub fn sanitize(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_owned();
    for s in secrets.iter().filter(|s| s.len() >= 3) {
        out = out.replace(s.as_str(), "***");
    }
    out
}

pub fn last_chars(s: &str, n: usize) -> String {
    let t = s.trim();
    let count = t.chars().count();
    if count <= n { t.to_owned() } else { t.chars().skip(count - n).collect() }
}

pub struct Output {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

pub async fn run(
    git: &Path,
    cwd: Option<&Path>,
    auth: &Auth,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut cmd = Command::new(git);
    if let Some(dir) = cwd {
        cmd.arg("-C").arg(dir);
    }
    cmd.args(&auth.config_args).args(args);
    for (k, v) in &auth.env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    crate::proc::no_window(&mut cmd);
    let out = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| format!("Tempo esgotado ({}s).", timeout.as_secs()))?
        .map_err(|e| format!("Não consegui executar o git: {e}"))?;
    Ok(Output {
        ok: out.status.success(),
        stdout: sanitize(&String::from_utf8_lossy(&out.stdout), &auth.secrets),
        stderr: sanitize(&String::from_utf8_lossy(&out.stderr), &auth.secrets),
    })
}

/// Auth sem credencial, só com variáveis de ambiente extras (ex.: autor do commit).
pub fn env_auth(extra: Vec<(String, String)>) -> Auth {
    let mut a = plain_auth();
    a.env.extend(extra);
    a
}

pub fn plain_auth() -> Auth {
    Auth {
        url: String::new(),
        config_args: Vec::new(),
        env: vec![("GIT_TERMINAL_PROMPT".into(), "0".into())],
        secrets: Vec::new(),
        _key_file: None,
    }
}

/// URL do remote `origin` (ou `None` se o repositório não tem).
pub async fn remote_url(git: &Path, repo: &Path) -> Option<String> {
    let o = run(git, Some(repo), &plain_auth(), &["remote", "get-url", "origin"], TIMEOUT_LOCAL)
        .await
        .ok()?;
    let url = o.stdout.trim().to_owned();
    (o.ok && !url.is_empty()).then_some(url)
}

/// Branches locais do repositório, em ordem alfabética.
pub async fn local_branches(git: &Path, repo: &Path) -> Vec<String> {
    let Ok(o) = run(git, Some(repo), &plain_auth(), &["for-each-ref", "--format=%(refname:short)", "refs/heads"], TIMEOUT_LOCAL).await
    else {
        return Vec::new();
    };
    if !o.ok {
        return Vec::new();
    }
    let mut v: Vec<String> = o.stdout.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect();
    v.sort_by_key(|b| b.to_lowercase());
    v
}

/// Testa acesso ao remoto: `git ls-remote --heads <url>`. Devolve um resumo ou a mensagem de erro.
pub async fn test_remote(git: &Path, auth: &Auth) -> Result<String, String> {
    let o = run(git, None, auth, &["ls-remote", "--heads", &auth.url], TIMEOUT_REMOTE).await?;
    if o.ok {
        let n = o.stdout.lines().filter(|l| !l.trim().is_empty()).count();
        Ok(format!("Acesso OK ({n} branch(es) no remoto)."))
    } else {
        let msg = last_chars(&o.stderr, 400);
        Err(if msg.is_empty() { "O git falhou sem mensagem.".to_owned() } else { msg })
    }
}

pub fn known_hosts_path(app_data: &Path) -> PathBuf {
    app_data.join("known_hosts")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kh() -> PathBuf {
        PathBuf::from("/tmp/kh")
    }

    #[test]
    fn to_https_converte_formas_comuns() {
        assert_eq!(to_https("https://github.com/o/r.git"), "https://github.com/o/r.git");
        assert_eq!(to_https("git@github.com:o/r.git"), "https://github.com/o/r.git");
        assert_eq!(to_https("github.com:o/r.git"), "https://github.com/o/r.git");
        assert_eq!(to_https("ssh://git@host.com:2222/o/r.git"), "https://host.com/o/r.git");
        assert_eq!(to_https("ssh://host.com/o/r.git"), "https://host.com/o/r.git");
        assert_eq!(to_https("git@host:/abs/r.git"), "https://host/abs/r.git");
    }

    #[test]
    fn sanitize_remove_segredos_e_ignora_curtos() {
        let s = vec!["tok-secreto".to_owned(), "ab".to_owned()];
        assert_eq!(sanitize("erro com tok-secreto aqui ab", &s), "erro com *** aqui ab");
    }

    #[test]
    fn token_vai_por_ambiente_e_nunca_na_linha_de_comando() {
        let a = build_auth(Kind::Token, None, Some("ghp_abc123"), "git@github.com:o/r.git", &kh()).unwrap();
        assert_eq!(a.url, "https://github.com/o/r.git");
        assert!(a.env.contains(&("CICD_GIT_SECRET".into(), "ghp_abc123".into())));
        assert!(a.env.contains(&("CICD_GIT_USER".into(), "x-access-token".into())));
        assert!(a.config_args.iter().all(|x| !x.contains("ghp_abc123")));
        assert!(!a.url.contains("ghp_abc123"));
        assert_eq!(a.secrets, ["ghp_abc123"]);
    }

    #[test]
    fn userpass_exige_usuario_e_segredo() {
        assert!(build_auth(Kind::Userpass, None, Some("s3nha"), "https://h/r.git", &kh()).is_err());
        assert!(build_auth(Kind::Userpass, Some("ana"), None, "https://h/r.git", &kh()).is_err());
        let a = build_auth(Kind::Userpass, Some("ana"), Some("s3nha"), "https://h/r.git", &kh()).unwrap();
        assert!(a.env.contains(&("CICD_GIT_USER".into(), "ana".into())));
    }

    #[test]
    fn chave_ssh_vira_arquivo_temporario_que_some_ao_final() {
        let path;
        {
            let a = build_auth(Kind::SshKey, Some("git"), Some("-----BEGIN KEY-----\nxx\n-----END KEY-----"),
                               "https://h/r.git", &kh()).unwrap();
            let cmd = &a.env.iter().find(|(k, _)| k == "GIT_SSH_COMMAND").unwrap().1;
            assert!(cmd.contains("IdentitiesOnly=yes") && cmd.contains("BatchMode=yes"));
            assert!(cmd.contains("-o User=git"));
            path = a._key_file.as_ref().unwrap().path().to_owned();
            let conteudo = std::fs::read_to_string(&path).unwrap();
            assert!(conteudo.ends_with("-----END KEY-----\n"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600);
            }
        }
        assert!(!path.exists(), "o arquivo da chave deve ser apagado");
    }

    /// O OpenSSH recusa chave com permissões abertas ("UNPROTECTED PRIVATE KEY FILE"; no Windows, ACL com outros usuários).
    /// Gera uma chave de verdade, passa pelo mesmo caminho do app e confere que o `ssh` a carrega (a conexão em si falha, de propósito).
    #[test]
    fn ssh_aceita_as_permissoes_da_chave_temporaria() {
        let (Ok(keygen), Ok(ssh)) = (which::which("ssh-keygen"), which::which("ssh")) else { return };
        let t = tempfile::tempdir().unwrap();
        let k = t.path().join("k");
        let ok = std::process::Command::new(keygen)
            .args(["-q", "-t", "ed25519", "-N", "", "-f"]).arg(&k).status().unwrap().success();
        assert!(ok, "ssh-keygen falhou");
        let privada = std::fs::read_to_string(&k).unwrap();
        let a = build_auth(Kind::SshKey, Some("git"), Some(&privada), "ssh://git@127.0.0.1:1/r.git", &kh()).unwrap();
        let path = a._key_file.as_ref().unwrap().path().to_owned();
        let out = std::process::Command::new(ssh)
            .args(["-i"]).arg(&path)
            .args(["-o", "IdentitiesOnly=yes", "-o", "BatchMode=yes", "-o", "ConnectTimeout=3", "-p", "1", "git@127.0.0.1", "exit"])
            .output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr).to_lowercase();
        assert!(!err.contains("unprotected") && !err.contains("bad permissions") && !err.contains("invalid format"), "{err}");
    }

    #[test]
    fn sem_usuario_extra_quando_a_url_ja_tem_arroba() {
        let a = build_auth(Kind::SshKey, Some("git"), Some("k"), "git@h:o/r.git", &kh()).unwrap();
        let cmd = &a.env.iter().find(|(k, _)| k == "GIT_SSH_COMMAND").unwrap().1;
        assert!(!cmd.contains("-o User="));
    }

    #[test]
    fn sistema_nao_injeta_segredo() {
        let a = build_auth(Kind::Sistema, None, None, "git@h:o/r.git", &kh()).unwrap();
        assert!(a.secrets.is_empty() && a.config_args.is_empty());
        assert_eq!(a.url, "git@h:o/r.git"); // não converte
    }

    /// Integração real com o `git` do sistema contra um remoto local (sem rede).
    #[tokio::test]
    async fn ls_remote_contra_repositorio_local() {
        let Ok(git) = which::which("git") else { return };
        let t = tempfile::tempdir().unwrap();
        let bare = t.path().join("remoto.git");
        let work = t.path().join("trab");
        std::fs::create_dir_all(&work).unwrap();
        let sh = |dir: &Path, args: &[&str]| {
            let ok = std::process::Command::new(&git).arg("-C").arg(dir).args(args)
                .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t")
                .output().unwrap().status.success();
            assert!(ok, "git {args:?} falhou");
        };
        std::fs::create_dir_all(&bare).unwrap();
        sh(&bare, &["init", "--bare", "-q"]);
        sh(&work, &["init", "-q", "-b", "prod"]);
        std::fs::write(work.join("a.txt"), "x").unwrap();
        sh(&work, &["add", "."]);
        sh(&work, &["commit", "-q", "-m", "um"]);
        sh(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
        sh(&work, &["push", "-q", "origin", "prod"]);

        assert_eq!(remote_url(&git, &work).await.as_deref(), bare.to_str());
        assert_eq!(local_branches(&git, &work).await, ["prod"]);
        let auth = build_auth(Kind::Sistema, None, None, bare.to_str().unwrap(), &kh()).unwrap();
        let msg = test_remote(&git, &auth).await.unwrap();
        assert!(msg.contains("1 branch"), "{msg}");

        let ruim = build_auth(Kind::Sistema, None, None, "/nao/existe.git", &kh()).unwrap();
        assert!(test_remote(&git, &ruim).await.is_err());
    }
}
