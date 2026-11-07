//! Detecção de `git` e `kiro-cli`.
//!
//! App de GUI não herda o PATH do shell (macOS/Linux abrindo pelo menu; Windows idem): além do
//! PATH, procura em pastas conhecidas e respeita um caminho escolhido pelo usuário (settings).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Git,
    Kiro,
}

impl Tool {
    pub fn key(self) -> &'static str {
        match self {
            Tool::Git => "git",
            Tool::Kiro => "kiro",
        }
    }

    /// Chave em `settings` com o caminho escolhido pelo usuário.
    pub fn setting_key(self) -> &'static str {
        match self {
            Tool::Git => "tool_path.git",
            Tool::Kiro => "tool_path.kiro",
        }
    }

    fn binary_names(self) -> &'static [&'static str] {
        match self {
            Tool::Git => &["git"],
            Tool::Kiro => &["kiro-cli", "kiro-cli-chat"],
        }
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            Tool::Git => "Instale o Git: https://git-scm.com/downloads",
            Tool::Kiro => "Instale o Kiro CLI: https://kiro.dev/cli/ (macOS/Linux: curl -fsSL https://cli.kiro.dev/install | bash)",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ToolStatus {
    pub tool: &'static str,
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    /// `true` quando o caminho veio da configuração do usuário (e não da detecção).
    pub custom: bool,
    pub hint: &'static str,
    pub error: Option<String>,
}

/// Pastas onde as ferramentas costumam ficar mas que um app de GUI pode não ter no PATH.
fn extra_dirs(tool: Tool) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    if let Some(h) = &home {
        v.push(h.join(".local/bin"));
        v.push(h.join("bin"));
        v.push(h.join(".kiro/bin"));
    }
    if cfg!(target_os = "macos") {
        v.push("/opt/homebrew/bin".into());
        v.push("/usr/local/bin".into());
    }
    if cfg!(target_os = "linux") {
        v.push("/usr/local/bin".into());
        v.push("/usr/bin".into());
    }
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            v.push(local.join("Programs").join("Kiro CLI"));
            v.push(local.join("Programs").join("Git").join("cmd"));
        }
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(p) = std::env::var_os(var).map(PathBuf::from) {
                v.push(p.join("Git").join("cmd"));
            }
        }
    }
    if tool == Tool::Kiro {
        // nada específico além das pastas acima
    }
    v
}

/// Acha o executável: caminho do usuário > PATH > pastas conhecidas.
pub fn locate(tool: Tool, custom: Option<&str>) -> Option<(PathBuf, bool)> {
    if let Some(p) = custom.map(str::trim).filter(|s| !s.is_empty()) {
        let p = PathBuf::from(p);
        return p.is_file().then_some((p, true));
    }
    for name in tool.binary_names() {
        if let Ok(p) = which::which(name) {
            return Some((p, false));
        }
    }
    for dir in extra_dirs(tool) {
        for name in tool.binary_names() {
            if let Ok(p) = which::which_in(name, Some(&dir), &dir) {
                return Some((p, false));
            }
        }
    }
    None
}

/// Primeira linha não vazia da saída de `--version`.
pub fn first_line(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_owned)
}

async fn version_of(path: &Path) -> Result<String, String> {
    let mut cmd = Command::new(path);
    cmd.arg("--version").stdin(Stdio::null()).kill_on_drop(true);
    crate::proc::no_window(&mut cmd);
    let out = tokio::time::timeout(Duration::from_secs(10), cmd.output())
        .await
        .map_err(|_| "tempo esgotado ao executar --version".to_string())?
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("--version terminou com {}", out.status));
    }
    first_line(&String::from_utf8_lossy(&out.stdout))
        .ok_or_else(|| "--version não devolveu texto".to_string())
}

pub async fn status(tool: Tool, custom: Option<&str>) -> ToolStatus {
    let base = |found, path: Option<String>, version, custom, error| ToolStatus {
        tool: tool.key(),
        found,
        path,
        version,
        custom,
        hint: tool.install_hint(),
        error,
    };
    match locate(tool, custom) {
        None => {
            let err = custom
                .filter(|s| !s.trim().is_empty())
                .map(|p| format!("O arquivo informado não existe: {p}"));
            base(false, None, None, err.is_some(), err)
        }
        Some((path, is_custom)) => {
            let shown = Some(path.display().to_string());
            match version_of(&path).await {
                Ok(v) => base(true, shown, Some(v), is_custom, None),
                Err(e) => base(false, shown, None, is_custom, Some(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primeira_linha_ignora_vazias() {
        assert_eq!(first_line("\n  \nkiro-cli 2.22.1\nresto").as_deref(), Some("kiro-cli 2.22.1"));
        assert_eq!(first_line("   \n"), None);
    }

    #[test]
    fn caminho_do_usuario_inexistente_nao_cai_no_path() {
        assert!(locate(Tool::Git, Some("/nao/existe/git")).is_none());
    }

    #[test]
    fn caminho_do_usuario_vazio_usa_deteccao() {
        // com string vazia vale a detecção normal (não deve entrar no ramo "custom")
        if let Some((_, custom)) = locate(Tool::Git, Some("  ")) {
            assert!(!custom);
        }
    }

    #[tokio::test]
    async fn status_de_ferramenta_ausente() {
        let s = status(Tool::Kiro, Some("/nao/existe/kiro")).await;
        assert!(!s.found);
        assert!(s.custom);
        assert!(s.error.is_some());
    }
}
