//! Exportação da prestação de contas para a descrição de uma US de destino.
//!
//! Porte fiel de `_inserir_em_secao` / `inserir_em_feito` / `export_items` do ci_cd
//! (`app/taiga_cards.py`). Os itens "feito" entram sob o título "Feito" da descrição; os
//! "pendencia" formam uma lista PRÓPRIA "Pendências". Agrupa por US de origem, sem duplicar,
//! e deixa o resto da descrição (ex.: "Para essa semana") intacto.
//!
//! Formato: `- [#75](url do projeto de origem) Título (N pts)` e, abaixo (4 espaços, o Markdown do
//! Taiga só aninha com 4), `    - ponto`. O link leva à US no projeto de ORIGEM: um `#75` puro seria
//! resolvido pelo Taiga para a US 75 do projeto onde o texto está.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{json, Value};

use crate::cards::{evict, Hub};
use crate::taiga::{host_of, parse_us_url};

const FILHO: &str = "    ";

static FEITO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s{0,3}(?:#{1,6}\s*)?(?:\*\*|__)?\s*feito\s*:?\s*(?:\*\*|__)?\s*:?\s*$").unwrap());
static PENDENCIAS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s{0,3}(?:#{1,6}\s*)?(?:\*\*|__)?\s*pend[eê]ncias?\s*:?\s*(?:\*\*|__)?\s*:?\s*$").unwrap()
});
static LISTA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(?:[-*+]|\d+\.)\s").unwrap());
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)(?:[-*+]|\d+\.)\s+(.*)$").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\(([^)\s]*)\)").unwrap());
static FORMAT_NOISE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[*_`]").unwrap());

/// Um ponto a exportar (já com os dados da US de origem).
#[derive(Debug, Clone)]
pub struct Item {
    pub us_id: i64,
    pub us_ref: Option<i64>,
    /// Slug do projeto de ORIGEM (para montar o link).
    pub slug: Option<String>,
    pub titulo: String,
    pub texto: String,
    /// "feito" | "pendencia"
    pub tipo: String,
}

/// Dados da US de origem vindos do Taiga (opcionais).
#[derive(Debug, Clone, Default)]
pub struct UsInfo {
    pub subject: Option<String>,
    pub total_points: Option<f64>,
    /// `https://taiga.exemplo.com` (esquema + host do Taiga da US de DESTINO).
    pub base: String,
}

#[derive(Debug)]
struct Grupo {
    us_id: i64,
    r#ref: Option<i64>,
    titulo: String,
    pts: Option<f64>,
    url: String,
    textos: Vec<String>,
}

fn fmt_pts(pts: Option<f64>) -> String {
    match pts {
        None => String::new(),
        Some(n) if n.fract() == 0.0 => format!(" ({} pts)", n as i64),
        Some(n) => format!(" ({} pts)", n.to_string().replace('.', ",")),
    }
}

fn grupos(items: &[&Item], us_info: &HashMap<i64, UsInfo>) -> Vec<Grupo> {
    let mut out: Vec<Grupo> = Vec::new();
    for it in items {
        let texto = it.texto.split_whitespace().collect::<Vec<_>>().join(" ");
        if let Some(g) = out.iter_mut().find(|g| g.us_id == it.us_id) {
            g.textos.push(texto);
            continue;
        }
        let info = us_info.get(&it.us_id).cloned().unwrap_or_default();
        let base = info.base.trim_end_matches('/');
        let url = match (&it.slug, it.us_ref) {
            (Some(slug), Some(r)) if !base.is_empty() && !slug.is_empty() => format!("{base}/project/{slug}/us/{r}"),
            _ => String::new(),
        };
        let titulo = info.subject.unwrap_or_else(|| it.titulo.clone()).trim().to_owned();
        out.push(Grupo { us_id: it.us_id, r#ref: it.us_ref, titulo, pts: info.total_points, url, textos: vec![texto] });
    }
    out
}

fn ref_txt(g: &Grupo) -> String {
    match g.r#ref {
        None => String::new(),
        Some(r) if !g.url.is_empty() => format!("[#{r}]({})", g.url),
        Some(r) => format!("#{r}"),
    }
}

fn cabecalho(g: &Grupo, com_pts: bool) -> String {
    let r = if g.r#ref.is_some() { format!("{} ", ref_txt(g)) } else { String::new() };
    format!("{r}{}{}", g.titulo, if com_pts { fmt_pts(g.pts) } else { String::new() }).trim().to_owned()
}

/// Índice (exclusivo) onde termina a seção que começa em `ini`: até a primeira linha não vazia que
/// não seja item de lista nem continuação indentada.
fn bloco_fim(ini: usize, linhas: &[String]) -> usize {
    let mut fim = ini;
    for (j, ln) in linhas.iter().enumerate().skip(ini) {
        if ln.trim().is_empty() {
            continue;
        }
        if LISTA.is_match(ln) || ln.starts_with(' ') || ln.starts_with('\t') {
            fim = j + 1;
        } else {
            break;
        }
    }
    fim
}

/// O item existente é a mesma US? Com link vale a URL (o mesmo #ref existe em vários projetos);
/// sem link (exportação antiga) vale o título.
fn casa_us(texto: &str, g: &Grupo) -> bool {
    if let Some(m) = LINK.captures(texto) {
        if !g.url.is_empty() {
            return m[2].trim_end_matches('/') == g.url;
        }
    }
    let t = LINK.replace_all(texto, "$1");
    let t = FORMAT_NOISE.replace_all(&t, "").trim().to_lowercase();
    !g.titulo.is_empty() && t.contains(&g.titulo.to_lowercase())
}

/// Troca a primeira ocorrência de `#<ref>` (não seguida de dígito) por `novo`.
fn troca_ref_pura(linha: &str, r: i64, novo: &str) -> String {
    let alvo = format!("#{r}");
    let mut from = 0;
    while let Some(pos) = linha[from..].find(&alvo) {
        let ini = from + pos;
        let fim = ini + alvo.len();
        if !linha[fim..].starts_with(|c: char| c.is_ascii_digit()) {
            return format!("{}{}{}", &linha[..ini], novo, &linha[fim..]);
        }
        from = fim;
    }
    linha.to_owned()
}

fn inserir_em_secao(
    descricao: &str,
    items: &[&Item],
    us_info: &HashMap<i64, UsInfo>,
    rx: &Regex,
    nome: &str,
    com_pts: bool,
) -> String {
    let gs = grupos(items, us_info);
    let normal = descricao.replace("\r\n", "\n");
    let linhas: Vec<String> = normal.split('\n').map(str::to_owned).collect();
    let Some(idx) = linhas.iter().position(|l| rx.is_match(l)) else {
        let mut corpo: Vec<String> = Vec::new();
        for g in &gs {
            corpo.push(format!("- {}", cabecalho(g, com_pts)));
            corpo.extend(g.textos.iter().map(|t| format!("{FILHO}- {t}")));
        }
        let base = descricao.trim_end();
        let pre = if base.is_empty() { String::new() } else { format!("{base}\n\n") };
        return format!("{pre}### {nome}\n\n{}\n", corpo.join("\n"));
    };

    let mut ini = idx + 1;
    while ini < linhas.len() && linhas[ini].trim().is_empty() {
        ini += 1;
    }
    let fim = bloco_fim(ini, &linhas).max(ini);
    let mut bloco: Vec<String> = linhas.get(ini..fim).map(<[String]>::to_vec).unwrap_or_default();

    for g in &gs {
        let topo = bloco.iter().position(|ln| {
            BULLET.captures(ln).is_some_and(|m| m[1].is_empty() && casa_us(&m[2], g))
        });
        let Some(topo) = topo else {
            bloco.push(format!("- {}", cabecalho(g, com_pts)));
            bloco.extend(g.textos.iter().map(|t| format!("{FILHO}- {t}")));
            continue;
        };
        if let (false, Some(r)) = (g.url.is_empty(), g.r#ref) {
            if !LINK.is_match(&bloco[topo]) {
                // exportação antiga: só "#ref" puro → vira link para o projeto de origem
                bloco[topo] = troca_ref_pura(&bloco[topo], r, &ref_txt(g));
            }
        }
        // fim dos filhos do cabeçalho existente
        let mut k = topo + 1;
        while k < bloco.len() && (bloco[k].trim().is_empty() || bloco[k].starts_with(' ') || bloco[k].starts_with('\t')) {
            k += 1;
        }
        while k > topo + 1 && bloco[k - 1].trim().is_empty() {
            k -= 1;
        }
        let existentes: HashSet<String> = bloco[topo + 1..k]
            .iter()
            .map(|x| {
                let t = BULLET.captures(x).map_or(x.as_str(), |m| m.get(2).map_or("", |g| g.as_str()));
                t.split_whitespace().collect::<Vec<_>>().join(" ")
            })
            .collect();
        let novos: Vec<String> = g.textos.iter().filter(|t| !existentes.contains(*t)).map(|t| format!("{FILHO}- {t}")).collect();
        let resto = bloco.split_off(k);
        bloco.extend(novos);
        bloco.extend(resto);
    }

    let mut resto: Vec<String> = linhas[fim.min(linhas.len())..].to_vec();
    while resto.first().is_some_and(|l| l.trim().is_empty()) {
        resto.remove(0);
    }
    let mut out: Vec<String> = linhas[..=idx].to_vec();
    out.push(String::new());
    out.extend(bloco);
    if !resto.is_empty() {
        out.push(String::new());
    }
    out.extend(resto);
    out.join("\n")
}

/// Itens "feito" → lista "Feito"; "pendencia" (dependências externas) → lista própria "Pendências".
pub fn inserir_em_feito(descricao: &str, items: &[Item], us_info: &HashMap<i64, UsInfo>) -> String {
    let feitos: Vec<&Item> = items.iter().filter(|i| i.tipo != "pendencia").collect();
    let pends: Vec<&Item> = items.iter().filter(|i| i.tipo == "pendencia").collect();
    let mut out = descricao.to_owned();
    if !feitos.is_empty() || pends.is_empty() {
        out = inserir_em_secao(&out, &feitos, us_info, &FEITO, "Feito", true);
    }
    if !pends.is_empty() {
        out = inserir_em_secao(&out, &pends, us_info, &PENDENCIAS, "Pendências", false);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://t.x.com";

    fn item(us_id: i64, r: i64, titulo: &str, texto: &str, tipo: &str) -> Item {
        Item { us_id, us_ref: Some(r), slug: Some("proj-a".into()), titulo: titulo.into(), texto: texto.into(), tipo: tipo.into() }
    }

    fn info(pairs: &[(i64, &str, f64)]) -> HashMap<i64, UsInfo> {
        pairs
            .iter()
            .map(|(id, s, p)| (*id, UsInfo { subject: Some((*s).into()), total_points: Some(*p), base: BASE.into() }))
            .collect()
    }

    fn aplicar(desc: &str, items: &[Item], i: &HashMap<i64, UsInfo>) -> String {
        inserir_em_feito(desc, items, i)
    }

    #[test]
    fn sem_a_secao_feito_cria_no_fim_com_link_e_pontos() {
        let i = info(&[(1, "Cadastro de eventos", 5.0)]);
        let out = aplicar("Pauta da semana.", &[item(1, 75, "x", "Criou o formulário.", "feito")], &i);
        assert_eq!(
            out,
            "Pauta da semana.\n\n### Feito\n\n- [#75](https://t.x.com/project/proj-a/us/75) Cadastro de eventos (5 pts)\n    - Criou o formulário.\n"
        );
    }

    #[test]
    fn descricao_vazia_comeca_direto_na_secao() {
        let out = aplicar("", &[item(1, 3, "T", "ponto", "feito")], &HashMap::new());
        assert_eq!(out, "### Feito\n\n- #3 T\n    - ponto\n");
    }

    #[test]
    fn reconhece_as_variacoes_do_titulo_feito() {
        let i = info(&[(1, "US", 2.0)]);
        for cab in ["Feito", "## Feito", "**Feito**", "Feito:", "### FEITO :", "__Feito__"] {
            let desc = format!("{cab}\n\n- [#9](https://t.x.com/project/proj-a/us/9) Outra (1 pts)\n    - já tinha\n");
            let out = aplicar(&desc, &[item(1, 75, "x", "novo ponto", "feito")], &i);
            assert!(out.starts_with(&format!("{cab}\n")), "{cab}: {out}");
            assert!(out.contains("- [#75](https://t.x.com/project/proj-a/us/75) US (2 pts)\n    - novo ponto"), "{cab}: {out}");
            assert!(out.contains("já tinha"), "{cab}");
            assert_eq!(out.matches("Feito").count() + out.matches("FEITO").count(), 1, "{cab}: não cria segunda seção");
        }
    }

    #[test]
    fn mesma_us_nao_duplica_o_cabecalho_e_so_acrescenta_o_que_falta() {
        let i = info(&[(1, "Cadastro", 3.0)]);
        let desc = "### Feito\n\n- [#75](https://t.x.com/project/proj-a/us/75) Cadastro (3 pts)\n    - ponto A\n";
        let out = aplicar(desc, &[item(1, 75, "x", "ponto A", "feito"), item(1, 75, "x", "ponto B", "feito")], &i);
        assert_eq!(out.matches("Cadastro").count(), 1, "{out}");
        assert_eq!(out.matches("ponto A").count(), 1, "não repete o que já existe");
        assert!(out.contains("    - ponto B"));
        // idempotente: aplicar de novo não muda nada
        assert_eq!(aplicar(&out, &[item(1, 75, "x", "ponto A", "feito"), item(1, 75, "x", "ponto B", "feito")], &i), out);
    }

    #[test]
    fn mesmo_numero_em_projetos_diferentes_sao_uss_diferentes() {
        // #75 do projeto A já listado; #75 do projeto B tem outra URL → novo cabeçalho
        let i = info(&[(2, "Outra coisa", 1.0)]);
        let desc = "Feito\n- [#75](https://t.x.com/project/proj-a/us/75) Cadastro (3 pts)\n    - A\n";
        let mut it = item(2, 75, "x", "B", "feito");
        it.slug = Some("proj-b".into());
        let out = aplicar(desc, &[it], &i);
        assert!(out.contains("[#75](https://t.x.com/project/proj-b/us/75) Outra coisa (1 pts)"), "{out}");
        assert!(out.contains("proj-a/us/75"), "o antigo continua");
    }

    #[test]
    fn exportacao_antiga_com_ref_puro_vira_link() {
        let i = info(&[(1, "Cadastro", 3.0)]);
        let desc = "Feito\n\n- #75 Cadastro (3 pts)\n    - antigo\n";
        let out = aplicar(desc, &[item(1, 75, "x", "novo", "feito")], &i);
        assert!(out.contains("- [#75](https://t.x.com/project/proj-a/us/75) Cadastro (3 pts)"), "{out}");
        assert!(out.contains("    - antigo") && out.contains("    - novo"));
        // #750 não é #75
        let desc2 = "Feito\n- #750 Cadastro (3 pts)\n";
        let out2 = aplicar(desc2, &[item(1, 75, "x", "n", "feito")], &i);
        assert!(out2.contains("- #750 Cadastro"), "não mexe no #750: {out2}");
    }

    #[test]
    fn pendencias_em_lista_propria_sem_pontos() {
        let i = info(&[(1, "Cadastro", 5.0)]);
        let items = [item(1, 75, "x", "entregou", "feito"), item(1, 75, "x", "aguarda aprovação da coordenação", "pendencia")];
        let out = aplicar("", &items, &i);
        assert!(out.contains("### Feito\n\n- [#75](https://t.x.com/project/proj-a/us/75) Cadastro (5 pts)\n    - entregou\n"), "{out}");
        assert!(out.contains("### Pendências\n\n- [#75](https://t.x.com/project/proj-a/us/75) Cadastro\n    - aguarda aprovação da coordenação\n"), "{out}");
        assert!(!out.contains("Cadastro (5 pts)\n    - aguarda"), "pendência nunca entra na lista Feito");
    }

    #[test]
    fn so_pendencias_nao_cria_secao_feito_vazia() {
        let out = aplicar("", &[item(1, 5, "T", "depende de terceiros", "pendencia")], &HashMap::new());
        assert!(!out.contains("Feito"), "{out}");
        assert!(out.starts_with("### Pendências"), "{out}");
    }

    #[test]
    fn o_resto_da_descricao_fica_intacto() {
        let i = info(&[(1, "Cadastro", 2.0)]);
        let desc = "Reunião de alinhamento\n\n## Feito\n\n- [#1](https://t.x.com/project/proj-a/us/1) Velha (1 pts)\n    - a\n\n## Para essa semana\n\n- fazer X\n- fazer Y\n\n## Pendências\n\n- algo antigo\n";
        let out = aplicar(desc, &[item(1, 75, "x", "novo", "feito")], &i);
        assert!(out.starts_with("Reunião de alinhamento\n\n## Feito\n\n"));
        assert!(out.contains("\n\n## Para essa semana\n\n- fazer X\n- fazer Y\n\n## Pendências\n\n- algo antigo"), "{out}");
        let (feito, resto) = out.split_once("## Para essa semana").unwrap();
        assert!(feito.contains("Cadastro (2 pts)\n    - novo"), "{feito}");
        assert!(resto.contains("fazer Y"));
    }

    #[test]
    fn crlf_e_normalizado() {
        let i = info(&[(1, "Cadastro", 2.0)]);
        let out = aplicar("Feito\r\n\r\n- #9 Velha\r\n    - a\r\n", &[item(1, 75, "x", "novo", "feito")], &i);
        assert!(!out.contains('\r'), "{out:?}");
        assert!(out.contains("Cadastro (2 pts)\n    - novo"));
    }

    #[test]
    fn pontos_decimais_usam_virgula_e_sem_pontos_omite() {
        assert_eq!(fmt_pts(Some(5.0)), " (5 pts)");
        assert_eq!(fmt_pts(Some(2.5)), " (2,5 pts)");
        assert_eq!(fmt_pts(None), "");
        let mut i = info(&[(1, "Cadastro", 0.0)]);
        i.get_mut(&1).unwrap().total_points = None;
        let out = aplicar("", &[item(1, 75, "x", "p", "feito")], &i);
        assert!(out.contains("Cadastro\n    - p"), "{out}");
    }

    #[test]
    fn sem_info_do_taiga_usa_o_titulo_guardado_e_sem_link() {
        let mut it = item(1, 75, "Título guardado", "p", "feito");
        it.slug = None;
        let out = aplicar("", &[it], &HashMap::new());
        assert_eq!(out, "### Feito\n\n- #75 Título guardado\n    - p\n");
    }

    #[test]
    fn varias_uss_agrupadas_na_ordem_de_aparicao_e_espacos_normalizados() {
        let i = info(&[(1, "Primeira", 1.0), (2, "Segunda", 2.0)]);
        let items = [item(2, 20, "x", "  texto   com\n  espaços ", "feito"), item(1, 10, "x", "a", "feito"), item(2, 20, "x", "b", "feito")];
        let out = aplicar("", &items, &i);
        let pos_seg = out.find("Segunda").unwrap();
        let pos_pri = out.find("Primeira").unwrap();
        assert!(pos_seg < pos_pri, "ordem de aparição: {out}");
        assert!(out.contains("    - texto com espaços\n    - b\n"), "{out}");
    }
}

/// Insere os itens na seção "Feito"/"Pendências" da descrição da US apontada por `url`. Não altera o
/// status dos itens (quem chama decide). A conta é a que usa o mesmo host da URL. Devolve (título, ref).
pub async fn export_to_us(hub: &Hub, url: &str, items: &[Item]) -> Result<(String, i64), String> {
    let (host, slug, r) = parse_us_url(url).map_err(|e| e.message)?;
    let account = hub
        .accounts()
        .into_iter()
        .find(|(_, h)| host_of(h).as_deref() == Some(host.as_str()))
        .map(|(i, _)| i)
        .ok_or_else(|| format!("Nenhuma conta Taiga sua usa o host {host}."))?;
    let client = hub.client(account).await?;
    let project = client.project(&slug).await.map_err(|e| e.message)?;
    let pid = project.get("id").and_then(Value::as_i64).ok_or("Projeto sem id.")?;
    let mut us = client.us_by_ref(pid, r).await.map_err(|e| e.message)?;
    let us_id = us.get("id").and_then(Value::as_i64).ok_or("História sem id.")?;
    let base = reqwest::Url::parse(url.trim()).map_err(|e| e.to_string())?.origin().ascii_serialization();

    // Pontos (total_points) só existem na US de origem; sem acesso: título guardado, sem pontos, link mantido.
    let mut info: HashMap<i64, UsInfo> = HashMap::new();
    for id in items.iter().map(|i| i.us_id).collect::<HashSet<_>>() {
        let u = match client.get_us(id).await {
            Ok(o) => UsInfo {
                subject: o.get("subject").and_then(Value::as_str).map(str::to_owned),
                total_points: o.get("total_points").and_then(Value::as_f64),
                base: base.clone(),
            },
            Err(_) => UsInfo { base: base.clone(), ..Default::default() },
        };
        info.insert(id, u);
    }
    for tentativa in 0..2 {
        let nova = inserir_em_feito(us.get("description").and_then(Value::as_str).unwrap_or(""), items, &info);
        match client.patch_us(us_id, &json!({"description": nova, "version": us.get("version")})).await {
            Ok(_) => break,
            // alguém editou a US entre a leitura e a gravação: relê e tenta uma vez de novo
            Err(e) if e.status == Some(409) && tentativa == 0 => us = client.get_us(us_id).await.map_err(|e| e.message)?,
            Err(e) => return Err(e.message),
        }
    }
    evict(account, &slug); // a lista seguinte deve enxergar a descrição nova
    Ok((us.get("subject").and_then(Value::as_str).map_or_else(|| format!("#{r}"), str::to_owned), r))
}

#[cfg(test)]
mod tests_export {
    use super::*;
    use httpmock::prelude::*;

    fn it(us_id: i64, texto: &str) -> Item {
        Item { us_id, us_ref: Some(75), slug: Some("origem".into()), titulo: "T".into(), texto: texto.into(), tipo: "feito".into() }
    }

    fn setup(server: &MockServer) -> Hub {
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/auth");
            then.status(200).json_body(json!({"auth_token": "T", "id": 1}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/projects/by_slug").query_param("slug", "reunioes");
            then.status(200).json_body(json!({"id": 5}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/by_ref").query_param("ref", "350").query_param("project", "5");
            then.status(200).json_body(json!({"id": 900, "subject": "Reunião semanal", "version": 3, "description": "Pauta.\n\n## Para essa semana\n\n- X\n"}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/1");
            then.status(200).json_body(json!({"id": 1, "subject": "Cadastro", "total_points": 5.0}));
        });
        Hub::new(HashMap::from([(21, (server.base_url(), "u".to_string(), "s".to_string()))]))
    }

    fn url(server: &MockServer) -> String {
        format!("{}/project/reunioes/us/350", server.base_url())
    }

    #[tokio::test]
    async fn exporta_para_a_descricao_da_us_de_destino() {
        let server = MockServer::start();
        let hub = setup(&server);
        let patch = server.mock(|when, then| {
            when.method(PATCH)
                .path("/api/v1/userstories/900")
                .json_body_includes(json!({"version": 3}).to_string())
                .body_includes("Entregou o formulário")
                .body_includes("Cadastro (5 pts)")
                .body_includes("Para essa semana");
            then.status(200).json_body(json!({}));
        });
        let (titulo, r) = export_to_us(&hub, &url(&server), &[it(1, "Entregou o formulário")]).await.unwrap();
        assert_eq!((titulo.as_str(), r), ("Reunião semanal", 350));
        patch.assert();
    }

    #[tokio::test]
    async fn conflito_409_rele_a_us_e_tenta_de_novo() {
        let server = MockServer::start();
        let hub = setup(&server);
        let mut conflito = server.mock(|when, then| {
            when.method(PATCH).path("/api/v1/userstories/900");
            then.status(409).json_body(json!({"_error_message": "version"}));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/v1/userstories/900");
            then.status(200).json_body(json!({"id": 900, "subject": "Reunião semanal", "version": 4, "description": "Pauta."}));
        });
        let u = url(&server);
        let falha = export_to_us(&hub, &u, &[it(1, "x")]).await;
        assert!(falha.is_err(), "409 nas duas tentativas = erro (nada é marcado como prestado)");
        conflito.delete();
        let ok = server.mock(|when, then| {
            when.method(PATCH).path("/api/v1/userstories/900");
            then.status(200).json_body(json!({}));
        });
        assert!(export_to_us(&hub, &u, &[it(1, "x")]).await.is_ok());
        ok.assert();
    }

    #[tokio::test]
    async fn erros_de_url_e_de_conta() {
        let server = MockServer::start();
        let hub = setup(&server);
        assert!(export_to_us(&hub, "https://t.x/project/p/task/1", &[it(1, "x")]).await.unwrap_err().contains("URL inválida"));
        let outro = export_to_us(&hub, "https://outro-taiga.com/project/p/us/1", &[it(1, "x")]).await.unwrap_err();
        assert!(outro.contains("Nenhuma conta Taiga sua usa o host outro-taiga.com"), "{outro}");
    }
}
