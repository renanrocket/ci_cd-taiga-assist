//! Comandos Tauri de "Taiga cards": lista, resumo (Kiro) + revisão, fechar em lote e prestação de contas.
//!
//! Fechar e exportar ALTERAM cards no Taiga: o fechamento só vale para quem tem resumo gerado (falha
//! do Kiro nunca resulta em card fechado) e a exportação só marca os pontos como prestados se a
//! edição da US de destino deu certo.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::cards::{self, Card, CloseReq, Hub, ReportRow, Src};
use crate::error::{AppError, AppResult};
use crate::export::{self, Item};
use crate::pipeline::KiroCfg;
use crate::taigadb::ProjectInfo;
use crate::{secrets, summary, taigadb, with_db, AppState};

const PER_PAGE: usize = 20;
const MAX_CLOSE: usize = 100;

fn build_hub(state: &AppState) -> AppResult<Hub> {
    let rows: Vec<(i64, String, String, String)> = with_db(state, |c| {
        let mut st = c.prepare("SELECT id, host, usuario, secret_ref FROM taiga_accounts")?;
        let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect::<Result<_, _>>()?;
        Ok(r)
    })?;
    let mut m = HashMap::new();
    for (id, host, user, sref) in rows {
        if let Some(pass) = secrets::get(&sref)? {
            m.insert(id, (host, user, pass));
        }
    }
    Ok(Hub::new(m))
}

fn proj_map(state: &AppState) -> AppResult<HashMap<i64, ProjectInfo>> {
    Ok(with_db(state, taigadb::list_projects)?.into_iter().map(|p| (p.id, p)).collect())
}

struct Rec {
    row: ReportRow,
    project_id: Option<i64>,
    us_id: i64,
}

fn load_reports(conn: &Connection, ids: Option<&[i64]>) -> AppResult<Vec<Rec>> {
    let mut sql = String::from(
        "SELECT r.id, r.us_id, r.us_ref, r.titulo, r.sha, r.branch, r.criado_em, r.externo, r.resumo_hash, r.project_id,
                COALESCE(lr.rel, ''), COALESCE(lf.caminho, ''),
                EXISTS(SELECT 1 FROM card_items ci WHERE ci.report_id = r.id)
           FROM reports r
           LEFT JOIN local_repos lr ON lr.id = r.repo_id
           LEFT JOIN local_folders lf ON lf.id = lr.folder_id",
    );
    if let Some(ids) = ids {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        sql.push_str(&format!(" WHERE r.id IN ({})", vec!["?"; ids.len()].join(",")));
    }
    sql.push_str(" ORDER BY r.id");
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map(rusqlite::params_from_iter(ids.unwrap_or(&[]).iter()), |r| {
        let (rel, caminho): (String, String) = (r.get(10)?, r.get(11)?);
        let repo = if rel == "." {
            std::path::Path::new(&caminho).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        } else {
            rel
        };
        Ok(Rec {
            us_id: r.get(1)?,
            project_id: r.get(9)?,
            row: ReportRow {
                id: r.get(0)?,
                us_ref: r.get(2)?,
                titulo: r.get(3)?,
                sha: r.get(4)?,
                branch: r.get(5)?,
                criado_em: r.get(6)?,
                externo: r.get::<_, i64>(7)? != 0,
                resumo_hash: r.get(8)?,
                repo,
                tem_itens: r.get(12)?,
            },
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn src_of(rec: &Rec, projects: &HashMap<i64, ProjectInfo>) -> Src {
    let p = rec.project_id.and_then(|id| projects.get(&id));
    Src {
        account_id: p.map(|p| p.account_id),
        project_id: rec.project_id,
        slug: p.map(|p| p.slug.clone()),
        proj_nome: p.map(|p| p.nome.clone()).unwrap_or_default(),
        us_id: rec.us_id,
        report: Some(rec.row.clone()),
    }
}

#[derive(Serialize)]
pub struct ProjetoOpt {
    id: i64,
    nome: String,
    conta: String,
}

#[derive(Serialize)]
pub struct CardsPage {
    cards: Vec<Card>,
    page: usize,
    pages: usize,
    total: usize,
    projetos: Vec<ProjetoOpt>,
}

/// Histórias dos projetos importados que NÃO foram criadas por esta ferramenta. Por padrão só as
/// ABERTAS são interessantes (as fechadas são centenas); projeto indisponível não gera externos
/// (os cards da ferramenta já mostram o erro).
async fn external_sources(hub: &Hub, projects: &[&ProjectInfo], known: &HashSet<(i64, i64)>, include_closed: bool, refresh: bool) -> Vec<Src> {
    let mut out = Vec::new();
    for p in projects {
        let Ok(snap) = cards::load_project(hub, p.account_id, &p.slug, refresh).await else { continue };
        for (us_id, us) in &snap.stories {
            if known.contains(&(p.id, *us_id)) || (!include_closed && cards::story_closed(&snap.info, us)) {
                continue;
            }
            out.push(Src { account_id: Some(p.account_id), project_id: Some(p.id), slug: Some(p.slug.clone()), proj_nome: p.nome.clone(), us_id: *us_id, report: None });
        }
    }
    out
}

#[tauri::command]
pub async fn cards_list(
    state: tauri::State<'_, AppState>,
    page: usize,
    project_id: Option<i64>,
    origem: String,
    ocultar_fechados: bool,
    refresh: bool,
) -> AppResult<CardsPage> {
    let hub = build_hub(&state)?;
    let projects = proj_map(&state)?;
    let today = chrono::Local::now().date_naive();
    let mut sources: Vec<Src> = Vec::new();
    let reports = with_db(&state, |c| load_reports(c, None))?;
    if origem != "externo" {
        sources.extend(reports.iter().filter(|r| project_id.is_none() || r.project_id == project_id).map(|r| src_of(r, &projects)));
    }
    if origem != "ferramenta" {
        let known: HashSet<(i64, i64)> = reports.iter().filter_map(|r| Some((r.project_id?, r.us_id))).collect();
        let mut sel: Vec<&ProjectInfo> = projects.values().filter(|p| project_id.is_none() || Some(p.id) == project_id).collect();
        sel.sort_by_key(|p| p.id);
        sources.extend(external_sources(&hub, &sel, &known, !ocultar_fechados, refresh).await);
    }
    let sources = cards::order_sources(&hub, sources, today, refresh).await;
    let total = sources.len();
    let pages = total.div_ceil(PER_PAGE).max(1);
    let page = page.clamp(1, pages);
    let slice = &sources[(page - 1) * PER_PAGE..(page * PER_PAGE).min(total)];
    let cards = cards::cards_for(&hub, slice, today, true, false).await;
    let mut projetos: Vec<ProjetoOpt> = projects.values().map(|p| ProjetoOpt { id: p.id, nome: p.nome.clone(), conta: p.account_nome.clone() }).collect();
    projetos.sort_by_key(|p| p.nome.to_lowercase());
    Ok(CardsPage { cards, page, pages, total, projetos })
}

// --------------------------------------------------------------------------- //
// Resumo (Kiro) em segundo plano → revisão → fechar
// --------------------------------------------------------------------------- //

#[derive(Deserialize, Clone)]
pub struct CardRef {
    report_id: Option<i64>,
    project_id: Option<i64>,
    us_id: i64,
}

/// Transforma cards externos selecionados em registros de `reports` ("adoção"), para poderem ser
/// resumidos, fechados e prestados como os da ferramenta. Só aceita projeto importado, história
/// existente e ainda ABERTA; reaproveita o registro se já foi adotado. → (ids, avisos).
async fn adopt(hub: &Hub, state: &AppState, refs: &[CardRef]) -> (Vec<i64>, Vec<String>) {
    let projects = proj_map(state).unwrap_or_default();
    let mut ids: Vec<i64> = Vec::new();
    let mut avisos: Vec<String> = Vec::new();
    let mut seen: HashSet<(Option<i64>, Option<i64>, i64)> = HashSet::new();
    for r in refs {
        if !seen.insert((r.report_id, r.project_id, r.us_id)) {
            continue;
        }
        if let Some(id) = r.report_id {
            let ok = with_db(state, |c| Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM reports WHERE id = ?1)", [id], |x| x.get::<_, bool>(0))?)).unwrap_or(false);
            if ok {
                ids.push(id);
            } else {
                avisos.push("US não encontrada.".into());
            }
            continue;
        }
        let Some(p) = r.project_id.and_then(|id| projects.get(&id)) else {
            avisos.push("US externa de um projeto que não está importado.".into());
            continue;
        };
        let existing = with_db(state, |c| {
            Ok(c.query_row("SELECT id FROM reports WHERE project_id = ?1 AND us_id = ?2", params![p.id, r.us_id], |x| x.get::<_, i64>(0)).optional()?)
        })
        .unwrap_or(None);
        if let Some(id) = existing {
            ids.push(id);
            continue;
        }
        let snap = match cards::load_project(hub, p.account_id, &p.slug, false).await {
            Ok(s) => s,
            Err(e) => {
                avisos.push(e);
                continue;
            }
        };
        let Some(us) = snap.stories.get(&r.us_id) else {
            avisos.push(format!("US {} não existe mais no Taiga.", r.us_id));
            continue;
        };
        let us_ref = us.get("ref").and_then(|v| v.as_i64());
        if cards::story_closed(&snap.info, us) {
            avisos.push(format!("US #{} já está fechada.", us_ref.unwrap_or(r.us_id)));
            continue;
        }
        let titulo: String = us.get("subject").and_then(|v| v.as_str()).unwrap_or("").chars().take(512).collect();
        let ins = with_db(state, |c| {
            c.execute(
                "INSERT INTO reports(repo_id, sha, branch, project_id, us_id, us_ref, titulo, externo) VALUES (NULL, ?1, '', ?2, ?3, ?4, ?5, 1)",
                params![format!("ext{}", r.us_id), p.id, r.us_id, us_ref, titulo],
            )?;
            Ok(c.last_insert_rowid())
        });
        match ins {
            Ok(id) => ids.push(id),
            Err(e) => avisos.push(e.to_string()),
        }
    }
    (ids, avisos)
}

fn kiro_cfg(state: &AppState) -> AppResult<KiroCfg> {
    Ok(KiroCfg { bin: crate::cmd_kiro::kiro_bin(state)?, api_key: secrets::get(crate::cmd_kiro::API_KEY_REF)? })
}

/// Passo 1 de "Revisar e fechar": gera o resumo (Kiro) em segundo plano. NADA é fechado aqui.
/// O resultado da tarefa traz `resultados` (um por card), `ids` e `avisos`.
#[tauri::command]
pub async fn cards_prepare(app: AppHandle, state: tauri::State<'_, AppState>, refs: Vec<CardRef>, regerar: bool) -> AppResult<String> {
    if refs.is_empty() {
        return Err(AppError::msg("Nenhuma US selecionada."));
    }
    if refs.len() > MAX_CLOSE {
        return Err(AppError::msg(format!("Selecione no máximo {MAX_CLOSE} US por vez.")));
    }
    kiro_cfg(&state)?; // falha cedo se o Kiro não está configurado
    let mut key: Vec<String> = refs.iter().map(|r| format!("{}/{}/{}", r.report_id.unwrap_or(0), r.project_id.unwrap_or(0), r.us_id)).collect();
    key.sort();
    let titulo = format!("{} {} US", if regerar { "Regerando resumo de" } else { "Resumo de" }, refs.len());
    let id = state.tasks.start(&format!("cards-resumo:{}", key.join(",")), &titulo).map_err(AppError::msg)?;
    let (app2, id2) = (app.clone(), id.clone());
    tauri::async_runtime::spawn(async move { run_prepare(app2, id2, refs, regerar).await });
    Ok(id)
}

async fn run_prepare(app: AppHandle, tid: String, refs: Vec<CardRef>, regerar: bool) {
    let state = app.state::<AppState>();
    let tasks = &state.tasks;
    let progress = |m: String| tasks.step(&tid, &m);
    let (hub, kiro) = match (build_hub(&state), kiro_cfg(&state)) {
        (Ok(h), Ok(k)) => (h, k),
        (Err(e), _) | (_, Err(e)) => return tasks.finish(&tid, false, &e.to_string(), None),
    };
    progress("Conferindo as US no Taiga".into());
    let (ids, avisos) = adopt(&hub, &state, &refs).await;
    if ids.is_empty() {
        let msg = if avisos.is_empty() { "Nenhuma US válida.".to_owned() } else { avisos.join(" ") };
        return tasks.finish(&tid, false, &msg, Some(serde_json::json!({"resultados": [], "ids": [], "avisos": avisos})));
    }
    let projects = proj_map(&state).unwrap_or_default();
    let recs = with_db(&state, |c| load_reports(c, Some(&ids))).unwrap_or_default();
    let mut sorted: Vec<&Rec> = recs.iter().collect();
    sorted.sort_by_key(|r| ids.iter().position(|i| *i == r.row.id));
    let srcs: Vec<Src> = sorted.iter().map(|r| src_of(r, &projects)).collect();
    let today = chrono::Local::now().date_naive();
    let cards = cards::cards_for(&hub, &srcs, today, true, true).await;
    let res = summary::prepare(&state.db, &kiro, &cards, regerar, &progress).await;
    let n_ok = res.iter().filter(|r| r.ok).count();
    let msg = if n_ok > 0 { format!("{n_ok} de {} US resumida(s). Revise e feche.", ids.len()) } else { "Nenhuma US foi resumida; veja os detalhes.".to_owned() };
    tasks.finish(&tid, n_ok > 0, &msg, Some(serde_json::json!({"resultados": res, "ids": ids, "avisos": avisos, "regerar": regerar})));
}

#[derive(Serialize)]
pub struct ReviewItem {
    id: i64,
    tipo: String,
    texto: String,
}

#[derive(Serialize)]
pub struct Review {
    id: i64,
    itens: Vec<ReviewItem>,
}

/// Pontos gravados (feito/pendência) de cada card, para a tela de revisão.
#[tauri::command]
pub fn cards_review(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> AppResult<Vec<Review>> {
    with_db(&state, |c| {
        let mut out: Vec<Review> = Vec::new();
        for id in ids {
            let mut st = c.prepare("SELECT id, tipo, texto FROM card_items WHERE report_id = ?1 ORDER BY tipo, ordem, id")?;
            let itens = st.query_map([id], |r| Ok(ReviewItem { id: r.get(0)?, tipo: r.get(1)?, texto: r.get(2)? }))?.collect::<Result<_, _>>()?;
            out.push(Review { id, itens });
        }
        Ok(out)
    })
}

/// Move cada US para o status fechado do SEU projeto. Só fecha quem já tem resumo gerado.
#[tauri::command]
pub async fn cards_close(app: AppHandle, state: tauri::State<'_, AppState>, ids: Vec<i64>) -> AppResult<String> {
    let ids: Vec<i64> = ids.into_iter().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    if ids.is_empty() {
        return Err(AppError::msg("Nenhuma US selecionada."));
    }
    if ids.len() > MAX_CLOSE {
        return Err(AppError::msg(format!("Selecione no máximo {MAX_CLOSE} US por vez.")));
    }
    let id = state
        .tasks
        .start(&format!("cards-fechar:{}", ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")), &format!("Fechando {} US no Taiga", ids.len()))
        .map_err(AppError::msg)?;
    let (app2, id2) = (app.clone(), id.clone());
    tauri::async_runtime::spawn(async move {
        let state = app2.state::<AppState>();
        let hub = match build_hub(&state) {
            Ok(h) => h,
            Err(e) => return state.tasks.finish(&id2, false, &e.to_string(), None),
        };
        let projects = proj_map(&state).unwrap_or_default();
        let recs = with_db(&state, |c| load_reports(c, Some(&ids))).unwrap_or_default();
        let mut results: Vec<cards::CloseResult> = Vec::new();
        let mut reqs: Vec<CloseReq> = Vec::new();
        for rec in &recs {
            let p = rec.project_id.and_then(|i| projects.get(&i));
            let titulo = rec.row.titulo.clone().unwrap_or_else(|| format!("US #{}", rec.row.us_ref.unwrap_or(rec.us_id)));
            let projeto = p.map(|p| p.nome.clone()).unwrap_or_default();
            // Guarda: só fecha quem já tem resumo (falha do Kiro nunca resulta em card fechado).
            if !rec.row.tem_itens || rec.row.resumo_hash.is_none() {
                results.push(cards::CloseResult { id: Some(rec.row.id), ref_: rec.row.us_ref, titulo, projeto, ok: false, detail: "Sem resumo gerado: use “Revisar e fechar” para gerar o resumo antes.".into() });
                continue;
            }
            reqs.push(CloseReq { report_id: rec.row.id, us_id: rec.us_id, us_ref: rec.row.us_ref, titulo, account_id: p.map(|p| p.account_id), slug: p.map(|p| p.slug.clone()), projeto });
        }
        if recs.len() < ids.len() {
            results.push(cards::CloseResult { id: None, ref_: None, titulo: String::new(), projeto: String::new(), ok: false, detail: format!("{} US não encontrada(s).", ids.len() - recs.len()) });
        }
        state.tasks.step(&id2, &format!("Fechando {} US no Taiga", reqs.len()));
        results.extend(cards::close_cards(&hub, reqs).await);
        let n_ok = results.iter().filter(|r| r.ok).count();
        let bad = results.len() - n_ok;
        let msg = format!("{n_ok} US fechada(s) no Taiga{}", if bad > 0 { format!("; {bad} com problema.") } else { ".".into() });
        state.tasks.finish(&id2, bad == 0, &msg, Some(serde_json::json!({"resultados": results})));
    });
    Ok(id)
}

// --------------------------------------------------------------------------- //
// Prestação de contas (itens do resumo)
// --------------------------------------------------------------------------- //

#[derive(Serialize)]
pub struct ItemOut {
    id: i64,
    tipo: String,
    texto: String,
    prestado_em: Option<String>,
}

#[derive(Serialize)]
pub struct GrupoOut {
    us_id: i64,
    #[serde(rename = "ref")]
    ref_: Option<i64>,
    titulo: String,
    project_id: Option<i64>,
    projeto: String,
    data: Option<String>,
    report_id: Option<i64>,
    pode_regerar: bool,
    itens: Vec<ItemOut>,
}

#[derive(Serialize)]
pub struct PrestacaoOut {
    grupos: Vec<GrupoOut>,
    total: usize,
    projetos: Vec<ProjetoOpt>,
    pendentes: i64,
}

fn like(q: &str) -> String {
    format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
}

fn pendentes(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("SELECT count(*) FROM card_items WHERE prestado_em IS NULL", [], |r| r.get(0))?)
}

#[tauri::command]
pub fn prestacao_pendentes(state: tauri::State<'_, AppState>) -> AppResult<i64> {
    with_db(&state, pendentes)
}

#[tauri::command]
pub fn prestacao_list(state: tauri::State<'_, AppState>, q: String, project_id: Option<i64>, estado: String) -> AppResult<PrestacaoOut> {
    let projects = proj_map(&state)?;
    with_db(&state, |c| {
        let mut sql = String::from("SELECT id, report_id, project_id, us_id, us_ref, titulo, texto, tipo, data_ref, prestado_em FROM card_items WHERE 1=1");
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        if !q.trim().is_empty() {
            sql.push_str(" AND (texto LIKE ? ESCAPE '\\' OR titulo LIKE ? ESCAPE '\\')");
            args.push(like(q.trim()).into());
            args.push(like(q.trim()).into());
        }
        if let Some(p) = project_id {
            sql.push_str(" AND project_id = ?");
            args.push(p.into());
        }
        match estado.as_str() {
            "prestado" => sql.push_str(" AND prestado_em IS NOT NULL"),
            "todos" => {}
            _ => sql.push_str(" AND prestado_em IS NULL"),
        }
        sql.push_str(" ORDER BY data_ref DESC, us_id, tipo, ordem, id LIMIT 300");
        let mut st = c.prepare(&sql)?;
        type Row = (i64, Option<i64>, Option<i64>, i64, Option<i64>, String, String, String, Option<String>, Option<String>);
        let rows: Vec<Row> = st
            .query_map(rusqlite::params_from_iter(args.iter()), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?))
            })?
            .collect::<Result<_, _>>()?;
        let total = rows.len();
        let vivos: HashSet<i64> = {
            let mut s = c.prepare("SELECT id FROM reports")?;
            s.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?
        };
        let mut grupos: Vec<GrupoOut> = Vec::new();
        for (id, report_id, pid, us_id, us_ref, titulo, texto, tipo, data, prestado) in rows {
            let idx = match grupos.iter().position(|g| g.us_id == us_id) {
                Some(i) => i,
                None => {
                    grupos.push(GrupoOut {
                        us_id,
                        ref_: us_ref,
                        titulo,
                        project_id: pid,
                        projeto: pid.and_then(|p| projects.get(&p)).map(|p| p.nome.clone()).unwrap_or_default(),
                        data,
                        report_id,
                        pode_regerar: false,
                        itens: Vec::new(),
                    });
                    grupos.len() - 1
                }
            };
            grupos[idx].itens.push(ItemOut { id, tipo, texto, prestado_em: prestado });
        }
        for g in &mut grupos {
            g.pode_regerar = g.report_id.is_some_and(|r| vivos.contains(&r)) && g.itens.iter().any(|i| i.prestado_em.is_none());
        }
        let ids_proj: HashSet<i64> = {
            let mut s = c.prepare("SELECT DISTINCT project_id FROM card_items WHERE project_id IS NOT NULL")?;
            s.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?
        };
        let mut projetos: Vec<ProjetoOpt> = ids_proj
            .iter()
            .filter_map(|id| projects.get(id))
            .map(|p| ProjetoOpt { id: p.id, nome: p.nome.clone(), conta: p.account_nome.clone() })
            .collect();
        projetos.sort_by_key(|p| p.nome.to_lowercase());
        Ok(PrestacaoOut { grupos, total, projetos, pendentes: pendentes(c)? })
    })
}

struct DbItem {
    id: i64,
    us_id: i64,
    us_ref: Option<i64>,
    project_id: Option<i64>,
    titulo: String,
    texto: String,
    tipo: String,
    prestado: bool,
}

fn load_items(conn: &Connection, ids: &[i64]) -> AppResult<Vec<DbItem>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT id, us_id, us_ref, project_id, titulo, texto, tipo, prestado_em IS NOT NULL FROM card_items WHERE id IN ({}) ORDER BY data_ref, us_id, tipo, ordem, id",
        vec!["?"; ids.len()].join(",")
    );
    let mut st = conn.prepare(&sql)?;
    let v = st
        .query_map(rusqlite::params_from_iter(ids.iter()), |r| {
            Ok(DbItem { id: r.get(0)?, us_id: r.get(1)?, us_ref: r.get(2)?, project_id: r.get(3)?, titulo: r.get(4)?, texto: r.get(5)?, tipo: r.get(6)?, prestado: r.get(7)? })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

fn dedup(ids: Vec<i64>) -> Vec<i64> {
    let mut seen = HashSet::new();
    ids.into_iter().filter(|i| seen.insert(*i)).collect()
}

fn mark(conn: &Connection, ids: &[i64], prestado: bool) -> AppResult<usize> {
    let mut n = 0;
    for id in ids {
        n += conn.execute(
            if prestado { "UPDATE card_items SET prestado_em = datetime('now') WHERE id = ?1 AND prestado_em IS NULL" } else { "UPDATE card_items SET prestado_em = NULL WHERE id = ?1 AND prestado_em IS NOT NULL" },
            [id],
        )?;
    }
    Ok(n)
}

/// Presta conta dos pontos MARCADOS. Com `exportar_url`, antes insere os pontos na seção "Feito" da US
/// de destino (digitada na hora); se a exportação falhar nada é marcado como prestado.
#[tauri::command]
pub async fn prestar(state: tauri::State<'_, AppState>, item_ids: Vec<i64>, exportar_url: Option<String>) -> AppResult<String> {
    let ids = dedup(item_ids);
    let itens = with_db(&state, |c| load_items(c, &ids))?;
    if itens.is_empty() {
        return Err(AppError::msg("Nenhum ponto marcado."));
    }
    let novos: Vec<&DbItem> = itens.iter().filter(|i| !i.prestado).collect(); // já prestados não entram de novo
    if novos.is_empty() {
        return Err(AppError::msg("Os pontos marcados já estavam prestados (use “Desfazer prestação”)."));
    }
    let mut extra = String::new();
    if let Some(url) = exportar_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        let hub = build_hub(&state)?;
        let projects = proj_map(&state)?;
        let export_items: Vec<Item> = novos
            .iter()
            .map(|i| Item {
                us_id: i.us_id,
                us_ref: i.us_ref,
                slug: i.project_id.and_then(|p| projects.get(&p)).map(|p| p.slug.clone()),
                titulo: i.titulo.clone(),
                texto: i.texto.clone(),
                tipo: i.tipo.clone(),
            })
            .collect();
        let (titulo, r) = export::export_to_us(&hub, url, &export_items)
            .await
            .map_err(|e| AppError::msg(format!("Não exportou (nada foi marcado como prestado): {e}")))?;
        extra = format!(" e exportado(s) para “Feito” na US #{r} “{titulo}”");
    }
    let marcar: Vec<i64> = novos.iter().map(|i| i.id).collect();
    let n = with_db(&state, |c| mark(c, &marcar, true))?;
    Ok(format!("{} ponto(s) selecionado(s); {n} marcado(s) como prestado(s){extra}.", itens.len()))
}

/// Volta a pendente os pontos prestados marcados. Não mexe no Taiga: o que já foi exportado para a
/// descrição de uma US continua lá (edite a US se quiser remover).
#[tauri::command]
pub fn desfazer(state: tauri::State<'_, AppState>, item_ids: Vec<i64>) -> AppResult<String> {
    let ids = dedup(item_ids);
    let alvo: Vec<i64> = with_db(&state, |c| load_items(c, &ids))?.into_iter().filter(|i| i.prestado).map(|i| i.id).collect();
    if alvo.is_empty() {
        return Err(AppError::msg("Nenhum ponto prestado marcado."));
    }
    let n = with_db(&state, |c| mark(c, &alvo, false))?;
    Ok(format!("{n} ponto(s) voltaram para pendente (o Taiga não foi alterado)."))
}

/// Marca/desmarca um único ponto (botão do próprio item).
#[tauri::command]
pub fn toggle_item(state: tauri::State<'_, AppState>, id: i64, prestado: bool) -> AppResult<()> {
    with_db(&state, |c| {
        if load_items(c, &[id])?.is_empty() {
            return Err(AppError::msg("Item não encontrado."));
        }
        mark(c, &[id], prestado)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = crate::db::open_in_memory().unwrap();
        c.execute("INSERT INTO taiga_accounts(id,nome,host,usuario,secret_ref) VALUES (1,'A','h','u','r')", []).unwrap();
        c.execute("INSERT INTO taiga_projects(id,account_id,slug,nome) VALUES (1,1,'p','Proj')", []).unwrap();
        c.execute("INSERT INTO reports(id,sha,us_id,us_ref,titulo,project_id,externo) VALUES (1,'ext10',10,3,'Primeira',1,1)", []).unwrap();
        c.execute("INSERT INTO reports(id,sha,us_id,us_ref,titulo,project_id,resumo_hash) VALUES (2,'abc1234',20,4,'Segunda',1,'h')", []).unwrap();
        for (i, (rep, us, tipo, texto)) in [(1, 10, "feito", "A"), (1, 10, "pendencia", "P"), (2, 20, "feito", "B")].iter().enumerate() {
            c.execute(
                "INSERT INTO card_items(report_id,project_id,us_id,us_ref,titulo,ordem,texto,tipo,data_ref) VALUES (?1,1,?2,?2,'T',?3,?4,?5,'2026-11-01')",
                params![rep, us, i as i64, texto, tipo],
            )
            .unwrap();
        }
        c
    }

    #[test]
    fn carrega_reports_com_flags_de_resumo() {
        let c = db();
        let r = load_reports(&c, None).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[0].row.externo && r[0].row.tem_itens && r[0].row.resumo_hash.is_none());
        assert!(!r[1].row.externo && r[1].row.resumo_hash.as_deref() == Some("h"));
        assert_eq!(load_reports(&c, Some(&[2])).unwrap().len(), 1);
        assert!(load_reports(&c, Some(&[])).unwrap().is_empty());
    }

    #[test]
    fn marcar_e_desmarcar_so_afetam_quem_precisa() {
        let c = db();
        assert_eq!(pendentes(&c).unwrap(), 3);
        assert_eq!(mark(&c, &[1, 2], true).unwrap(), 2);
        assert_eq!(mark(&c, &[1], true).unwrap(), 0, "já prestado: não conta de novo");
        assert_eq!(pendentes(&c).unwrap(), 1);
        assert_eq!(mark(&c, &[1, 3], false).unwrap(), 1, "só o prestado volta");
        assert_eq!(pendentes(&c).unwrap(), 2);
    }

    #[test]
    fn busca_escapa_curingas_do_like() {
        assert_eq!(like("100%_a\\b"), "%100\\%\\_a\\\\b%");
        let c = db();
        c.execute("UPDATE card_items SET texto = 'tem 100% de cobertura' WHERE id = 1", []).unwrap();
        let n: i64 = c.query_row("SELECT count(*) FROM card_items WHERE texto LIKE ?1 ESCAPE '\\'", [like("100%")], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        let n: i64 = c.query_row("SELECT count(*) FROM card_items WHERE texto LIKE ?1 ESCAPE '\\'", [like("%")], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "o % digitado não vira curinga");
    }

    #[test]
    fn dedup_mantem_a_ordem() {
        assert_eq!(dedup(vec![3, 1, 3, 2, 1]), [3, 1, 2]);
    }
}
