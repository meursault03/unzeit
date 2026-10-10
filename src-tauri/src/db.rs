use crate::{
    models::*,
    scanner::{approved_file, path_string},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub type Db = Arc<Mutex<Connection>>;
pub fn lock_instance(data: &Path) -> Result<std::fs::File, String> {
    let file = std::fs::File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data.join("instance.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|e| {
        format!("Cannot lock Unzeit's library; another instance may be running: {e}")
    })?;
    Ok(file)
}
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
pub fn open(path: &Path) -> Result<Db, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    initialize(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}
pub fn initialize(conn: &Connection) -> Result<(), String> {
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
        .map_err(|e| e.to_string())?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version > 3 {
        return Err("This database needs a newer version of Unzeit".into());
    }
    if version == 0 {
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))
            .map_err(|e| e.to_string())?;
    }
    if version < 2 {
        conn.execute_batch(include_str!("../migrations/002_search.sql"))
            .map_err(|e| e.to_string())?;
    }
    if version < 3 {
        conn.execute_batch(include_str!("../migrations/003_caption_search.sql"))
            .map_err(|e| e.to_string())?;
    }
    conn.execute("UPDATE jobs SET status='interrupted', stage='interrupted', error='App closed before this job finished; retry to continue' WHERE status IN ('running','queued')", []).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn settings(conn: &Connection) -> Result<Settings, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    value
        .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
        .unwrap_or_else(|| Ok(Settings::default()))
}
pub fn initial_settings(conn: &Connection, language: &str) -> Result<Settings, String> {
    let stored: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key='preferences')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !stored {
        save_settings(
            conn,
            &Settings {
                language: language.into(),
                ..Settings::default()
            },
        )?;
    }
    settings(conn)
}
pub fn save_settings(conn: &Connection, value: &Settings) -> Result<(), String> {
    if !value.speed.is_finite()
        || !(0.25..=4.0).contains(&value.speed)
        || !["system", "light", "dark"].contains(&value.theme.as_str())
        || !["pt-BR", "en", "es"].contains(&value.language.as_str())
    {
        return Err("Invalid playback, appearance, or language preference".into());
    }
    conn.execute("INSERT INTO settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(value).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn import(conn: &mut Connection, root: &Path, previews: &[Preview]) -> Result<(), String> {
    let root = path_string(&root.canonicalize().map_err(|e| e.to_string())?)?;
    for (index, preview) in previews.iter().enumerate() {
        conn_path(&root, &preview.path)?;
        if previews[..index].iter().any(|other| {
            Path::new(&preview.path).starts_with(&other.path)
                || Path::new(&other.path).starts_with(&preview.path)
        }) {
            return Err("Choose separate course folders, without overlapping selections".into());
        }
    }
    let mut prefs = settings(conn)?;
    prefs.library_root = root.clone();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for p in previews {
        tx.execute("INSERT INTO courses(title,path) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET title=excluded.title", params![p.title,p.path]).map_err(|e| e.to_string())?;
        let course: i64 = tx
            .query_row("SELECT id FROM courses WHERE path=?1", [&p.path], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        tx.execute("UPDATE lessons SET missing=1 WHERE course_id=?1", [course])
            .map_err(|e| e.to_string())?;
        let mut order = 0;
        for section in &p.sections {
            for lesson in &section.lessons {
                tx.execute("INSERT INTO lessons(course_id,title,section,path,sort_order) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(course_id,path) DO UPDATE SET title=excluded.title, section=excluded.section, sort_order=excluded.sort_order, missing=0", params![course,lesson.title,section.title,lesson.path,order]).map_err(|e| e.to_string())?;
                let id: i64 = tx
                    .query_row(
                        "SELECT id FROM lessons WHERE course_id=?1 AND path=?2",
                        params![course, lesson.path],
                        |r| r.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                for asset in lesson.captions.iter().chain(&lesson.resources) {
                    upsert_asset(&tx, course, Some(id), asset, false)?;
                }
                order += 1;
            }
        }
        for asset in &p.resources {
            upsert_asset(&tx, course, None, asset, false)?;
        }
        if p.warnings.is_empty() {
            let seen: std::collections::HashSet<&str> = p
                .sections
                .iter()
                .flat_map(|s| &s.lessons)
                .flat_map(|l| l.captions.iter().chain(&l.resources))
                .chain(&p.resources)
                .map(|a| a.path.as_str())
                .collect();
            let mut statement = tx
                .prepare("SELECT id,path FROM assets WHERE course_id=?1 AND external=0")
                .map_err(|e| e.to_string())?;
            let assets = statement
                .query_map([course], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            for (id, path) in assets {
                if !seen.contains(path.as_str()) {
                    tx.execute("DELETE FROM assets WHERE id=?1", [id])
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    save_settings(&tx, &prefs)?;
    tx.commit().map_err(|e| e.to_string())
}
fn conn_path(root: &str, folder: &str) -> Result<(), String> {
    let p = Path::new(folder)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !p.is_dir() || !p.starts_with(root) {
        return Err("Course is outside the library root".into());
    }
    Ok(())
}
pub fn upsert_asset(
    conn: &Connection,
    course: i64,
    lesson: Option<i64>,
    asset: &PreviewAsset,
    external: bool,
) -> Result<(), String> {
    conn.execute("INSERT INTO assets(course_id,lesson_id,title,path,kind,external) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(course_id,path,kind) DO UPDATE SET lesson_id=excluded.lesson_id,title=excluded.title", params![course,lesson,asset.title,asset.path,asset.kind,external]).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn library(conn: &Connection) -> Result<Vec<Course>, String> {
    let mut stmt = conn.prepare("SELECT id,title,path,last_lesson_id,updated_at FROM courses ORDER BY updated_at DESC,title COLLATE NOCASE").map_err(|e| e.to_string())?;
    let courses = stmt
        .query_map([], |r| {
            Ok(Course {
                id: r.get(0)?,
                title: r.get(1)?,
                path: r.get(2)?,
                last_lesson_id: r.get(3)?,
                updated_at: r.get(4)?,
                lessons: Vec::new(),
                resources: Vec::new(),
            })
        })
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for course in courses {
        let mut course = course.map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id,title,section,path,position,duration,completed,missing FROM lessons WHERE course_id=?1 ORDER BY sort_order,id").map_err(|e| e.to_string())?;
        let lessons = stmt
            .query_map([course.id], |r| {
                Ok(Lesson {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    section: r.get(2)?,
                    path: r.get(3)?,
                    position: r.get(4)?,
                    duration: r.get(5)?,
                    completed: r.get(6)?,
                    missing: r.get(7)?,
                    assets: Vec::new(),
                })
            })
            .map_err(|e| e.to_string())?;
        for lesson in lessons {
            course.lessons.push(lesson.map_err(|e| e.to_string())?);
        }
        let mut stmt = conn
            .prepare("SELECT id,title,kind,lesson_id FROM assets WHERE course_id=?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let assets = stmt
            .query_map([course.id], |r| {
                Ok(Asset {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    kind: r.get(2)?,
                    lesson_id: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for asset in assets {
            let asset = asset.map_err(|e| e.to_string())?;
            if let Some(lesson) = course
                .lessons
                .iter_mut()
                .find(|l| Some(l.id) == asset.lesson_id)
            {
                lesson.assets.push(asset);
            } else {
                course.resources.push(asset);
            }
        }
        result.push(course);
    }
    Ok(result)
}

pub fn lesson_file(conn: &Connection, id: i64) -> Result<(i64, PathBuf, PathBuf), String> {
    let (course, root, rel): (i64,String,String) = conn.query_row("SELECT c.id,c.path,l.path FROM lessons l JOIN courses c ON c.id=l.course_id WHERE l.id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e| e.to_string())?;
    let root = PathBuf::from(root);
    let file = approved_file(&root, &root.join(rel))?;
    Ok((course, root, file))
}
pub fn asset_file(conn: &Connection, id: i64, data: &Path) -> Result<(PathBuf, String), String> {
    let (root, rel, kind, external): (String,String,String,bool) = conn.query_row("SELECT c.path,a.path,a.kind,a.external FROM assets a JOIN courses c ON c.id=a.course_id WHERE a.id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e| e.to_string())?;
    let root = if external {
        data.to_path_buf()
    } else {
        PathBuf::from(root)
    };
    Ok((approved_file(&root, &root.join(rel))?, kind))
}
pub fn progress(
    conn: &Connection,
    id: i64,
    position: f64,
    duration: f64,
    completed: Option<bool>,
) -> Result<(), String> {
    if !position.is_finite()
        || !duration.is_finite()
        || position < 0.0
        || duration < 0.0
        || position > duration + 1.0
    {
        return Err("Invalid watch position".into());
    }
    if conn.execute("UPDATE lessons SET position=?2,duration=?3,completed=COALESCE(?4,completed) WHERE id=?1", params![id,position,duration,completed]).map_err(|e| e.to_string())? == 0 { return Err("Lesson not found".into()); }
    conn.execute("UPDATE courses SET last_lesson_id=?1,updated_at=?2 WHERE id=(SELECT course_id FROM lessons WHERE id=?1)", params![id,now()]).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn notes(conn: &Connection, lesson: i64) -> Result<Vec<Note>, String> {
    let mut stmt = conn.prepare("SELECT id,lesson_id,position,text,bookmark FROM notes WHERE lesson_id=?1 ORDER BY position,id").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([lesson], |r| {
            Ok(Note {
                id: r.get(0)?,
                lesson_id: r.get(1)?,
                position: r.get(2)?,
                text: r.get(3)?,
                bookmark: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}
pub fn save_note(conn: &Connection, note: &Note) -> Result<(), String> {
    if !note.position.is_finite()
        || note.position < 0.0
        || note.text.len() > 20_000
        || (!note.bookmark && note.text.trim().is_empty())
    {
        return Err("Enter a note of at most 20,000 bytes and a valid timestamp".into());
    }
    if note.id == 0 {
        conn.execute(
            "INSERT INTO notes(lesson_id,position,text,bookmark) VALUES(?1,?2,?3,?4)",
            params![
                note.lesson_id,
                note.position,
                note.text.trim(),
                note.bookmark
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        let changed = conn
            .execute(
                "UPDATE notes SET text=?2 WHERE id=?1 AND lesson_id=?3",
                params![note.id, note.text.trim(), note.lesson_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("Note not found in this lesson".into());
        }
    }
    Ok(())
}
pub fn relink_course(conn: &Connection, id: i64, folder: &Path) -> Result<(), String> {
    let folder = folder.canonicalize().map_err(|e| e.to_string())?;
    if !folder.is_dir() {
        return Err("Choose a directory".into());
    }
    if conn
        .execute(
            "UPDATE courses SET path=?2 WHERE id=?1",
            params![id, path_string(&folder)?],
        )
        .map_err(|e| e.to_string())?
        == 0
    {
        return Err("Course not found".into());
    }
    let mut stmt = conn
        .prepare("SELECT id,path FROM lessons WHERE course_id=?1")
        .map_err(|e| e.to_string())?;
    for row in stmt
        .query_map([id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
    {
        let (lesson, path) = row.map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE lessons SET missing=?2 WHERE id=?1",
            params![lesson, approved_file(&folder, &folder.join(path)).is_err()],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn relink_lesson(conn: &Connection, id: i64, file: &Path) -> Result<(), String> {
    let root: String = conn
        .query_row(
            "SELECT c.path FROM lessons l JOIN courses c ON c.id=l.course_id WHERE l.id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let file = approved_file(Path::new(&root), file)?;
    if !file
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
    {
        return Err("Choose an MP4 file".into());
    }
    conn.execute(
        "UPDATE lessons SET path=?2,missing=0 WHERE id=?1",
        params![
            id,
            path_string(file.strip_prefix(&root).map_err(|e| e.to_string())?)?
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn jobs(conn: &Connection) -> Result<Vec<Job>, String> {
    let mut stmt = conn.prepare("SELECT j.id,j.lesson_id,l.title,j.status,j.stage,j.progress,j.error FROM jobs j JOIN lessons l ON l.id=j.lesson_id WHERE j.status IN ('queued','running') OR j.id IN (SELECT id FROM jobs ORDER BY id DESC LIMIT 200) ORDER BY j.id DESC").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Job {
                id: r.get(0)?,
                lesson_id: r.get(1)?,
                title: r.get(2)?,
                status: r.get(3)?,
                stage: r.get(4)?,
                progress: r.get(5)?,
                error: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}
pub fn enqueue(conn: &mut Connection, lessons: &[i64]) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for id in lessons {
        lesson_file(&tx, *id)?;
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE lesson_id=?1 AND status IN ('queued','running'))", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
        let has_caption: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM assets WHERE lesson_id=?1 AND kind='caption')",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !active && !has_caption {
            tx.execute(
                "INSERT INTO jobs(lesson_id,status,stage) VALUES(?1,'queued','queued')",
                [id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::scan_course;
    #[test]
    fn language_preferences_upgrade_and_persist() {
        let conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        conn.execute("INSERT INTO settings VALUES('preferences',?1)", [r#"{"libraryRoot":"/courses","modelPath":"","speed":1.5,"autoplay":false,"theme":"dark"}"#]).unwrap();
        let mut prefs = settings(&conn).unwrap();
        assert_eq!(prefs.language, "pt-BR");
        assert_eq!(initial_settings(&conn, "en").unwrap().language, "pt-BR");
        assert_eq!(prefs.speed, 1.5);
        for language in ["pt-BR", "en", "es"] {
            prefs.language = language.into();
            save_settings(&conn, &prefs).unwrap();
            assert_eq!(settings(&conn).unwrap().language, language);
        }
        prefs.language = "unknown".into();
        assert!(save_settings(&conn, &prefs).is_err());
        assert_eq!(settings(&conn).unwrap().language, "es");
    }
    #[test]
    fn first_run_language_is_persisted_without_overwriting_preferences() {
        for language in ["pt-BR", "es", "en"] {
            let conn = Connection::open_in_memory().unwrap();
            initialize(&conn).unwrap();
            let mut prefs = initial_settings(&conn, language).unwrap();
            assert_eq!(prefs.language, language);
            assert_eq!(settings(&conn).unwrap().language, language);
            assert_eq!(initial_settings(&conn, "en").unwrap().language, language);
            prefs.language = "es".into();
            prefs.speed = 1.5;
            save_settings(&conn, &prefs).unwrap();
            let saved = initial_settings(&conn, "pt-BR").unwrap();
            assert_eq!(saved.language, "es");
            assert_eq!(saved.speed, 1.5);
        }
        let conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        assert!(initial_settings(&conn, "unknown").is_err());
        assert_eq!(initial_settings(&conn, "en").unwrap().language, "en");
    }
    #[test]
    fn one_instance_owns_job_recovery_and_audio_cleanup() {
        let data = tempfile::tempdir().unwrap();
        let first = lock_instance(data.path()).unwrap();
        assert!(lock_instance(data.path()).is_err());
        drop(first);
        assert!(lock_instance(data.path()).is_ok());
    }
    #[test]
    fn rescan_and_relink_preserve_learning_data() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Course");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("1.mp4"), "video").unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        import(
            &mut conn,
            dir.path(),
            &[scan_course(dir.path(), &folder).unwrap()],
        )
        .unwrap();
        let course = library(&conn).unwrap().remove(0);
        let id = course.lessons[0].id;
        progress(&conn, id, 30.0, 100.0, Some(true)).unwrap();
        save_note(
            &conn,
            &Note {
                id: 0,
                lesson_id: id,
                position: 20.0,
                text: "Remember".into(),
                bookmark: false,
            },
        )
        .unwrap();
        import(
            &mut conn,
            dir.path(),
            &[scan_course(dir.path(), &folder).unwrap()],
        )
        .unwrap();
        assert_eq!(library(&conn).unwrap()[0].lessons[0].position, 30.0);
        assert!(library(&conn).unwrap()[0].lessons[0].completed);
        std::fs::rename(folder.join("1.mp4"), folder.join("renamed.mp4")).unwrap();
        relink_lesson(&conn, id, &folder.join("renamed.mp4")).unwrap();
        let moved = dir.path().join("Moved");
        std::fs::rename(folder, &moved).unwrap();
        relink_course(&conn, course.id, &moved).unwrap();
        assert!(lesson_file(&conn, id).is_ok());
        assert_eq!(notes(&conn, id).unwrap().len(), 1);
        let mut note = notes(&conn, id).unwrap().remove(0);
        note.id += 1000;
        assert!(save_note(&conn, &note).is_err());
        assert!(progress(&conn, id, f64::NAN, 100.0, None).is_err());
        std::fs::remove_file(moved.join("renamed.mp4")).unwrap();
        import(
            &mut conn,
            dir.path(),
            &[scan_course(dir.path(), &moved).unwrap()],
        )
        .unwrap();
        assert!(library(&conn).unwrap()[0].lessons[0].missing);
    }
    #[test]
    fn queue_deduplicates_and_recovery_is_retryable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("1.mp4"), "").unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        import(
            &mut conn,
            dir.path(),
            &[scan_course(dir.path(), dir.path()).unwrap()],
        )
        .unwrap();
        let id = library(&conn).unwrap()[0].lessons[0].id;
        enqueue(&mut conn, &[id, id]).unwrap();
        assert_eq!(jobs(&conn).unwrap().len(), 1);
        initialize(&conn).unwrap();
        assert_eq!(jobs(&conn).unwrap()[0].status, "interrupted");
        enqueue(&mut conn, &[id]).unwrap();
        assert_eq!(jobs(&conn).unwrap().len(), 2);
    }
    #[test]
    fn rescanning_removed_captions_allows_generation_and_overlap_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let course = dir.path().join("Course");
        std::fs::create_dir(&course).unwrap();
        std::fs::write(course.join("1.mp4"), "video").unwrap();
        std::fs::write(course.join("1.srt"), "caption").unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        import(
            &mut conn,
            dir.path(),
            &[scan_course(dir.path(), &course).unwrap()],
        )
        .unwrap();
        let id = library(&conn).unwrap()[0].lessons[0].id;
        enqueue(&mut conn, &[id]).unwrap();
        assert!(jobs(&conn).unwrap().is_empty());
        std::fs::remove_file(course.join("1.srt")).unwrap();
        import(
            &mut conn,
            dir.path(),
            &[scan_course(dir.path(), &course).unwrap()],
        )
        .unwrap();
        enqueue(&mut conn, &[id]).unwrap();
        assert_eq!(jobs(&conn).unwrap().len(), 1);
        assert!(import(
            &mut conn,
            dir.path(),
            &[
                scan_course(dir.path(), dir.path()).unwrap(),
                scan_course(dir.path(), &course).unwrap()
            ]
        )
        .is_err());
    }
}
