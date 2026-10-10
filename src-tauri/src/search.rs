use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub kind: String,
    pub id: i64,
    pub course_id: i64,
    pub course_title: String,
    pub lesson_id: Option<i64>,
    pub lesson_title: Option<String>,
    pub position: Option<f64>,
    pub snippet: String,
    pub missing: bool,
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, String> {
    if query.chars().count() > 256 {
        return Err("Search is limited to 256 characters".into());
    }
    // Quote literal terms so filenames and notes cannot become FTS operators.
    let terms: Vec<_> = query
        .split_whitespace()
        .filter(|term| term.chars().any(char::is_alphanumeric))
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    if terms.len() > 24 {
        return Err("Search is limited to 24 words".into());
    }
    let mut stmt = conn.prepare("SELECT s.kind,s.item_id,c.id,c.title,l.id,l.title,COALESCE(n.position,cue.position),
        snippet(library_search,-1,'','',' … ',24),COALESCE(l.missing,0)
        FROM library_search s
        LEFT JOIN notes n ON s.kind='note' AND n.id=s.item_id
        LEFT JOIN caption_cues cue ON s.kind='caption' AND cue.id=s.item_id
        LEFT JOIN lessons l ON (s.kind='lesson' AND l.id=s.item_id) OR (s.kind='note' AND l.id=n.lesson_id) OR (s.kind='caption' AND l.id=cue.lesson_id)
        JOIN courses c ON (s.kind='course' AND c.id=s.item_id) OR (s.kind IN ('lesson','note','caption') AND c.id=l.course_id)
        WHERE library_search MATCH ?1 ORDER BY rank,c.title,s.item_id LIMIT 100").map_err(|e| e.to_string())?;
    let results = stmt
        .query_map(params![terms.join(" AND ")], |r| {
            Ok(SearchResult {
                kind: r.get(0)?,
                id: r.get(1)?,
                course_id: r.get(2)?,
                course_title: r.get(3)?,
                lesson_id: r.get(4)?,
                lesson_title: r.get(5)?,
                position: r.get(6)?,
                snippet: r.get(7)?,
                missing: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    results.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    #[test]
    fn migration_indexes_existing_data_and_keeps_updates_in_sync() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        conn.execute_batch(
            "INSERT INTO courses VALUES(1,'Filosofia','/course',1,123);
            INSERT INTO lessons VALUES(2,1,'Introdução','General','1.mp4',0,20,100,1,0);
            INSERT INTO notes VALUES(3,2,15,'Eterno retorno',0);",
        )
        .unwrap();
        db::initialize(&conn).unwrap();
        assert_eq!(search(&conn, "filo").unwrap()[0].course_id, 1);
        assert_eq!(search(&conn, "introducao").unwrap()[0].lesson_id, Some(2));
        let note = search(&conn, "eterno ret").unwrap().remove(0);
        assert_eq!(note.position, Some(15.0));
        assert_eq!(note.kind, "note");
        assert_eq!(db::library(&conn).unwrap()[0].lessons[0].position, 20.0);
        assert!(db::library(&conn).unwrap()[0].lessons[0].completed);
        conn.execute("UPDATE notes SET text='Perspectivismo' WHERE id=3", [])
            .unwrap();
        assert!(search(&conn, "eterno").unwrap().is_empty());
        assert_eq!(search(&conn, "perspect").unwrap().len(), 1);
        conn.execute("DELETE FROM notes WHERE id=3", []).unwrap();
        assert!(search(&conn, "perspect").unwrap().is_empty());
        conn.execute("UPDATE lessons SET title='Lógica' WHERE id=2", [])
            .unwrap();
        assert!(search(&conn, "introducao").unwrap().is_empty());
        assert_eq!(search(&conn, "logica").unwrap().len(), 1);
        for query in ["\"", "*", "OR", "a OR b", "title:foo", "\" OR 1=1 --"] {
            assert!(search(&conn, query).is_ok(), "{query}");
        }
        assert!(search(&conn, &"x".repeat(257)).is_err());
        db::initialize(&conn).unwrap();
        assert_eq!(search(&conn, "logica").unwrap().len(), 1);
    }
}
