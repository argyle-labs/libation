use plugin_toolkit::prelude::*;
use plugin_toolkit::serde_json::Value;

#[orca_struct(args)]
pub struct ExportArgs {
    /// Path to a `LibationCli export --json -p <file>` export on this host.
    #[arg(long)]
    pub file: String,
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

fn load(file: &str) -> Result<Vec<LibationBook>> {
    if !file.ends_with(".json") {
        bail!("export file must be a .json Libation export");
    }
    let body = std::fs::read_to_string(file).map_err(|_| anyhow!("cannot read export file"))?;
    parse_export(&body)
}

/// List the Audible library from a Libation JSON export: ASIN, title, authors,
/// narrators, series + order and liberation status. The purchased-audiobook
/// inventory for gap audits against audiobookshelf/LazyLibrarian.
#[orca_tool(domain = "libation", verb = "library.list", role = "admin")]
async fn libation_library_list(args: ExportArgs, _ctx: &ToolCtx) -> Result<Vec<LibationBook>> {
    let books = load(&args.file)?;
    Ok(match args.status {
        Some(want) => books
            .into_iter()
            .filter(|b| b.status.eq_ignore_ascii_case(&want))
            .collect(),
        None => books,
    })
}

/// Count books in a Libation JSON export by liberation status.
#[orca_tool(domain = "libation", verb = "library.summary", role = "admin")]
async fn libation_library_summary(args: ExportArgs, _ctx: &ToolCtx) -> Result<LibationSummary> {
    Ok(summarize(&load(&args.file)?))
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
    fn tools_declare_admin_role() {
        for n in ["libation.library.list", "libation.library.summary"] {
            assert_eq!(
                plugin_toolkit::dispatch::required_role(n),
                Some("admin"),
                "{n}"
            );
        }
    }
}
