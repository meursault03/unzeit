use crate::{
    db::{self, Db},
    subtitles::{self, Cue},
};
use rusqlite::{params, Connection};
use std::{io::Read, path::Path};

pub fn read(file: &Path) -> Result<Vec<Cue>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(file)
        .map_err(|e| e.to_string())?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Caption is larger than 8 MiB".into());
    }
    subtitles::cues(&String::from_utf8(bytes).map_err(|e| e.to_string())?)
}
pub fn store(conn: &mut Connection, asset: i64, lesson: i64, cues: &[Cue]) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let valid: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE id=?1 AND kind='caption' AND lesson_id=?2)",
            params![asset, lesson],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !valid {
        return Err("Caption changed while indexing; rescan the course".into());
    }
    let existing = {
        let mut stmt = tx
            .prepare("SELECT position,end,text FROM caption_cues WHERE asset_id=?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([asset], |row| {
                Ok(Cue {
                    position: row.get(0)?,
                    end: row.get(1)?,
                    text: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    if existing == cues {
        return Ok(());
    }
    tx.execute("DELETE FROM caption_cues WHERE asset_id=?1", [asset])
        .map_err(|e| e.to_string())?;
    {
        let mut insert = tx.prepare("INSERT INTO caption_cues(asset_id,lesson_id,position,end,text) VALUES(?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
        for cue in cues {
            insert
                .execute(params![asset, lesson, cue.position, cue.end, cue.text])
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}
pub fn index(db: &Db, data: &Path, id: i64) -> Result<Vec<Cue>, String> {
    let (file, lesson) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let (file, kind) = db::asset_file(&conn, id, data)?;
        if kind != "caption" {
            return Err("Not a caption".into());
        }
        let lesson = conn
            .query_row("SELECT lesson_id FROM assets WHERE id=?1", [id], |r| {
                r.get::<_, i64>(0)
            })
            .map_err(|e| e.to_string())?;
        (file, lesson)
    };
    let cues = read(&file)?;
    let mut conn = db.lock().map_err(|e| e.to_string())?;
    if db::asset_file(&conn, id, data)?.0 != file {
        return Err("Caption changed while indexing; rescan the course".into());
    }
    store(&mut conn, id, lesson, &cues)?;
    Ok(cues)
}
pub fn reindex(db: &Db, data: &Path, course: Option<i64>) -> Result<Vec<String>, String> {
    let ids = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id,title,path,lesson_id FROM assets WHERE kind='caption' AND lesson_id IS NOT NULL AND (?1 IS NULL OR course_id=?1)").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([course], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };
    let mut warnings = Vec::new();
    for (id, title, path, lesson) in ids {
        if let Err(e) = index(db, data, id) {
            db.lock()
                .map_err(|e| e.to_string())?
                .execute("DELETE FROM caption_cues WHERE asset_id IN (SELECT id FROM assets WHERE id=?1 AND path=?2 AND lesson_id=?3 AND kind='caption')", params![id,path,lesson])
                .map_err(|e| e.to_string())?;
            warnings.push(format!("{title}: {e}"));
        }
    }
    Ok(warnings)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caption_rowid_migration_preserves_search_and_learning_data() {
        let conn = Connection::open_in_memory().unwrap();
        for migration in [
            include_str!("../migrations/001_initial.sql"),
            include_str!("../migrations/002_search.sql"),
            include_str!("../migrations/003_caption_search.sql"),
            include_str!("../migrations/004_study_time.sql"),
        ] {
            conn.execute_batch(migration).unwrap();
        }
        conn.execute_batch("INSERT INTO courses(id,title,path,last_lesson_id) VALUES(1,'Course','/course',1);
            INSERT INTO lessons(id,course_id,title,section,path,sort_order,position,completed) VALUES(1,1,'Lesson','Basics','1.mp4',0,42,1);
            INSERT INTO assets(id,course_id,lesson_id,title,path,kind) VALUES(1,1,1,'Caption','1.srt','caption');
            INSERT INTO notes(id,lesson_id,position,text) VALUES(1,1,10,'Bookmark');
            INSERT INTO caption_cues(id,asset_id,lesson_id,position,end,text) VALUES(1,1,1,2,3,'Ownership');
            INSERT INTO caption_cues(id,asset_id,lesson_id,position,end,text) VALUES(2,1,1,3,4,'Borrowing');").unwrap();
        db::initialize(&conn).unwrap();
        assert_eq!(
            crate::search::search(&conn, "ownership").unwrap()[0].position,
            Some(2.)
        );
        assert_eq!(crate::search::search(&conn, "bookmark").unwrap().len(), 1);
        let lesson = &db::library(&conn).unwrap()[0].lessons[0];
        assert_eq!(lesson.position, 42.);
        assert!(lesson.completed);
        conn.execute_batch("DELETE FROM caption_cues WHERE id=1;
            INSERT INTO caption_cues(id,asset_id,lesson_id,position,end,text) VALUES(3,1,1,4,5,'Lifetimes');
            UPDATE notes SET text='Remember' WHERE id=1;").unwrap();
        assert!(crate::search::search(&conn, "ownership")
            .unwrap()
            .is_empty());
        assert_eq!(crate::search::search(&conn, "borrow").unwrap().len(), 1);
        assert_eq!(crate::search::search(&conn, "lifetime").unwrap().len(), 1);
        assert_eq!(crate::search::search(&conn, "remember").unwrap().len(), 1);
        conn.execute(
            "INSERT INTO library_search(library_search) VALUES('integrity-check')",
            [],
        )
        .unwrap();
        db::initialize(&conn).unwrap();
        assert_eq!(crate::search::search(&conn, "borrow").unwrap().len(), 1);
        conn.execute("DELETE FROM assets WHERE id=1", []).unwrap();
        assert!(crate::search::search(&conn, "borrow").unwrap().is_empty());
        assert!(crate::search::search(&conn, "lifetime").unwrap().is_empty());
    }
    #[test]
    #[ignore = "manual 40,000-cue performance benchmark"]
    fn caption_indexing_benchmark() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::initialize(&conn).unwrap();
        conn.execute(
            "INSERT INTO courses(id,title,path) VALUES(1,'Course','/course')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO lessons(id,course_id,title,section,path,sort_order) VALUES(1,1,'Lesson','Basics','1.mp4',0)", []).unwrap();
        let tx = conn.transaction().unwrap();
        for asset in 1..=400 {
            tx.execute("INSERT INTO assets(id,course_id,lesson_id,title,path,kind) VALUES(?1,1,1,'Caption',?2,'caption')", params![asset, format!("{asset}.srt")]).unwrap();
            for cue in 0..100 {
                tx.execute("INSERT INTO caption_cues(asset_id,lesson_id,position,end,text) VALUES(?1,1,?2,?3,'Learning programming')", params![asset, cue as f64, cue as f64 + 1.0]).unwrap();
            }
        }
        tx.commit().unwrap();
        let mut cues: Vec<Cue> = (0..100)
            .map(|cue| Cue {
                position: cue as f64,
                end: cue as f64 + 1.0,
                text: "Learning programming".into(),
            })
            .collect();
        let mut unchanged = Vec::new();
        let mut changed = Vec::new();
        for run in 0..3 {
            let writes = conn.total_changes();
            let start = std::time::Instant::now();
            store(&mut conn, 1, 1, &cues).unwrap();
            unchanged.push((
                start.elapsed().as_secs_f64() * 1000.,
                conn.total_changes() - writes,
            ));
            cues[0].text = format!("Changed caption {run}");
            let start = std::time::Instant::now();
            store(&mut conn, 1, 1, &cues).unwrap();
            changed.push(start.elapsed().as_secs_f64() * 1000.);
        }
        eprintln!("40,000 cues; unchanged (ms,writes): {unchanged:?}; changed ms: {changed:?}");
    }
    #[test]
    fn caption_search_tracks_rescans_and_deletions_without_losing_progress() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("1.mp4"), "video").unwrap();
        let caption = dir.path().join("1.srt");
        std::fs::write(
            &caption,
            "1\n00:00:02,000 --> 00:00:04,000\nOwnership and borrowing",
        )
        .unwrap();
        let db = db::open(&dir.path().join("db.sqlite")).unwrap();
        db::import(
            &mut db.lock().unwrap(),
            dir.path(),
            &[crate::scanner::scan_course(dir.path(), dir.path()).unwrap()],
        )
        .unwrap();
        assert!(reindex(&db, dir.path(), None).unwrap().is_empty());
        let result = crate::search::search(&db.lock().unwrap(), "borrow")
            .unwrap()
            .remove(0);
        assert_eq!(result.kind, "caption");
        assert_eq!(result.position, Some(2.0));
        let asset: i64 = db
            .lock()
            .unwrap()
            .query_row("SELECT id FROM assets WHERE kind='caption'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let cues = read(&caption).unwrap();
        let changes = db.lock().unwrap().total_changes();
        reindex(&db, dir.path(), None).unwrap();
        assert_eq!(
            db.lock().unwrap().total_changes(),
            changes,
            "Unchanged captions must not rewrite SQLite or FTS"
        );
        assert!(store(
            &mut db.lock().unwrap(),
            asset,
            result.lesson_id.unwrap() + 100,
            &cues
        )
        .is_err());
        assert_eq!(
            crate::search::search(&db.lock().unwrap(), "borrow")
                .unwrap()
                .len(),
            1
        );
        db::progress(
            &db.lock().unwrap(),
            result.lesson_id.unwrap(),
            10.0,
            100.0,
            Some(true),
        )
        .unwrap();
        std::fs::write(&caption, "1\n00:00:03,000 --> 00:00:05,000\nMemory safety").unwrap();
        reindex(&db, dir.path(), None).unwrap();
        assert!(crate::search::search(&db.lock().unwrap(), "borrow")
            .unwrap()
            .is_empty());
        assert_eq!(
            crate::search::search(&db.lock().unwrap(), "safety").unwrap()[0].position,
            Some(3.0)
        );
        std::fs::remove_file(caption).unwrap();
        db::import(
            &mut db.lock().unwrap(),
            dir.path(),
            &[crate::scanner::scan_course(dir.path(), dir.path()).unwrap()],
        )
        .unwrap();
        assert!(crate::search::search(&db.lock().unwrap(), "safety")
            .unwrap()
            .is_empty());
        let library = db::library(&db.lock().unwrap()).unwrap();
        assert_eq!(library[0].lessons[0].position, 10.0);
        assert!(library[0].lessons[0].completed);
    }
}
