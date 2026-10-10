use crate::db;
use rusqlite::Connection;
use serde::Serialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

const TEXT_LIMIT: u64 = 1024 * 1024;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub preview: &'static str,
    pub size: u64,
}
pub fn format(file: &Path) -> (&'static str, &'static str) {
    match file
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
        .as_str()
    {
        "pdf" => ("pdf", "application/pdf"),
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "txt" | "md" | "csv" | "json" | "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "css"
        | "toml" | "yaml" | "yml" | "c" | "h" | "cpp" | "hpp" | "java" | "go" | "rb" | "php"
        | "sh" | "sql" | "kt" | "swift" | "cs" | "ipynb" => ("text", "text/plain; charset=utf-8"),
        _ => ("external", "application/octet-stream"),
    }
}
pub fn file(conn: &Connection, id: i64, data: &Path) -> Result<PathBuf, String> {
    let (file, kind) = db::asset_file(conn, id, data)?;
    if kind != "resource" {
        return Err("Not a resource".into());
    }
    Ok(file)
}
pub fn info(file: &Path) -> Result<Resource, String> {
    let size = std::fs::metadata(file).map_err(|e| e.to_string())?.len();
    let preview = format(file).0;
    Ok(Resource {
        preview: if (preview == "text" && size > TEXT_LIMIT)
            || (preview == "image" && size > 16 * 1024 * 1024)
        {
            "external"
        } else {
            preview
        },
        size,
    })
}
pub fn text(file: &Path) -> Result<String, String> {
    if format(file).0 != "text" {
        return Err("This resource is not plain text".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(file)
        .map_err(|e| e.to_string())?
        .take(TEXT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > TEXT_LIMIT {
        return Err("Text preview is limited to 1 MiB; open it in your system app".into());
    }
    String::from_utf8(bytes)
        .map_err(|_| "This file is not UTF-8 text; open it in your system app".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previews_are_passive_and_text_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("script.js");
        std::fs::write(&file, "<script>alert(1)</script>").unwrap();
        assert_eq!(text(&file).unwrap(), "<script>alert(1)</script>");
        assert_eq!(format(&dir.path().join("page.html")).0, "external");
        assert_eq!(format(&dir.path().join("image.svg")).0, "external");
        assert_eq!(format(&dir.path().join("BOOK.PDF")).0, "pdf");
        for name in [
            "exercise.tsx",
            "main.cpp",
            "query.sql",
            "script.sh",
            "notebook.ipynb",
        ] {
            assert_eq!(format(&dir.path().join(name)).0, "text");
        }
        std::fs::write(&file, vec![b'x'; TEXT_LIMIT as usize + 1]).unwrap();
        assert!(text(&file).is_err());
        assert_eq!(info(&file).unwrap().preview, "external");
        std::fs::write(&file, [0xff]).unwrap();
        assert!(text(&file).is_err());
        let large = dir.path().join("large.png");
        std::fs::File::create(&large)
            .unwrap()
            .set_len(16 * 1024 * 1024 + 1)
            .unwrap();
        assert_eq!(info(&large).unwrap().preview, "external");
    }
}
