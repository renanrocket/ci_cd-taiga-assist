//! Tarefas em segundo plano (commit em lote, reenvio ao Taiga) e a caixa de notificação.
//!
//! Vivem em memória (fechar o app as perde; o que já foi gravado fica). Cada mudança é enviada
//! à janela pelo evento `task-update`, e a UI mantém a lista sincronizada.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

const MAX_TASKS: usize = 50;
const MAX_STEPS: usize = 200;

#[derive(Debug, Serialize, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub key: String,
    pub titulo: String,
    /// rodando | ok | erro
    pub estado: String,
    pub mensagem: String,
    pub passos: Vec<String>,
    pub resultado: Option<Value>,
    pub iniciado_em: String,
    pub terminado_em: Option<String>,
}

pub struct Tasks {
    app: Option<AppHandle>,
    seq: AtomicU64,
    inner: Mutex<Vec<TaskInfo>>,
}

fn now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

impl Tasks {
    pub fn new(app: AppHandle) -> Self {
        Tasks { app: Some(app), seq: AtomicU64::new(1), inner: Mutex::new(Vec::new()) }
    }

    /// Sem janela (testes).
    #[cfg(test)]
    pub fn detached() -> Self {
        Tasks { app: None, seq: AtomicU64::new(1), inner: Mutex::new(Vec::new()) }
    }

    fn emit(&self, t: &TaskInfo) {
        if let Some(a) = &self.app {
            let _ = a.emit("task-update", t);
        }
    }

    /// Inicia uma tarefa. Recusa se já há outra rodando com a mesma `key`.
    pub fn start(&self, key: &str, titulo: &str) -> Result<String, String> {
        let mut v = self.inner.lock().map_err(|_| "tarefas ocupadas".to_string())?;
        if v.iter().any(|t| t.key == key && t.estado == "rodando") {
            return Err("Já há uma tarefa igual em andamento; acompanhe pela caixa de notificações.".into());
        }
        let id = format!("t{}", self.seq.fetch_add(1, Ordering::Relaxed));
        let t = TaskInfo {
            id: id.clone(),
            key: key.to_owned(),
            titulo: titulo.to_owned(),
            estado: "rodando".into(),
            mensagem: "Iniciando…".into(),
            passos: Vec::new(),
            resultado: None,
            iniciado_em: now(),
            terminado_em: None,
        };
        v.insert(0, t.clone());
        // descarta as mais antigas já terminadas
        while v.len() > MAX_TASKS {
            if let Some(pos) = v.iter().rposition(|x| x.estado != "rodando") {
                v.remove(pos);
            } else {
                break;
            }
        }
        drop(v);
        self.emit(&t);
        Ok(id)
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut TaskInfo)) {
        let snapshot = {
            let Ok(mut v) = self.inner.lock() else { return };
            let Some(t) = v.iter_mut().find(|t| t.id == id) else { return };
            f(t);
            t.clone()
        };
        self.emit(&snapshot);
    }

    pub fn step(&self, id: &str, msg: &str) {
        self.update(id, |t| {
            t.mensagem = msg.to_owned();
            if t.passos.len() < MAX_STEPS {
                t.passos.push(format!("{} {msg}", now()));
            }
        });
    }

    pub fn finish(&self, id: &str, ok: bool, msg: &str, resultado: Option<Value>) {
        if !ok {
            tauri_plugin_log::log::warn!("tarefa {id} terminou com problema: {msg}");
        }
        self.update(id, |t| {
            t.estado = if ok { "ok" } else { "erro" }.into();
            t.mensagem = msg.to_owned();
            t.resultado = resultado;
            t.terminado_em = Some(now());
        });
    }

    pub fn list(&self) -> Vec<TaskInfo> {
        self.inner.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Remove uma tarefa terminada da caixa (as em andamento não podem ser dispensadas).
    pub fn dismiss(&self, id: &str) {
        if let Ok(mut v) = self.inner.lock() {
            v.retain(|t| !(t.id == id && t.estado != "rodando"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciclo_de_vida_e_chave_unica() {
        let t = Tasks::detached();
        let a = t.start("commit:1", "Commit").unwrap();
        assert!(t.start("commit:1", "Commit de novo").is_err(), "mesma chave rodando");
        let b = t.start("commit:2", "Outro").unwrap();
        assert_ne!(a, b);
        t.step(&a, "passo 1");
        t.step(&a, "passo 2");
        t.finish(&a, true, "pronto", Some(serde_json::json!({"x": 1})));
        let l = t.list();
        let ta = l.iter().find(|x| x.id == a).unwrap();
        assert_eq!((ta.estado.as_str(), ta.mensagem.as_str(), ta.passos.len()), ("ok", "pronto", 2));
        assert!(ta.terminado_em.is_some());
        // terminou: a chave pode ser reutilizada
        assert!(t.start("commit:1", "Commit").is_ok());
    }

    #[test]
    fn so_dispensa_tarefas_terminadas() {
        let t = Tasks::detached();
        let a = t.start("k", "a").unwrap();
        t.dismiss(&a);
        assert_eq!(t.list().len(), 1, "em andamento não some");
        t.finish(&a, false, "falhou", None);
        t.dismiss(&a);
        assert!(t.list().is_empty());
    }

    #[test]
    fn mais_recente_primeiro_e_limite() {
        let t = Tasks::detached();
        for i in 0..60 {
            let id = t.start(&format!("k{i}"), "x").unwrap();
            t.finish(&id, true, "ok", None);
        }
        let l = t.list();
        assert_eq!(l.len(), MAX_TASKS);
        assert_eq!(l[0].key, "k59");
    }
}
