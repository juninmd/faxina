//! Read-only command-line front end (`faxina-cli`): scan, search and list junk as JSON, CSV or
//! HTML for scripts and CI. It never deletes anything; removal stays in the app, behind the guard.

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::export::{self, Format};
use crate::model::Kind;
use crate::query::{self, Query};
use crate::scan::{self, Progress};
use crate::{guard, junk};

pub const USAGE: &str = "faxina-cli — leitura apenas, nunca apaga nada

USO
  faxina-cli search <pasta> [filtros] [--format json|csv|html]
  faxina-cli scan <pasta> [--top N]
  faxina-cli junk

FILTROS (search)
  --text <texto>       nome contém o texto; use * e ? para curingas
  --min-size <tam>     ex.: 500MB, 2GB, 100k
  --older-than <idade> parado há: 30d, 12m (meses), 2y
  --kind <tipo>        cache|build|git|code|media|documents|archives|apps|other
  --dirs               inclui pastas
  --limit <n>          máximo de resultados (padrão 200)
";

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `500MB`, `2g`, `100k`, `1234` (bytes). Binary multiples, case-insensitive.
pub fn parse_size(s: &str) -> Result<u64, String> {
    let t = s.trim().to_ascii_lowercase();
    let digits = t.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let unit = &t[digits.len()..];
    let mult: u64 = match unit {
        "" | "b" => 1,
        "k" | "kb" => 1 << 10,
        "m" | "mb" => 1 << 20,
        "g" | "gb" => 1 << 30,
        "t" | "tb" => 1 << 40,
        _ => return Err(format!("tamanho inválido: {s}")),
    };
    let n: f64 = digits
        .parse()
        .map_err(|_| format!("tamanho inválido: {s}"))?;
    let bytes = n * mult as f64;
    if !(n.is_finite() && n >= 0.0 && bytes.is_finite() && bytes <= u64::MAX as f64) {
        return Err(format!("tamanho inválido: {s}"));
    }
    Ok(bytes as u64)
}

/// `30d`, `12m` (months of 30 days), `2y`, or plain days.
pub fn parse_age(s: &str) -> Result<u64, String> {
    let t = s.trim().to_ascii_lowercase();
    let (digits, unit) = match t.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&t[..t.len() - 1], c),
        _ => (t.as_str(), 'd'),
    };
    let n: u64 = digits.parse().map_err(|_| format!("idade inválida: {s}"))?;
    let days = match unit {
        'd' => 1,
        'm' => 30,
        'y' => 365,
        _ => return Err(format!("idade inválida: {s}")),
    };
    Ok(n.saturating_mul(days).saturating_mul(24 * 3600))
}

fn parse_kind(s: &str) -> Result<Kind, String> {
    serde_json::from_value(serde_json::Value::String(s.to_ascii_lowercase()))
        .map_err(|_| format!("tipo inválido: {s}"))
}

struct Args {
    rest: Vec<String>,
}

impl Args {
    fn flag(&mut self, name: &str) -> bool {
        let before = self.rest.len();
        self.rest.retain(|a| a != name);
        self.rest.len() != before
    }

    fn value(&mut self, name: &str) -> Result<Option<String>, String> {
        let Some(i) = self.rest.iter().position(|a| a == name) else {
            return Ok(None);
        };
        if i + 1 >= self.rest.len() {
            return Err(format!("{name} precisa de um valor"));
        }
        let v = self.rest.remove(i + 1);
        self.rest.remove(i);
        Ok(Some(v))
    }
}

fn folder(args: &mut Args) -> Result<PathBuf, String> {
    let rest = std::mem::take(&mut args.rest);
    if let Some(flag) = rest
        .iter()
        .find(|a| a.starts_with('-') && (rest.len() != 1 || !std::path::Path::new(a).is_dir()))
    {
        return Err(format!("opção desconhecida: {flag}"));
    }
    let [path] = <[String; 1]>::try_from(rest).map_err(|_| "informe exatamente uma pasta")?;
    let p = PathBuf::from(path);
    if p.is_dir() {
        Ok(p)
    } else {
        Err(format!("pasta não encontrada: {}", p.display()))
    }
}

fn analyze(root: &std::path::Path) -> crate::model::Node {
    scan::scan(root, &Progress::default())
}

pub fn run(args: &[String], out: &mut dyn Write) -> Result<(), String> {
    let Some((cmd, rest)) = args.split_first() else {
        return Err(USAGE.into());
    };
    let mut a = Args {
        rest: rest.to_vec(),
    };
    let io = |e: std::io::Error| e.to_string();
    match cmd.as_str() {
        "search" => {
            let format = match a.value("--format")? {
                Some(f) => Format::parse(&f).ok_or_else(|| format!("formato inválido: {f}"))?,
                None => Format::Json,
            };
            let q = Query {
                text: a.value("--text")?.unwrap_or_default(),
                min_size: a.value("--min-size")?.map_or(Ok(0), |v| parse_size(&v))?,
                older_than_secs: a.value("--older-than")?.map_or(Ok(0), |v| parse_age(&v))?,
                kind: a.value("--kind")?.map(|v| parse_kind(&v)).transpose()?,
                include_dirs: a.flag("--dirs"),
                limit: a.value("--limit")?.map_or(Ok(0), |v| {
                    v.parse().map_err(|_| format!("limite inválido: {v}"))
                })?,
            };
            let root = folder(&mut a)?;
            let tree = analyze(&root);
            let protection = guard::Protection::new();
            let hits = query::search(&tree, &root, &q, now(), |p| !protection.is_protected(p));
            let body = export::render(&hits, format, &root.to_string_lossy(), now());
            out.write_all(body.as_bytes()).map_err(io)
        }
        "scan" => {
            let top: usize = a.value("--top")?.map_or(Ok(20), |v| {
                v.parse().map_err(|_| format!("limite inválido: {v}"))
            })?;
            let root = folder(&mut a)?;
            let tree = analyze(&root);
            let protection = guard::Protection::new();
            let biggest = crate::view::biggest_files(&tree, &root, top.clamp(1, 1000), |p| {
                !protection.is_protected(p)
            });
            let doc = serde_json::json!({
                "root": root,
                "bytes": tree.size,
                "files": tree.files,
                "biggest": biggest,
            });
            writeln!(
                out,
                "{}",
                serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?
            )
            .map_err(io)
        }
        "junk" => {
            if !a.rest.is_empty() {
                return Err(USAGE.into());
            }
            let (items, _) = junk::scan_all();
            writeln!(
                out,
                "{}",
                serde_json::to_string_pretty(&items).map_err(|e| e.to_string())?
            )
            .map_err(io)
        }
        "-h" | "--help" | "help" => out.write_all(USAGE.as_bytes()).map_err(io),
        other => Err(format!("comando desconhecido: {other}\n\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Result<String, String> {
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        run(&argv, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn sizes_and_ages_parse() {
        assert_eq!(parse_size("500MB"), Ok(500 << 20));
        assert_eq!(parse_size("1.5g"), Ok(3 << 29));
        assert_eq!(parse_size("1234"), Ok(1234));
        assert!(parse_size("abc").is_err());
        assert!(parse_size("5xb").is_err());
        assert!(parse_size("999999999999999TB").is_err());
        assert!(parse_size("1e300").is_err());
        assert!(parse_size("-5MB").is_err());
        assert_eq!(parse_age("12m"), Ok(12 * 30 * 86400));
        assert_eq!(parse_age("30"), Ok(30 * 86400));
        assert_eq!(parse_age("2y"), Ok(2 * 365 * 86400));
        assert!(parse_age("2w").is_err());
    }

    #[test]
    fn search_lists_matches_as_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("alpha.log"), vec![0u8; 700 * 1024]).unwrap();
        std::fs::write(dir.path().join("beta.txt"), b"x").unwrap();
        let out = cli(&["search", dir.path().to_str().unwrap(), "--text", "*.log"]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["count"], 1);
        assert_eq!(v["items"][0]["name"], "alpha.log");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(cli(&[]).is_err());
        assert!(cli(&["nope"]).is_err());
        assert!(cli(&["search"]).is_err());
        assert!(cli(&["search", "/definitely/missing"]).is_err());
        assert!(cli(&["search", "/tmp", "--format", "pdf"]).is_err());
        assert!(cli(&["search", "/tmp", "--limit"]).is_err());
        let err = cli(&["search", "/tmp", "--bogus"]).unwrap_err();
        assert!(err.contains("--bogus"), "got: {err}");
    }
}
