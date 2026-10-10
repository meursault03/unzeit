use crate::{
    db::{self, Db},
    models::*,
    resources, scanner, search, server, transcript,
    transcription::{self, Worker},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tauri::{Emitter, Manager, State};

pub struct AppState {
    _instance: fs::File,
    db: Db,
    data: PathBuf,
    media_base: String,
    worker: Arc<Worker>,
}
async fn database<T: Send + 'static>(
    state: &AppState,
    task: impl FnOnce(&mut rusqlite::Connection) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || task(&mut *db.lock().map_err(|e| e.to_string())?))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn get_library(state: State<'_, AppState>) -> Result<Vec<Course>, String> {
    database(&state, |conn| db::library(conn)).await
}
#[tauri::command]
async fn search_library(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<search::SearchResult>, String> {
    database(&state, move |conn| search::search(conn, &query)).await
}
#[tauri::command]
async fn get_caption_cues(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<crate::subtitles::Cue>, String> {
    let db = state.db.clone();
    let data = state.data.clone();
    tauri::async_runtime::spawn_blocking(move || transcript::index(&db, &data, id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    database(&state, |conn| db::settings(conn)).await
}
#[tauri::command]
async fn save_settings(state: State<'_, AppState>, mut settings: Settings) -> Result<(), String> {
    database(&state, move |conn| {
        let previous = db::settings(conn)?;
        settings.model_path = previous.model_path;
        if !settings.library_root.is_empty() && settings.library_root != previous.library_root {
            let root = Path::new(&settings.library_root)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !root.is_dir() {
                return Err("Library root must be a directory".into());
            }
            settings.library_root = scanner::path_string(&root)?;
        }
        db::save_settings(conn, &settings)
    })
    .await
}
#[tauri::command]
async fn scan_library(root: String) -> Result<Vec<Preview>, String> {
    tauri::async_runtime::spawn_blocking(move || scanner::scan_library(Path::new(&root)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn preview_course(root: String, path: String) -> Result<Preview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_course(Path::new(&root), Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn import_courses(
    state: State<'_, AppState>,
    root: String,
    paths: Vec<String>,
) -> Result<(), String> {
    if paths.is_empty() || paths.len() > 1000 {
        return Err("Choose between 1 and 1000 course folders".into());
    }
    let root = PathBuf::from(root);
    let previews = tauri::async_runtime::spawn_blocking({
        let root = root.clone();
        move || {
            paths
                .iter()
                .map(|p| scanner::scan_course(&root, Path::new(p)))
                .collect::<Result<Vec<_>, _>>()
        }
    })
    .await
    .map_err(|e| e.to_string())??;
    let ids = database(&state, move |conn| {
        db::import(conn, &root, &previews)?;
        previews
            .iter()
            .map(|p| {
                conn.query_row("SELECT id FROM courses WHERE path=?1", [&p.path], |r| {
                    r.get::<_, i64>(0)
                })
                .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;
    let db = state.db.clone();
    let data = state.data.clone();
    tauri::async_runtime::spawn_blocking(move || {
        for id in ids {
            for warning in transcript::reindex(&db, &data, Some(id))? {
                eprintln!("Caption index: {warning}");
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn rescan_course(state: State<'_, AppState>, id: i64) -> Result<Vec<String>, String> {
    let root = database(&state, move |conn| {
        conn.query_row("SELECT path FROM courses WHERE id=?1", [id], |r| {
            r.get::<_, String>(0)
        })
        .map_err(|e| e.to_string())
    })
    .await?;
    let root = PathBuf::from(root);
    let preview = tauri::async_runtime::spawn_blocking({
        let root = root.clone();
        move || scanner::scan_course(&root, &root)
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut warnings = preview.warnings.clone();
    database(&state, move |conn| {
        let settings = db::settings(conn)?;
        db::import(conn, &root, &[preview])?;
        db::save_settings(conn, &settings)
    })
    .await?;
    let db = state.db.clone();
    let data = state.data.clone();
    warnings.extend(
        tauri::async_runtime::spawn_blocking(move || transcript::reindex(&db, &data, Some(id)))
            .await
            .map_err(|e| e.to_string())??,
    );
    Ok(warnings)
}
#[tauri::command]
async fn save_progress(
    state: State<'_, AppState>,
    id: i64,
    position: f64,
    duration: f64,
    completed: Option<bool>,
) -> Result<(), String> {
    database(&state, move |conn| {
        db::progress(conn, id, position, duration, completed)
    })
    .await
}
#[tauri::command]
async fn get_notes(state: State<'_, AppState>, lesson_id: i64) -> Result<Vec<Note>, String> {
    database(&state, move |conn| db::notes(conn, lesson_id)).await
}
#[tauri::command]
async fn save_note(state: State<'_, AppState>, note: Note) -> Result<(), String> {
    database(&state, move |conn| db::save_note(conn, &note)).await
}
#[tauri::command]
async fn delete_note(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    database(&state, move |conn| {
        conn.execute("DELETE FROM notes WHERE id=?1", [id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
async fn relink_course(state: State<'_, AppState>, id: i64, path: String) -> Result<(), String> {
    database(&state, move |conn| {
        db::relink_course(conn, id, Path::new(&path))
    })
    .await
}
#[tauri::command]
async fn relink_lesson(state: State<'_, AppState>, id: i64, path: String) -> Result<(), String> {
    database(&state, move |conn| {
        db::relink_lesson(conn, id, Path::new(&path))
    })
    .await
}
#[tauri::command]
fn media_base(state: State<'_, AppState>) -> String {
    state.media_base.clone()
}
#[tauri::command]
async fn get_resource(state: State<'_, AppState>, id: i64) -> Result<resources::Resource, String> {
    let data = state.data.clone();
    let file = database(&state, move |conn| resources::file(conn, id, &data)).await?;
    tauri::async_runtime::spawn_blocking(move || resources::info(&file))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn read_resource_text(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let data = state.data.clone();
    let file = database(&state, move |conn| resources::file(conn, id, &data)).await?;
    tauri::async_runtime::spawn_blocking(move || resources::text(&file))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn open_resource(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let data = state.data.clone();
    let file = database(&state, move |conn| {
        let (file, kind) = db::asset_file(conn, id, &data)?;
        if kind != "resource" {
            return Err("Not a resource".into());
        }
        Ok(file)
    })
    .await?;
    tauri::async_runtime::spawn_blocking(move || {
        let ext = file
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        // Code and active document formats are revealed in a folder, never executed.
        let target = if [
            "pdf", "txt", "md", "png", "jpg", "jpeg", "webp", "gif", "zip", "gz", "tar", "7z",
            "csv",
        ]
        .contains(&ext.as_str())
        {
            file.as_path()
        } else {
            file.parent().ok_or("No parent directory")?
        };
        let status = Command::new("xdg-open")
            .arg(target)
            .status()
            .map_err(|e| format!("Cannot open resource: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("The system application could not open this resource".into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn get_jobs(state: State<'_, AppState>) -> Result<Vec<Job>, String> {
    database(&state, |conn| db::jobs(conn)).await
}
#[tauri::command]
async fn enqueue_transcription(
    state: State<'_, AppState>,
    lesson_ids: Vec<i64>,
) -> Result<(), String> {
    if lesson_ids.is_empty() || lesson_ids.len() > 1000 {
        return Err("Choose between 1 and 1000 lessons".into());
    }
    database(&state, move |conn| {
        if db::settings(conn)?.model_path.is_empty() {
            return Err("Import a medium model in Settings first".into());
        }
        db::enqueue(conn, &lesson_ids)
    })
    .await
}
#[tauri::command]
fn cancel_transcription(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.worker.cancel(id)
}
#[tauri::command]
async fn import_model(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let db = state.db.clone();
    let data = state.data.clone();
    tauri::async_runtime::spawn_blocking(move || {
        {
            let conn = db.lock().map_err(|e| e.to_string())?;
            if db::jobs(&conn)?
                .iter()
                .any(|j| ["queued", "running"].contains(&j.status.as_str()))
            {
                return Err("Finish or cancel transcription jobs before changing the model".into());
            }
        }
        let source = PathBuf::from(path)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let directory = data.join("models");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let destination = directory.join(format!("medium-{}.bin", uuid::Uuid::new_v4()));
        fs::copy(source, &destination).map_err(|e| e.to_string())?;
        if let Err(e) = transcription::load_medium(&destination) {
            let _ = fs::remove_file(&destination);
            return Err(e);
        }
        let path = scanner::path_string(&destination)?;
        let conn = db.lock().map_err(|e| e.to_string())?;
        if db::jobs(&conn)?
            .iter()
            .any(|j| ["queued", "running"].contains(&j.status.as_str()))
        {
            let _ = fs::remove_file(&destination);
            return Err("Finish or cancel transcription jobs before changing the model".into());
        }
        let mut settings = db::settings(&conn)?;
        let previous = settings.model_path.clone();
        settings.model_path = path.clone();
        if let Err(e) = db::save_settings(&conn, &settings) {
            let _ = fs::remove_file(&destination);
            return Err(e);
        }
        if !previous.is_empty() && Path::new(&previous).starts_with(&directory) {
            let _ = fs::remove_file(previous);
        }
        Ok(path)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn dependencies() -> Result<Vec<(String, bool)>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        ["ffmpeg", "ffprobe", "xdg-open"]
            .into_iter()
            .map(|name| {
                (
                    name.to_string(),
                    Command::new(name)
                        .arg(if name == "xdg-open" {
                            "--version"
                        } else {
                            "-version"
                        })
                        .output()
                        .is_ok_and(|o| o.status.success()),
                )
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .on_page_load(|_window, _payload| {
            #[cfg(feature = "webview-smoke")]
            if _payload.event() == tauri::webview::PageLoadEvent::Finished {
                crate::smoke::inject(_window);
            }
        })
        .setup(|app| {
            #[cfg(not(feature = "webview-smoke"))]
            let data = app.path().app_data_dir()?;
            #[cfg(feature = "webview-smoke")]
            let data = crate::smoke::install(app)?;
            fs::create_dir_all(&data)?;
            let instance = db::lock_instance(&data).map_err(std::io::Error::other)?;
            let audio = data.join("audio");
            if audio.is_dir() {
                for entry in fs::read_dir(&audio)?.flatten() {
                    if entry.path().extension().is_some_and(|e| e == "wav") {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
            let db = db::open(&data.join("unzeit.sqlite")).map_err(std::io::Error::other)?;
            let token = uuid::Uuid::new_v4().to_string();
            let listener =
                tauri::async_runtime::block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))?;
            let media_base = format!(
                "http://127.0.0.1:{}/{}",
                listener.local_addr()?.port(),
                token
            );
            let routes = server::router(server::MediaState {
                db: db.clone(),
                data: data.clone(),
                token,
            });
            tauri::async_runtime::spawn(async move {
                if let Err(e) = axum::serve(listener, routes).await {
                    eprintln!("Media server failed: {e}");
                }
            });
            let handle = app.handle().clone();
            let worker = Worker::start(
                db.clone(),
                data.clone(),
                Arc::new(move |job| {
                    let _ = handle.emit("transcription-progress", job);
                }),
            );
            let index_db = db.clone();
            let index_data = data.clone();
            let index_handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = transcript::reindex(&index_db, &index_data, None) {
                    eprintln!("Caption index failed: {error}");
                }
                let _ = index_handle.emit("captions-indexed", ());
            });
            app.manage(AppState {
                _instance: instance,
                db,
                data,
                media_base,
                worker,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library,
            get_caption_cues,
            search_library,
            get_settings,
            save_settings,
            scan_library,
            preview_course,
            import_courses,
            rescan_course,
            save_progress,
            get_notes,
            save_note,
            delete_note,
            relink_course,
            relink_lesson,
            media_base,
            open_resource,
            get_resource,
            read_resource_text,
            get_jobs,
            enqueue_transcription,
            cancel_transcription,
            import_model,
            dependencies
        ])
        .build(tauri::generate_context!())
        .expect("Failed to initialize Unzeit")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                app.state::<AppState>().worker.stop();
            }
        });
}
