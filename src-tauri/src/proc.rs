//! Utilidades para processos filhos (git, kiro-cli).

use tokio::process::Command;

/// No Windows, evita abrir uma janela de console a cada processo filho.
pub fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}
