//! Opt-in native WebKit integration test support; excluded from normal builds.
use std::{fs, path::PathBuf};
use tauri::{Listener, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};

pub fn install(app: &mut tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
            "compact",
            "search",
            "resource",
            "study",
            "localized",
            "code",
            "layout",
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
pub fn inject(window: &tauri::Webview<tauri::Wry>) {
    let Ok(directory) = std::env::var("UNZEIT_TEST_DIR") else {
        return;
    };
    let root = PathBuf::from(directory).join("courses");
    let script = format!(
        "globalThis.__UNZEIT_TEST_ROOT__ = {}; globalThis.__UNZEIT_CLOSE_ONLY__ = {}; {}",
        serde_json::to_string(&root).unwrap(),
        std::env::var_os("UNZEIT_TEST_CLOSE_ONLY").is_some(),
        include_str!("../../tests/webview-smoke.js")
    );
    if let Err(error) = window.eval(script) {
        eprintln!("Smoke script failed: {error}");
    }
}
