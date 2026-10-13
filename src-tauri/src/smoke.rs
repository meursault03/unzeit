//! Opt-in native WebKit integration test support; excluded from normal builds.
use std::{fs, io::Write, path::PathBuf};
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};

pub fn install(app: &mut tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    app.manage(Rules::default());
    let rule_handle = app.handle().clone();
    app.listen("unzeit-test-rule", move |event| {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            if let Some(command) = value["command"].as_str() {
                let rules = rule_handle.state::<Rules>();
                let mut rules = rules.0.lock().unwrap();
                if value["rule"].is_null() {
                    rules.remove(command);
                } else {
                    rules.insert(command.into(), value["rule"].clone());
                }
            }
        }
    });
    let directory = PathBuf::from(std::env::var("UNZEIT_TEST_DIR")?);
    fs::create_dir_all(&directory)?;
    let report = directory.join("webview-result.json");
    let handle = app.handle().clone();
    let snapshots = directory.clone();
    let capture_handle = handle.clone();
    let resize_handle = handle.clone();
    if let Some(window) = app.get_webview_window("main") {
        window.set_title("Unzeit · Interface test")?;
    }
    let clipboard_handle = handle.clone();
    app.listen("unzeit-test-clipboard", move |_| {
        let content = clipboard_handle.clipboard().read_text().unwrap_or_default();
        if let Some(window) = clipboard_handle.get_webview_window("main") {
            let _ = window.eval(format!(
                "globalThis.__UNZEIT_COPIED_TEXT__ = {}",
                serde_json::to_string(&content).unwrap()
            ));
        }
    });
    app.listen("unzeit-test-compact", move |_| {
        if let Some(window) = resize_handle.get_webview_window("main") {
            let _ = window.set_fullscreen(false);
            let _ = window.unmaximize();
            let _ = window.set_size(tauri::LogicalSize::new(720.0, 560.0));
        }
    });
    app.listen("unzeit-test-snapshot", move |event| {
        let label: String = serde_json::from_str(event.payload()).unwrap_or_default();
        if ![
            "empty",
            "library",
            "player",
            "settings",
            "select-dark",
            "compact",
            "search",
            "resource",
            "study",
            "localized",
            "localized-ru",
            "localized-zh-CN",
            "localized-fr",
            "code",
            "layout",
            "ux",
        ]
        .contains(&label.as_str())
        {
            return;
        }
        let Some(window) = capture_handle.get_webview_window("main") else {
            return;
        };
        let complete = window.clone();
        let path = snapshots.join(format!("{label}.png"));
        let _ = window.with_webview(move |webview| {
            webview.inner().snapshot(
                SnapshotRegion::FullDocument,
                SnapshotOptions::NONE,
                None::<&webkit2gtk::gio::Cancellable>,
                move |surface| {
                    let result = surface.map_err(|e| e.to_string()).and_then(|surface| {
                        let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
                        surface.write_to_png(&mut file).map_err(|e| e.to_string())
                    });
                    let value = result
                        .err()
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Bool(true));
                    let _ = complete.eval(format!("globalThis.__UNZEIT_SNAPSHOT_DONE__ = {value}"));
                },
            );
        });
    });
    let picker_request = directory.join("picker-request.json");
    app.listen("unzeit-test-picker", move |event| {
        let _ = fs::write(&picker_request, event.payload());
    });
    app.listen("unzeit-test-result", move |event| {
        let _ = fs::write(&report, event.payload());
        let success = serde_json::from_str::<serde_json::Value>(event.payload())
            .ok()
            .and_then(|v| v["ok"].as_bool())
            .unwrap_or(false);
        if !success {
            handle.exit(1);
        }
    });
    Ok(directory.join("app-data"))
}
#[derive(Default)]
struct Rules(std::sync::Mutex<std::collections::HashMap<String, serde_json::Value>>);
pub fn rule(request: &tauri::ipc::Invoke) -> Option<serde_json::Value> {
    let command = request.message.command();
    if command == "save_progress" {
        record_shutdown("save-progress:begin");
    }
    let handle = request.message.webview().app_handle().clone();
    let _ = handle.emit("unzeit-test-call", command);
    let selected = handle
        .state::<Rules>()
        .0
        .lock()
        .unwrap()
        .get(command)
        .cloned();
    selected
}
pub fn dispatch(
    request: tauri::ipc::Invoke,
    rule: serde_json::Value,
    handler: fn(tauri::ipc::Invoke) -> bool,
) {
    tauri::async_runtime::spawn(async move {
        if let Some(delay) = rule["delayMs"].as_u64() {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }
        if let Some(error) = rule["error"].as_str() {
            request.resolver.reject(error);
        } else if let Some(response) = rule.get("response") {
            request.resolver.resolve(response.clone());
        } else {
            handler(request);
        }
    });
}
pub fn record_shutdown(phase: &str) {
    let Ok(directory) = std::env::var("UNZEIT_TEST_DIR") else {
        return;
    };
    if let Ok(mut log) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(PathBuf::from(directory).join("shutdown.jsonl"))
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let _ = writeln!(
            log,
            "{}",
            serde_json::json!({"phase": phase, "timestamp": timestamp})
        );
    }
}
pub fn inject(window: &tauri::Webview<tauri::Wry>) {
    let Ok(directory) = std::env::var("UNZEIT_TEST_DIR") else {
        return;
    };
    let root = PathBuf::from(directory).join("courses");
    let script = format!(
        "globalThis.__UNZEIT_TEST_ROOT__ = {}; globalThis.__UNZEIT_CLOSE_ONLY__ = {}; globalThis.__UNZEIT_PLAYING_CLOSE__ = {}; globalThis.__UNZEIT_TEST_PICKER_MOCK__ = {}; globalThis.__UNZEIT_TEST_MODEL__ = {}; globalThis.__UNZEIT_SELECT_CLOSE__ = {}; {}",
        serde_json::to_string(&root).unwrap(),
        std::env::var_os("UNZEIT_TEST_CLOSE_ONLY").is_some(),
        std::env::var_os("UNZEIT_TEST_PLAYING_CLOSE").is_some(),
        std::env::var_os("UNZEIT_TEST_PICKER_MOCK").is_some(),
        serde_json::to_string(&std::env::var("UNZEIT_TEST_MODEL").ok()).unwrap(),
        std::env::var_os("UNZEIT_TEST_SELECT_CLOSE").is_some(),
        if std::env::var_os("UNZEIT_TEST_UX_ONLY").is_some() {
            include_str!("../../tests/ux-smoke.js")
        } else {
            include_str!("../../tests/webview-smoke.js")
        }
    );
    if let Err(error) = window.eval(script) {
        eprintln!("Smoke script failed: {error}");
    }
}
