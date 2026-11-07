//! Leitura dos cards (US) no Taiga e fechamento em lote — porte de `taiga_cards.py`.
//!
//! O Taiga é a fonte de verdade: título, descrição, status e prazo vêm da API (cache de 60 s por
//! projeto); as tabelas `reports`/`card_items` só dizem QUAIS histórias esta ferramenta trata.
//! Card apagado no Taiga vira `estado = "removido"`; falha de rede/credencial vira `estado = "erro"`:
//! o card nunca some da lista por causa disso.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::taiga::Client;

pub const PREVIEW_CHARS: usize = 150;
pub const CACHE_TTL: Duration = Duration::from_secs(60);
pub const DIAS_PROXIMO: i64 = 7;

// --------------------------------------------------------------------------- //
// Utilidades puras
// --------------------------------------------------------------------------- //

static MD_NOISE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|\s)(#{1,6}|[-*+>]|\d+\.)\s+|[`*_]{1,3}|!?\[([^\]]*)\]\([^)]*\)").unwrap());
static CLOSED_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)termin|done|conclu|finaliz|fechad|closed").unwrap());

/// Texto puro (sem marcações Markdown) cortado em `limit` caracteres + reticências.
pub fn preview(markdown: &str, limit: usize) -> String {
    let text = MD_NOISE.replace_all(markdown, |c: &regex::Captures| {
        let m0 = c.get(0).map_or("", |m| m.as_str());
        if m0.starts_with('!') || m0.starts_with('[') {
            c.get(3).map(|x| x.as_str()).filter(|s| !s.is_empty()).unwrap_or(" ").to_owned()
        } else {
            " ".to_owned()
        }
    });
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= limit {
        text
    } else {
        format!("{}…", text.chars().take(limit).collect::<String>().trim_end())
    }
}

/// (dias até o prazo, estado) — estado: fechado | sem_prazo | atrasado | hoje | proximo | ok.
pub fn prazo_info(due: Option<&str>, fechado: bool, today: NaiveDate) -> (Option<i64>, &'static str) {
    if fechado {
        return (None, "fechado");
    }
    let Some(due) = due.filter(|d| !d.is_empty()) else { return (None, "sem_prazo") };
    let head: String = due.chars().take(10).collect();
    let Ok(d) = NaiveDate::parse_from_str(&head, "%Y-%m-%d") else { return (None, "sem_prazo") };
    let dias = (d - today).num_days();
    match dias {
        d if d < 0 => (Some(d), "atrasado"),
        0 => (Some(0), "hoje"),
        d if d <= DIAS_PROXIMO => (Some(d), "proximo"),
        d => (Some(d), "ok"),
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct PontoPapel {
    pub papel: String,
    pub pontos: String,
}

/// Pontos da US por papel (UX, Design, Back, Front…). `us.points` mapeia role_id → point_id.
pub fn points_by_role(project: &Value, us: &Value) -> Vec<PontoPapel> {
    let arr = |k: &str| project.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let roles: HashMap<i64, String> = arr("roles")
        .iter()
        .filter(|r| r.get("computable").and_then(Value::as_bool).unwrap_or(true))
        .filter_map(|r| Some((r.get("id")?.as_i64()?, r.get("name")?.as_str()?.to_owned())))
        .collect();
    let pts: HashMap<i64, String> =
        arr("points").iter().filter_map(|p| Some((p.get("id")?.as_i64()?, p.get("name")?.as_str()?.to_owned()))).collect();
    let mut out: Vec<PontoPapel> = Vec::new();
    if let Some(m) = us.get("points").and_then(Value::as_object) {
        for (rid, pid) in m {
            let Ok(rid) = rid.parse::<i64>() else { continue };
            let Some(nome) = roles.get(&rid) else { continue };
            let pontos = pid.as_i64().and_then(|p| pts.get(&p).cloned()).unwrap_or_else(|| "?".into());
            out.push(PontoPapel { papel: nome.clone(), pontos });
        }
    }
    out.sort_by(|a, b| a.papel.cmp(&b.papel));
    out
}

/// A história está fechada? Vale o `is_closed` do STATUS no projeto: o flag da própria história fica
/// defasado em alguns casos (ex.: Done/Archived com `is_closed = false`).
pub fn story_closed(project: &Value, us: &Value) -> bool {
    let st = us.get("status").and_then(Value::as_i64);
    if let Some(s) = project.get("us_statuses").and_then(Value::as_array).and_then(|a| a.iter().find(|s| s.get("id").and_then(Value::as_i64) == st)) {
        return s.get("is_closed").and_then(Value::as_bool).unwrap_or(false);
    }
    us.get("is_closed").and_then(Value::as_bool).unwrap_or(false)
}

/// Status "fechado" do projeto (ids variam): entre os `is_closed`, prefere nome tipo Terminado/Done/
/// Concluído; senão o primeiro pela ordem. Sem nenhum → erro.
pub fn closing_status(project: &Value) -> Result<Value, String> {
    let mut closed: Vec<Value> = project
        .get("us_statuses")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter(|s| s.get("is_closed").and_then(Value::as_bool).unwrap_or(false)).cloned().collect())
        .unwrap_or_default();
    if closed.is_empty() {
        return Err("Projeto sem status de história fechado (ex.: Terminado).".into());
    }
    closed.sort_by_key(|s| {
        let nome = s.get("name").and_then(Value::as_str).unwrap_or("");
        (!CLOSED_NAME.is_match(nome), s.get("order").and_then(Value::as_i64).unwrap_or(0))
    });
    Ok(closed.swap_remove(0))
}

/// Link da US no Taiga (`<base>/project/<slug>/us/<ref>`).
pub fn web_url(host: &str, slug: &str, r: Option<i64>) -> Option<String> {
    let r = r?;
    let mut base = host.trim().trim_end_matches('/').to_owned();
    if !base.starts_with("http://") && !base.starts_with("https://") {
        base = format!("https://{base}");
    }
    let base = base.strip_suffix("/api/v1").unwrap_or(&base);
    Some(format!("{base}/project/{slug}/us/{r}"))
}

// --------------------------------------------------------------------------- //
// Acesso ao Taiga (clientes por conta + cache por projeto)
// --------------------------------------------------------------------------- //

/// Um cliente autenticado por conta, criado sob demanda e reaproveitado durante uma operação.
pub struct Hub {
    creds: HashMap<i64, (String, String, String)>,
    clients: tokio::sync::Mutex<HashMap<i64, Client>>,
}

impl Hub {
    /// `creds`: conta → (host, usuário, senha).
    pub fn new(creds: HashMap<i64, (String, String, String)>) -> Self {
        Hub { creds, clients: tokio::sync::Mutex::new(HashMap::new()) }
    }

    pub async fn client(&self, account_id: i64) -> Result<Client, String> {
        let mut g = self.clients.lock().await;
        if let Some(c) = g.get(&account_id) {
            return Ok(c.clone());
        }
        let (h, u, p) = self.creds.get(&account_id).ok_or("Conta Taiga não encontrada.")?;
        let mut c = Client::new(h, u, p).map_err(|e| e.message)?;
        c.login().await.map_err(|e| e.message)?;
        g.insert(account_id, c.clone());
        Ok(c)
    }

    pub fn host(&self, account_id: i64) -> String {
        self.creds.get(&account_id).map(|c| c.0.clone()).unwrap_or_default()
    }

    pub fn accounts(&self) -> Vec<(i64, String)> {
        self.creds.iter().map(|(k, v)| (*k, v.0.clone())).collect()
    }
}

#[derive(Clone)]
pub struct Snapshot {
    pub info: Value,
    pub stories: HashMap<i64, Value>,
}

struct Entry {
    snap: Arc<Snapshot>,
    at: Instant,
}

static CACHE: LazyLock<Mutex<HashMap<(i64, String), Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn evict(account: i64, slug: &str) {
    if let Ok(mut c) = CACHE.lock() {
        c.remove(&(account, slug.to_owned()));
    }
}

pub fn clear_cache() {
    if let Ok(mut c) = CACHE.lock() {
        c.clear();
    }
}

/// Projeto + todas as histórias (cache de 60 s). A descrição já baixada é reaproveitada quando a
/// `version` da história não mudou.
pub async fn load_project(hub: &Hub, account: i64, slug: &str, refresh: bool) -> Result<Arc<Snapshot>, String> {
    let key = (account, slug.to_owned());
    let old = {
        let c = CACHE.lock().map_err(|_| "cache ocupado".to_string())?;
        match c.get(&key) {
            Some(e) if !refresh && e.at.elapsed() < CACHE_TTL => return Ok(e.snap.clone()),
            Some(e) => Some(e.snap.clone()),
            None => None,
        }
    };
    let client = hub.client(account).await?;
    let info = client.project(slug).await.map_err(|e| e.message)?;
    let pid = info.get("id").and_then(Value::as_i64).ok_or("Projeto sem id.")?;
    let mut stories: HashMap<i64, Value> = HashMap::new();
    for mut s in client.userstories(pid).await.map_err(|e| e.message)? {
        let Some(id) = s.get("id").and_then(Value::as_i64) else { continue };
        if let Some(prev) = old.as_ref().and_then(|o| o.stories.get(&id)) {
            if prev.get("description").is_some() && prev.get("version") == s.get("version") {
                s["description"] = prev["description"].clone();
            }
        }
        stories.insert(id, s);
    }
    let snap = Arc::new(Snapshot { info, stories });
    if let Ok(mut c) = CACHE.lock() {
        c.insert(key, Entry { snap: snap.clone(), at: Instant::now() });
    }
    Ok(snap)
}

/// A LISTAGEM do Taiga não traz `description` (só o detalhe): busca o detalhe só dos cards pedidos
/// que ainda não têm, em paralelo (5), e guarda no cache.
pub async fn fill_descriptions(hub: &Hub, account: i64, slug: &str, snap: Arc<Snapshot>, ids: &[i64]) -> Arc<Snapshot> {
    let need: Vec<i64> = ids.iter().copied().filter(|i| snap.stories.get(i).is_some_and(|s| s.get("description").is_none())).collect();
    if need.is_empty() {
        return snap;
    }
    let Ok(client) = hub.client(account).await else { return snap };
    let sem = Arc::new(Semaphore::new(5));
    let mut set: JoinSet<(i64, Option<String>)> = JoinSet::new();
    for id in need {
        let (c, sem) = (client.clone(), sem.clone());
        set.spawn(async move {
            let _permit = sem.acquire_owned().await;
            (id, c.get_us(id).await.ok().map(|u| u.get("description").and_then(Value::as_str).unwrap_or("").to_owned()))
        });
    }
    let mut got: Vec<(i64, String)> = Vec::new();
    while let Some(Ok((id, d))) = set.join_next().await {
        if let Some(d) = d {
            got.push((id, d));
        }
    }
    if got.is_empty() {
        return snap;
    }
    let mut new = (*snap).clone();
    for (id, d) in got {
        if let Some(s) = new.stories.get_mut(&id) {
            s["description"] = Value::String(d);
        }
    }
    let new = Arc::new(new);
    if let Ok(mut c) = CACHE.lock() {
        if let Some(e) = c.get_mut(&(account, slug.to_owned())) {
            e.snap = new.clone();
        }
    }
    new
}

// --------------------------------------------------------------------------- //
// Cards
// --------------------------------------------------------------------------- //

/// Registro de `reports` de um card tratado por esta ferramenta.
#[derive(Clone, Debug, Default)]
pub struct ReportRow {
    pub id: i64,
    pub us_ref: Option<i64>,
    pub titulo: Option<String>,
    pub sha: String,
    pub branch: String,
    pub repo: String,
    pub criado_em: String,
    pub externo: bool,
    pub resumo_hash: Option<String>,
    pub tem_itens: bool,
}

/// De onde vem um card: relatório criado pela ferramenta (`report`) ou história EXTERNA.
#[derive(Clone, Debug)]
pub struct Src {
    pub account_id: Option<i64>,
    pub project_id: Option<i64>,
    pub slug: Option<String>,
    pub proj_nome: String,
    pub us_id: i64,
    pub report: Option<ReportRow>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Card {
    pub chave: String,
    pub report_id: Option<i64>,
    pub project_id: Option<i64>,
    pub us_id: i64,
    #[serde(rename = "ref")]
    pub ref_: Option<i64>,
    pub repo: String,
    pub sha: String,
    pub branch: String,
    pub slug: Option<String>,
    pub criado_em: Option<String>,
    pub criado_sort: String,
    pub titulo: String,
    pub descricao: String,
    pub resumo: String,
    pub projeto: String,
    pub status: Option<String>,
    pub status_cor: Option<String>,
    pub fechado: bool,
    pub prazo: Option<String>,
    pub dias: Option<i64>,
    pub prazo_estado: String,
    pub url: Option<String>,
    /// ok | erro | removido
    pub estado: String,
    pub erro: Option<String>,
    pub status_final: Option<String>,
    pub selecionavel: bool,
    pub criado_taiga: Option<String>,
    pub descricao_carregada: bool,
    pub resumo_pronto: bool,
    pub externo: bool,
    pub criado_por: Option<String>,
    pub pontos_papeis: Vec<PontoPapel>,
    pub pontos_total: Option<f64>,
}

/// Um card por fonte, na mesma ordem. Nunca falha por causa do Taiga.
/// `descriptions = false` pula o detalhe de cada US (usado só para ordenar/contar).
pub async fn cards_for(hub: &Hub, sources: &[Src], today: NaiveDate, descriptions: bool, refresh: bool) -> Vec<Card> {
    let mut wanted: HashMap<(i64, String), Vec<i64>> = HashMap::new();
    for s in sources {
        if let (Some(a), Some(sl)) = (s.account_id, &s.slug) {
            wanted.entry((a, sl.clone())).or_default().push(s.us_id);
        }
    }
    let mut projects: HashMap<(i64, String), Result<Arc<Snapshot>, String>> = HashMap::new();
    for ((a, sl), ids) in &wanted {
        match load_project(hub, *a, sl, refresh).await {
            Ok(snap) => {
                let snap = if descriptions { fill_descriptions(hub, *a, sl, snap, ids).await } else { snap };
                projects.insert((*a, sl.clone()), Ok(snap));
            }
            Err(e) => {
                projects.insert((*a, sl.clone()), Err(e));
            }
        }
    }
    sources.iter().map(|s| build_card(hub, s, &projects, today)).collect()
}

fn build_card(hub: &Hub, s: &Src, projects: &HashMap<(i64, String), Result<Arc<Snapshot>, String>>, today: NaiveDate) -> Card {
    let rep = s.report.as_ref();
    let mut c = Card {
        chave: rep.map_or_else(|| format!("x{}-{}", s.project_id.unwrap_or(0), s.us_id), |r| r.id.to_string()),
        report_id: rep.map(|r| r.id),
        project_id: s.project_id,
        us_id: s.us_id,
        ref_: rep.and_then(|r| r.us_ref),
        repo: rep.map(|r| r.repo.clone()).unwrap_or_default(),
        sha: rep.map(|r| r.sha.clone()).unwrap_or_default(),
        branch: rep.map(|r| r.branch.clone()).unwrap_or_default(),
        slug: s.slug.clone(),
        criado_em: rep.map(|r| r.criado_em.clone()),
        criado_sort: rep.map(|r| r.criado_em.clone()).unwrap_or_default(),
        titulo: rep
            .and_then(|r| r.titulo.clone())
            .unwrap_or_else(|| format!("US #{}", rep.and_then(|r| r.us_ref).unwrap_or(s.us_id))),
        descricao: String::new(),
        resumo: String::new(),
        projeto: s.proj_nome.clone(),
        status: None,
        status_cor: None,
        fechado: false,
        prazo: None,
        dias: None,
        prazo_estado: "sem_prazo".into(),
        url: None,
        estado: "ok".into(),
        erro: None,
        status_final: None,
        selecionavel: false,
        criado_taiga: None,
        descricao_carregada: false,
        resumo_pronto: rep.is_some_and(|r| r.resumo_hash.is_some() && r.tem_itens),
        externo: rep.map_or(true, |r| r.externo),
        criado_por: None,
        pontos_papeis: Vec::new(),
        pontos_total: None,
    };
    let (Some(account), Some(slug)) = (s.account_id, &s.slug) else {
        c.estado = "erro".into();
        c.erro = Some("Projeto Taiga não disponível (foi removido das configurações?).".into());
        return c;
    };
    let snap = match projects.get(&(account, slug.clone())) {
        Some(Ok(s)) => s,
        Some(Err(e)) => {
            c.estado = "erro".into();
            c.erro = Some(e.clone());
            return c;
        }
        None => return c,
    };
    if let Some(n) = snap.info.get("name").and_then(Value::as_str) {
        c.projeto = n.to_owned();
    }
    c.status_final = closing_status(&snap.info).ok().and_then(|s| s.get("name").and_then(Value::as_str).map(str::to_owned));
    let Some(us) = snap.stories.get(&s.us_id) else {
        c.estado = "removido".into();
        c.erro = Some("Esta história não existe mais no Taiga.".into());
        return c;
    };
    let r = us.get("ref").and_then(Value::as_i64).or(c.ref_);
    c.ref_ = r;
    c.url = web_url(&hub.host(account), slug, r);
    let fechado = story_closed(&snap.info, us);
    let due = us.get("due_date").and_then(Value::as_str);
    let (dias, pe) = prazo_info(due, fechado, today);
    let desc = us.get("description").and_then(Value::as_str);
    let info = us.get("status_extra_info");
    let owner = us.get("owner_extra_info");
    c.pontos_papeis = points_by_role(&snap.info, us);
    c.pontos_total = us.get("total_points").and_then(Value::as_f64);
    c.titulo = us.get("subject").and_then(Value::as_str).map_or(c.titulo.clone(), str::to_owned);
    c.descricao = desc.unwrap_or("").to_owned();
    c.resumo = preview(desc.unwrap_or(""), PREVIEW_CHARS);
    c.status = info.and_then(|i| i.get("name")).and_then(Value::as_str).map(str::to_owned);
    c.status_cor = info.and_then(|i| i.get("color")).and_then(Value::as_str).map(str::to_owned);
    c.fechado = fechado;
    c.prazo = due.map(str::to_owned);
    c.dias = dias;
    c.prazo_estado = pe.into();
    let created = us.get("created_date").and_then(Value::as_str).unwrap_or("");
    c.criado_taiga = (!created.is_empty()).then(|| created.chars().take(10).collect());
    if !created.is_empty() {
        c.criado_sort = created.to_owned();
    }
    c.descricao_carregada = desc.is_some();
    c.criado_por = owner
        .and_then(|o| o.get("full_name_display").and_then(Value::as_str).filter(|s| !s.is_empty()).or_else(|| o.get("username").and_then(Value::as_str)))
        .map(str::to_owned);
    // Externo ainda não adotado também é selecionável: é "adotado" ao pedir o resumo.
    c.selecionavel = !fechado && c.status_final.is_some();
    c
}

/// ABERTOS primeiro; depois indisponíveis, fechados e removidos. Dentro de cada grupo, os mais
/// recentes antes. O status só existe no Taiga, então isto vem ANTES da paginação (custa 1 chamada
/// por projeto, cache de 60 s, e NÃO busca descrições).
pub async fn order_sources(hub: &Hub, sources: Vec<Src>, today: NaiveDate, refresh: bool) -> Vec<Src> {
    let cards = cards_for(hub, &sources, today, false, refresh).await;
    let rank = |c: &Card| match c.estado.as_str() {
        "ok" => {
            if c.fechado { 2 } else { 0 }
        }
        "erro" => 1,
        _ => 3,
    };
    let mut pairs: Vec<(Card, Src)> = cards.into_iter().zip(sources).collect();
    pairs.sort_by(|a, b| b.0.criado_sort.cmp(&a.0.criado_sort)); // mais recentes antes
    pairs.sort_by_key(|(c, _)| rank(c)); // estável: mantém "mais recentes antes" em cada grupo
    pairs.into_iter().map(|(_, s)| s).collect()
}

// --------------------------------------------------------------------------- //
// Fechar em lote
// --------------------------------------------------------------------------- //

#[derive(Debug, Serialize, Clone)]
pub struct CloseResult {
    pub id: Option<i64>,
    #[serde(rename = "ref")]
    pub ref_: Option<i64>,
    pub titulo: String,
    pub projeto: String,
    pub ok: bool,
    pub detail: String,
}

pub struct CloseReq {
    pub report_id: i64,
    pub us_id: i64,
    pub us_ref: Option<i64>,
    pub titulo: String,
    pub account_id: Option<i64>,
    pub slug: Option<String>,
    pub projeto: String,
}

static INFLIGHT: LazyLock<Mutex<HashSet<i64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

async fn close_one(client: &Client, req: &CloseReq, target: &Value, project: &Value) -> CloseResult {
    let mut out = CloseResult { id: Some(req.report_id), ref_: req.us_ref, titulo: req.titulo.clone(), projeto: req.projeto.clone(), ok: false, detail: String::new() };
    for attempt in 1..=2 {
        let us = match client.get_us(req.us_id).await {
            Ok(u) => u,
            Err(e) => {
                out.detail = if e.status == Some(404) { "Esta história não existe mais no Taiga.".into() } else { e.message };
                return out;
            }
        };
        if let Some(t) = us.get("subject").and_then(Value::as_str) {
            out.titulo = t.to_owned();
        }
        if story_closed(project, &us) {
            out.ok = true;
            out.detail = "Já estava fechada no Taiga.".into();
            return out;
        }
        let payload = json!({"status": target["id"], "version": us.get("version")});
        match client.patch_us(req.us_id, &payload).await {
            Ok(_) => {
                out.ok = true;
                out.detail = format!("Movida para “{}”.", target.get("name").and_then(Value::as_str).unwrap_or("?"));
                return out;
            }
            // alguém editou entre o GET e o PATCH: tenta uma vez de novo
            Err(e) if e.status == Some(409) && attempt == 1 => continue,
            Err(e) => {
                out.detail = e.message;
                return out;
            }
        }
    }
    out
}

/// Move cada US para o status fechado do SEU projeto. Resultado parcial por card; nunca falha.
/// Um card já em andamento (clique duplo) é recusado em vez de repetido.
pub async fn close_cards(hub: &Hub, reqs: Vec<CloseReq>) -> Vec<CloseResult> {
    let mut results: Vec<CloseResult> = Vec::new();
    let mut claimed: Vec<CloseReq> = Vec::new();
    {
        let mut g = INFLIGHT.lock().unwrap_or_else(|e| e.into_inner());
        for r in reqs {
            if g.insert(r.report_id) {
                claimed.push(r);
            } else {
                results.push(CloseResult { id: Some(r.report_id), ref_: r.us_ref, titulo: r.titulo, projeto: r.projeto, ok: false, detail: "Já está sendo fechado.".into() });
            }
        }
    }
    let fail = |r: &CloseReq, d: &str| CloseResult { id: Some(r.report_id), ref_: r.us_ref, titulo: r.titulo.clone(), projeto: r.projeto.clone(), ok: false, detail: d.to_owned() };
    let mut groups: HashMap<(i64, String), Vec<&CloseReq>> = HashMap::new();
    for r in &claimed {
        match (r.account_id, &r.slug) {
            (Some(a), Some(s)) => groups.entry((a, s.clone())).or_default().push(r),
            _ => results.push(fail(r, "Credencial ou projeto Taiga não disponível.")),
        }
    }
    for ((account, slug), items) in groups {
        let prep = async {
            let client = hub.client(account).await?;
            let project = client.project(&slug).await.map_err(|e| e.message)?;
            let target = closing_status(&project)?;
            Ok::<_, String>((client, project, target))
        }
        .await;
        match prep {
            Ok((client, project, target)) => {
                for r in items {
                    results.push(close_one(&client, r, &target, &project).await);
                }
            }
            Err(e) => results.extend(items.into_iter().map(|r| fail(r, &e))),
        }
        evict(account, &slug); // a lista seguinte reflete o novo status
    }
    let mut g = INFLIGHT.lock().unwrap_or_else(|e| e.into_inner());
    for r in &claimed {
        g.remove(&r.report_id);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn d(y: i32, m: u32, dd: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, dd).unwrap()
    }

    #[test]
    fn preview_tira_markdown_e_corta() {
        assert_eq!(preview("## Título\n\n- item **forte** e `código`\n1. outro [link](http://x) fim", 150), "Título item forte e código outro link fim");
        assert_eq!(preview("![img](u) texto", 150), "img texto", "mantém o texto alternativo, como o ci_cd");
        assert_eq!(preview("", 10), "");
        let longo = "palavra ".repeat(50);
        let p = preview(&longo, 20);
        assert!(p.ends_with('…') && p.chars().count() <= 21, "{p}");
    }

    #[test]
    fn prazo_estados() {
        let t = d(2026, 11, 9);
        assert_eq!(prazo_info(Some("2026-11-09"), false, t), (Some(0), "hoje"));
        assert_eq!(prazo_info(Some("2026-11-08"), false, t), (Some(-1), "atrasado"));
        assert_eq!(prazo_info(Some("2026-11-16"), false, t), (Some(7), "proximo"));
        assert_eq!(prazo_info(Some("2026-11-17T00:00:00Z"), false, t), (Some(8), "ok"));
        assert_eq!(prazo_info(None, false, t), (None, "sem_prazo"));
        assert_eq!(prazo_info(Some("lixo"), false, t), (None, "sem_prazo"));
        assert_eq!(prazo_info(Some("2026-11-01"), true, t), (None, "fechado"));
    }

    fn projeto() -> Value {
        json!({
            "id": 7, "name": "Plataforma",
            "roles": [{"id": 1, "name": "UX"}, {"id": 2, "name": "Back"}, {"id": 3, "name": "Gerente", "computable": false}],
            "points": [{"id": 10, "name": "1"}, {"id": 11, "name": "5"}],
            "us_statuses": [
                {"id": 1, "name": "Novo", "is_closed": false, "order": 1},
                {"id": 2, "name": "Arquivado", "is_closed": true, "order": 9},
                {"id": 3, "name": "Done", "is_closed": true, "order": 8}
            ]
        })
    }

    #[test]
    fn pontos_por_papel_ignora_nao_computaveis() {
        let us = json!({"points": {"1": 10, "2": 11, "3": 10, "x": 10, "2x": 99}});
        assert_eq!(
            points_by_role(&projeto(), &us),
            vec![PontoPapel { papel: "Back".into(), pontos: "5".into() }, PontoPapel { papel: "UX".into(), pontos: "1".into() }]
        );
    }

    #[test]
    fn fechado_pelo_status_do_projeto_e_status_de_fechamento() {
        let p = projeto();
        assert!(story_closed(&p, &json!({"status": 2, "is_closed": false})), "o flag do status vence o da US");
        assert!(!story_closed(&p, &json!({"status": 1, "is_closed": true})));
        assert!(story_closed(&p, &json!({"status": 99, "is_closed": true})), "status desconhecido: cai no flag da US");
        // prefere o nome tipo Done mesmo com ordem maior
        assert_eq!(closing_status(&p).unwrap()["id"], 3);
        let sem = json!({"us_statuses": [{"id": 1, "is_closed": false}]});
        assert!(closing_status(&sem).unwrap_err().contains("status de história fechado"));
        let so_arquivado = json!({"us_statuses": [{"id": 5, "name": "Arquivado", "is_closed": true, "order": 3}, {"id": 6, "name": "Outro", "is_closed": true, "order": 1}]});
        assert_eq!(closing_status(&so_arquivado).unwrap()["id"], 6, "sem nome conhecido: o primeiro pela ordem");
    }

    #[test]
    fn link_da_us() {
        assert_eq!(web_url("taiga.x.com/", "p", Some(3)).as_deref(), Some("https://taiga.x.com/project/p/us/3"));
        assert_eq!(web_url("https://t.x/api/v1", "p", Some(3)).as_deref(), Some("https://t.x/project/p/us/3"));
        assert_eq!(web_url("t.x", "p", None), None);
    }

    fn mock_taiga(server: &MockServer) {
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/auth");
            then.status(200).json_body(json!({"auth_token": "T", "id": 42}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/projects/by_slug").query_param("slug", "plat");
            then.status(200).json_body(projeto());
        });
    }

    fn hub(server: &MockServer) -> Hub {
        hub_id(server, 1)
    }

    /// O cache de projetos é global: cada teste usa um id de conta próprio para não colidir.
    fn hub_id(server: &MockServer, id: i64) -> Hub {
        Hub::new(HashMap::from([(id, (server.base_url(), "u".to_string(), "s".to_string()))]))
    }

    #[tokio::test]
    async fn lista_cards_abertos_primeiro_e_descricao_so_do_que_for_pedido() {
        let server = MockServer::start();
        mock_taiga(&server);
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories").query_param("project", "7").header("x-disable-pagination", "True");
            then.status(200).json_body(json!([
                {"id": 100, "ref": 1, "subject": "Aberta", "status": 1, "version": 1, "due_date": "2026-11-12", "created_date": "2026-11-01T10:00:00Z", "total_points": 6.0, "points": {"1": 10, "2": 11}, "status_extra_info": {"name": "Novo", "color": "#fff"}, "owner_extra_info": {"full_name_display": "Ana"}},
                {"id": 101, "ref": 2, "subject": "Fechada", "status": 3, "version": 1, "created_date": "2026-11-02T10:00:00Z"}
            ]));
        });
        let det = server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/100");
            then.status(200).json_body(json!({"id": 100, "description": "## Feito\n- coisa"}));
        });
        let h = hub_id(&server, 11);
        let mk = |us_id| Src { account_id: Some(11), project_id: Some(1), slug: Some("plat".into()), proj_nome: "Plat".into(), us_id, report: None };
        let srcs = vec![mk(101), mk(100), mk(999)];
        let ordered = order_sources(&h, srcs, d(2026, 11, 9), true).await;
        assert_eq!(ordered.iter().map(|s| s.us_id).collect::<Vec<_>>(), [100, 101, 999], "aberta, fechada, removida");
        assert_eq!(det.hits(), 0, "ordenar não baixa descrições");

        let cards = cards_for(&h, &ordered[..1], d(2026, 11, 9), true, false).await;
        let c = &cards[0];
        assert_eq!((c.estado.as_str(), c.titulo.as_str(), c.projeto.as_str()), ("ok", "Aberta", "Plataforma"));
        assert_eq!((c.dias, c.prazo_estado.as_str(), c.status_final.as_deref()), (Some(3), "proximo", Some("Done")));
        assert_eq!((c.descricao_carregada, c.resumo.as_str(), c.criado_por.as_deref()), (true, "Feito coisa", Some("Ana")));
        assert!(c.selecionavel && c.externo);
        assert_eq!(c.url.as_deref().map(|u| u.ends_with("/project/plat/us/1")), Some(true));
        assert_eq!(c.pontos_total, Some(6.0));
        det.assert_hits(1);

        let removido = cards_for(&h, &ordered[2..], d(2026, 11, 9), false, false).await;
        assert_eq!(removido[0].estado, "removido");
    }

    #[tokio::test]
    async fn taiga_fora_do_ar_vira_erro_no_card_e_nao_some() {
        let h = Hub::new(HashMap::from([(9, ("http://127.0.0.1:1".to_string(), "u".to_string(), "s".to_string()))]));
        let s = Src { account_id: Some(9), project_id: Some(1), slug: Some("x".into()), proj_nome: "X".into(), us_id: 5, report: None };
        let c = &cards_for(&h, &[s], d(2026, 11, 9), true, false).await[0];
        assert_eq!(c.estado, "erro");
        assert!(c.erro.is_some() && !c.selecionavel);
        let sem_projeto = Src { account_id: None, project_id: None, slug: None, proj_nome: "?".into(), us_id: 6, report: None };
        assert_eq!(cards_for(&h, &[sem_projeto], d(2026, 11, 9), true, false).await[0].estado, "erro");
    }

    #[tokio::test]
    async fn cache_de_60s_e_descricao_reaproveitada_se_a_versao_nao_mudou() {
        let server = MockServer::start();
        mock_taiga(&server);
        let lista = server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories");
            then.status(200).json_body(json!([{"id": 100, "ref": 1, "subject": "A", "status": 1, "version": 3}]));
        });
        let det = server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/100");
            then.status(200).json_body(json!({"id": 100, "description": "texto"}));
        });
        let h = hub_id(&server, 12);
        let s = Src { account_id: Some(12), project_id: Some(1), slug: Some("plat".into()), proj_nome: "P".into(), us_id: 100, report: None };
        cards_for(&h, &[s.clone()], d(2026, 11, 9), true, false).await;
        cards_for(&h, &[s.clone()], d(2026, 11, 9), true, false).await; // dentro do TTL
        assert_eq!(lista.hits(), 1, "cache");
        let c = &cards_for(&h, &[s], d(2026, 11, 9), true, true).await[0]; // refresh: relista, mas mesma versão
        assert_eq!(lista.hits(), 2);
        assert_eq!(det.hits(), 1, "descrição reaproveitada");
        assert_eq!(c.descricao, "texto");
    }

    fn req(id: i64) -> CloseReq {
        CloseReq { report_id: id, us_id: 100 + id, us_ref: Some(id), titulo: format!("T{id}"), account_id: Some(1), slug: Some("plat".into()), projeto: "Plat".into() }
    }

    #[tokio::test]
    async fn fechar_move_para_o_status_fechado_com_retry_em_409_e_trata_404_e_ja_fechada() {
        let server = MockServer::start();
        mock_taiga(&server);
        // 101: ok na 1ª; 102: 409 e depois ok; 103: já fechada; 104: 404
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/101");
            then.status(200).json_body(json!({"id": 101, "subject": "Um", "status": 1, "version": 4}));
        });
        let patch1 = server.mock(|when, then| {
            when.method(PATCH).path("/api/v1/userstories/101").json_body(json!({"status": 3, "version": 4}));
            then.status(200).json_body(json!({}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/102");
            then.status(200).json_body(json!({"id": 102, "status": 1, "version": 1}));
        });
        let mut conflito = server.mock(|when, then| {
            when.method(PATCH).path("/api/v1/userstories/102");
            then.status(409).json_body(json!({"_error_message": "version"}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/103");
            then.status(200).json_body(json!({"id": 103, "status": 2, "version": 1}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/104");
            then.status(404).json_body(json!({"_error_message": "nf"}));
        });
        let h = hub(&server);
        let _ = &mut conflito; // 409 na primeira tentativa; troca o mock antes da segunda
        let primeira = close_cards(&h, vec![req(2)]).await;
        assert!(!primeira[0].ok, "409 duas vezes seguidas = falha: {primeira:?}");
        conflito.delete();
        server.mock(|when, then| {
            when.method(PATCH).path("/api/v1/userstories/102");
            then.status(200).json_body(json!({}));
        });
        let r = close_cards(&h, vec![req(1), req(2), req(3), req(4)]).await;
        let by = |i: i64| r.iter().find(|x| x.id == Some(i)).unwrap();
        assert!(by(1).ok && by(1).detail.contains("Movida para “Done”"), "{r:?}");
        assert!(by(2).ok, "{r:?}");
        assert!(by(3).ok && by(3).detail.contains("Já estava fechada"));
        assert!(!by(4).ok && by(4).detail.contains("não existe mais"));
        assert_eq!(by(1).titulo, "Um", "título atualizado do Taiga");
        patch1.assert();
    }

    #[tokio::test]
    async fn fechar_duas_vezes_ao_mesmo_tempo_recusa_a_segunda() {
        let server = MockServer::start();
        mock_taiga(&server);
        let h = hub(&server);
        INFLIGHT.lock().unwrap().insert(555);
        let r = close_cards(&h, vec![CloseReq { report_id: 555, ..req(5) }]).await;
        assert!(!r[0].ok && r[0].detail.contains("Já está sendo fechado"));
        INFLIGHT.lock().unwrap().remove(&555);
    }

    #[tokio::test]
    async fn conta_ou_projeto_ausente_nao_fecha() {
        let h = Hub::new(HashMap::new());
        let mut q = req(7);
        q.account_id = None;
        assert!(!close_cards(&h, vec![q]).await[0].ok);
        let r = close_cards(&h, vec![req(8)]).await;
        assert!(!r[0].ok && r[0].detail.contains("Conta Taiga não encontrada"));
    }
}
