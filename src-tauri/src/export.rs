//! Report rendering for search results: CSV, JSON and a self-contained HTML page. Pure string
//! building, so it is shared by the app command and the command-line tool.

use serde::Deserialize;

use crate::query::Hit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Json,
    Html,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "html" => Some(Self::Html),
            _ => None,
        }
    }
}

pub fn render(hits: &[Hit], format: Format, root: &str, generated_at: u64) -> String {
    match format {
        Format::Csv => csv(hits),
        Format::Json => json(hits, root, generated_at),
        Format::Html => html(hits, root, generated_at),
    }
}

/// RFC 4180 quoting, plus a leading `'` on cells a spreadsheet would run as a formula.
fn csv_cell(s: &str) -> String {
    let guarded = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn kind_name(h: &Hit) -> String {
    serde_json::to_value(h.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn csv(hits: &[Hit]) -> String {
    let mut out = String::from("path,name,size_bytes,modified_unix,kind,is_dir\r\n");
    for h in hits {
        out.push_str(&format!(
            "{},{},{},{},{},{}\r\n",
            csv_cell(&h.path),
            csv_cell(&h.name),
            h.size,
            h.modified,
            kind_name(h),
            h.is_dir
        ));
    }
    out
}

fn json(hits: &[Hit], root: &str, generated_at: u64) -> String {
    let doc = serde_json::json!({
        "root": root,
        "generatedAt": generated_at,
        "count": hits.len(),
        "totalBytes": hits.iter().map(|h| h.size).sum::<u64>(),
        "items": hits,
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    // 1023.95 KB would round up to "1024.0 KB": promote to the next unit instead.
    if i > 0 && i < UNITS.len() - 1 && (v * 10.0).round() >= 10240.0 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn html(hits: &[Hit], root: &str, generated_at: u64) -> String {
    let total: u64 = hits.iter().map(|h| h.size).sum();
    let rows: String = hits
        .iter()
        .map(|h| {
            format!(
                "<tr><td title=\"{p}\">{n}</td><td class=\"n\">{s}</td><td>{k}</td><td class=\"p\">{p}</td></tr>\n",
                p = escape(&h.path),
                n = escape(&h.name),
                s = human_bytes(h.size),
                k = kind_name(h),
            )
        })
        .collect();
    format!(
        "<!doctype html><html lang=\"pt-BR\"><meta charset=\"utf-8\"><title>Faxina — relatório</title>\n\
<style>body{{font:14px system-ui;margin:2rem;color:#1e293b}}table{{border-collapse:collapse;width:100%}}\
td,th{{padding:.4rem .6rem;border-bottom:1px solid #e2e8f0;text-align:left}}.n{{text-align:right;white-space:nowrap}}\
.p{{color:#64748b;font:12px ui-monospace,monospace;word-break:break-all}}</style>\n\
<h1>Relatório do Faxina</h1><p>{root} · {count} itens · {total} · gerado em {at} (Unix)</p>\n\
<table><tr><th>Nome</th><th>Tamanho</th><th>Tipo</th><th>Caminho</th></tr>\n{rows}</table></html>\n",
        root = escape(root),
        count = hits.len(),
        total = human_bytes(total),
        at = generated_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn hit(name: &str, size: u64) -> Hit {
        Hit {
            path: format!("/r/{name}"),
            name: name.into(),
            size,
            modified: 7,
            kind: Kind::Media,
            is_dir: false,
        }
    }

    #[test]
    fn csv_quotes_and_neutralizes_formulas() {
        let out = csv(&[hit("a,b\"c", 1), hit("=cmd()", 2)]);
        assert!(out.contains("\"/r/a,b\"\"c\""));
        assert!(out.contains(",'=cmd(),"));
        assert!(out.starts_with("path,name,size_bytes"));
    }

    #[test]
    fn html_escapes_names() {
        let out = html(&[hit("<script>x</script>", 1)], "/r", 0);
        assert!(!out.contains("<script>x"));
        assert!(out.contains("&lt;script&gt;"));
    }

    #[test]
    fn json_reports_totals() {
        let out = json(&[hit("a", 10), hit("b", 5)], "/r", 3);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["count"], 2);
        assert_eq!(v["totalBytes"], 15);
        assert_eq!(v["items"][0]["kind"], "media");
    }

    #[test]
    fn format_parse_is_case_insensitive() {
        assert_eq!(Format::parse("CSV"), Some(Format::Csv));
        assert_eq!(Format::parse("pdf"), None);
    }

    #[test]
    fn human_bytes_scales() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes((1 << 20) - 41), "1.0 MB");
        assert_eq!(human_bytes((1 << 30) - 1024), "1.0 GB");
    }
}
