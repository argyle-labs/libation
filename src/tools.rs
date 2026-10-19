use plugin_toolkit::prelude::*;
use plugin_toolkit::serde_json::Value;

use crate::endpoint_db;

#[orca_struct(args)]
pub struct ExportArgs {
    /// Registered libation endpoint name.
    #[arg(long)]
    pub name: String,
    /// Only return books whose BookStatus matches (e.g. Liberated, NotLiberated, Error).
    #[arg(long)]
    pub status: Option<String>,
}

#[orca_struct]
#[derive(Debug, Clone, PartialEq)]
pub struct LibationBook {
    pub asin: String,
    pub title: String,
    pub authors: String,
    pub narrators: String,
    pub series: String,
    pub series_order: String,
    pub status: String,
    pub date_published: String,
    pub length_minutes: i64,
}

#[orca_struct]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LibationSummary {
    pub total: usize,
    pub liberated: usize,
    pub not_liberated: usize,
    pub error: usize,
    pub other: usize,
}

fn text(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

pub fn parse_export(json: &str) -> Result<Vec<LibationBook>> {
    let root: Value = plugin_toolkit::serde_json::from_str(json)
        .map_err(|_| anyhow!("not a Libation JSON export"))?;
    let rows = root
        .as_array()
        .ok_or_else(|| anyhow!("not a Libation JSON export"))?;
    if rows.len() > MAX_ROWS {
        bail!("Libation export exceeds {MAX_ROWS} rows");
    }
    Ok(rows
        .iter()
        .map(|r| LibationBook {
            asin: text(r, "AudibleProductId"),
            title: text(r, "Title"),
            authors: text(r, "AuthorNames"),
            narrators: text(r, "NarratorNames"),
            series: text(r, "SeriesNames"),
            series_order: text(r, "SeriesOrder"),
            status: text(r, "BookStatus"),
            date_published: text(r, "DatePublished"),
            length_minutes: r
                .get("LengthInMinutes")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        })
        .collect())
}

pub fn summarize(books: &[LibationBook]) -> LibationSummary {
    let mut s = LibationSummary {
        total: books.len(),
        ..Default::default()
    };
    for b in books {
        match b.status.as_str() {
            "Liberated" => s.liberated += 1,
            "NotLiberated" => s.not_liberated += 1,
            "Error" => s.error += 1,
            _ => s.other += 1,
        }
    }
    s
}

const MAX_EXPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ROWS: usize = 100_000;

fn read_export(file: &str) -> Option<String> {
    use std::io::Read;
    let path = std::fs::canonicalize(file).ok()?;
    if path.extension()? != "json" {
        return None;
    }
    let f = std::fs::File::open(&path).ok()?;
    let meta = f.metadata().ok()?;
    if !meta.is_file() || meta.len() > MAX_EXPORT_BYTES {
        return None;
    }
    let mut body = String::new();
    f.take(MAX_EXPORT_BYTES + 1)
        .read_to_string(&mut body)
        .ok()?;
    (body.len() as u64 <= MAX_EXPORT_BYTES).then_some(body)
}

fn load_endpoint(name: &str) -> Result<Vec<LibationBook>> {
    load(&endpoint_db::require(name)?.export_path)
}

fn load(file: &str) -> Result<Vec<LibationBook>> {
    let body = read_export(file).ok_or_else(|| anyhow!("not a readable Libation JSON export"))?;
    parse_export(&body).map_err(|_| anyhow!("not a readable Libation JSON export"))
}

/// List the Audible library from a Libation JSON export: ASIN, title, authors,
/// narrators, series + order and liberation status. The purchased-audiobook
/// inventory for gap audits against audiobookshelf/LazyLibrarian.
#[orca_tool(domain = "libation", verb = "library.list", role = "read")]
async fn libation_library_list(args: ExportArgs, _ctx: &ToolCtx) -> Result<Vec<LibationBook>> {
    let books = load_endpoint(&args.name)?;
    Ok(match args.status {
        Some(want) => books
            .into_iter()
            .filter(|b| b.status.eq_ignore_ascii_case(&want))
            .collect(),
        None => books,
    })
}

/// Count books in a Libation JSON export by liberation status.
#[orca_tool(domain = "libation", verb = "library.summary", role = "read")]
async fn libation_library_summary(args: ExportArgs, _ctx: &ToolCtx) -> Result<LibationSummary> {
    Ok(summarize(&load_endpoint(&args.name)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = r#"[
        {"AudibleProductId":"B01","Title":"We Are Legion","AuthorNames":"Dennis E. Taylor","NarratorNames":"Ray Porter","SeriesNames":"Bobiverse","SeriesOrder":"1","BookStatus":"Liberated","DatePublished":"2016-09-20T00:00:00","LengthInMinutes":580},
        {"AudibleProductId":"B02","Title":"For We Are Many","AuthorNames":"Dennis E. Taylor","SeriesNames":"Bobiverse","SeriesOrder":2,"BookStatus":"NotLiberated","DatePublished":null},
        {"AudibleProductId":"B03","Title":"Broken","BookStatus":"Error"}
    ]"#;

    #[test]
    fn parses_export_rows() {
        let books = parse_export(EXPORT).unwrap();
        assert_eq!(books.len(), 3);
        assert_eq!(books[0].asin, "B01");
        assert_eq!(books[0].length_minutes, 580);
        assert_eq!(books[1].series_order, "2");
        assert_eq!(books[1].narrators, "");
        assert_eq!(books[1].date_published, "");
    }

    #[test]
    fn summarizes_by_status() {
        let s = summarize(&parse_export(EXPORT).unwrap());
        assert_eq!(
            s,
            LibationSummary {
                total: 3,
                liberated: 1,
                not_liberated: 1,
                error: 1,
                other: 0
            }
        );
    }

    #[test]
    fn rejects_non_export_without_echoing_content() {
        let err = parse_export("{\"secret\":\"hunter2\"}")
            .unwrap_err()
            .to_string();
        assert!(!err.contains("hunter2"));
        assert!(load("/etc/passwd").is_err());
    }

    #[test]
    fn load_rejects_symlink_to_non_json_and_dirs() {
        let dir = std::env::temp_dir().join(format!("libation-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("d.json")).unwrap();
        let target = dir.join("secret.txt");
        std::fs::write(&target, "[]").unwrap();
        let link = dir.join("link.json");
        std::fs::remove_file(&link).ok();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let ok = dir.join("ok.json");
        std::fs::write(&ok, EXPORT).unwrap();
        let e1 = load(link.to_str().unwrap()).unwrap_err().to_string();
        let e2 = load(dir.join("d.json").to_str().unwrap())
            .unwrap_err()
            .to_string();
        let e3 = load(dir.join("missing.json").to_str().unwrap())
            .unwrap_err()
            .to_string();
        assert_eq!(e1, e2);
        assert_eq!(e2, e3);
        assert_eq!(load(ok.to_str().unwrap()).unwrap().len(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_export_over_row_cap() {
        let body = format!("[{}{{}}]", "{},".repeat(MAX_ROWS));
        assert!(parse_export(&body).is_err());
        let body = format!("[{}{{}}]", "{},".repeat(MAX_ROWS - 1));
        assert_eq!(parse_export(&body).unwrap().len(), MAX_ROWS);
    }

    #[test]
    fn read_tools_are_read_role_and_endpoint_writes_admin() {
        for n in ["libation.library.list", "libation.library.summary"] {
            assert_eq!(
                plugin_toolkit::dispatch::required_role(n),
                Some("read"),
                "{n}"
            );
        }
        for n in ["libation.create", "libation.update", "libation.delete"] {
            assert_eq!(
                plugin_toolkit::dispatch::required_role(n),
                Some("admin"),
                "{n}"
            );
        }
    }

    #[test]
    fn export_args_take_endpoint_name_not_file() {
        let args: ExportArgs = plugin_toolkit::serde_json::from_value(
            plugin_toolkit::serde_json::json!({"name": "home"}),
        )
        .unwrap();
        assert_eq!(args.name, "home");
        assert!(args.status.is_none());
    }
}
