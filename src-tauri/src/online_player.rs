use gtk::prelude::*;
use serde::Deserialize;
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};
use tauri::{Emitter, Manager, WebviewWindow};
use webkit2gtk::{
    DownloadExt, NavigationPolicyDecisionExt, PolicyDecisionExt, SettingsExt, URIRequestExt,
    WebContextExt, WebViewExt,
};

const NAME: &str = "unzeit-online-player";

thread_local! {
    static PLAYER_BOUNDS: Cell<Option<Bounds>> = const { Cell::new(None) };
    static HIDDEN_PAUSE: RefCell<Option<gtk::glib::SourceId>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Deserialize)]
pub struct Bounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn player_overlay() -> gtk::Overlay {
    let overlay = gtk::Overlay::new();
    // A size request is a minimum. Allocate the exact slot when the window shrinks.
    gtk::OverlaySignals::connect_get_child_position(&overlay, |_, child| {
        if child.widget_name() != NAME {
            return None;
        }
        PLAYER_BOUNDS
            .get()
            .map(|bounds| gtk::Rectangle::new(bounds.x, bounds.y, bounds.width, bounds.height))
    });
    overlay
}

fn place(overlay: &gtk::Overlay, view: &webkit2gtk::WebView, bounds: Bounds, zoom: f64) {
    // The slot uses CSS pixels and GTK allocates the separate player in logical pixels.
    let scaled = |value| (f64::from(value) * zoom).round() as i32;
    PLAYER_BOUNDS.set(Some(Bounds {
        x: scaled(bounds.x),
        y: scaled(bounds.y),
        width: scaled(bounds.width),
        height: scaled(bounds.height),
    }));
    if view.zoom_level() != zoom {
        view.set_zoom_level(zoom);
    }
    cancel_hidden_pause();
    view.show();
    overlay.queue_allocate();
}

fn cancel_hidden_pause() {
    if let Some(timer) = HIDDEN_PAUSE.take() {
        timer.remove();
    }
}

fn hide(view: &webkit2gtk::WebView) {
    if !view.is_visible() {
        return;
    }
    view.hide();
    let view = view.downgrade();
    // A brief invalid layout during a monitor switch must not interrupt playback.
    let timer = gtk::glib::timeout_add_local_once(Duration::from_millis(500), move || {
        HIDDEN_PAUSE.take();
        if let Some(view) = view.upgrade() {
            view.evaluate_javascript(
                "document.querySelectorAll('video, audio').forEach(media => media.pause());",
                Some("unzeit-media"),
                None,
                None::<&gio::Cancellable>,
                |_| {},
            );
        }
    });
    HIDDEN_PAUSE.replace(Some(timer));
}

pub enum Action {
    Open {
        url: url::Url,
        session: u32,
        bounds: Bounds,
    },
    Place(Bounds),
    Hide,
    Close,
}

impl Action {
    fn validate(&self) -> Result<(), String> {
        let bounds = match self {
            Self::Open { bounds, .. } | Self::Place(bounds) => bounds,
            Self::Hide | Self::Close => return Ok(()),
        };
        if ![bounds.x, bounds.y]
            .into_iter()
            .all(|v| (0..=16_384).contains(&v))
            || ![bounds.width, bounds.height]
                .into_iter()
                .all(|v| (200..=16_384).contains(&v))
        {
            return Err("The embedded video area is unavailable".into());
        }
        Ok(())
    }
}

pub async fn update(window: WebviewWindow, action: Action) -> Result<(), String> {
    if window.label() != "main" {
        return Err("The embedded player is only available in the main window".into());
    }
    action.validate()?;
    let handle = window.clone();
    let (send, receive) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |native| {
            let result = apply(&handle, &native.inner(), action);
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
    receive
        .await
        .map_err(|_| "Unable to update the embedded video".to_owned())?
}

fn apply(window: &WebviewWindow, main: &webkit2gtk::WebView, action: Action) -> Result<(), String> {
    let parent = main
        .parent()
        .ok_or("The embedded video area is unavailable")?;
    let overlay = if let Some(overlay) = parent.downcast_ref::<gtk::Overlay>() {
        overlay.clone()
    } else if matches!(action, Action::Open { .. }) {
        let container = parent
            .downcast::<gtk::Box>()
            .map_err(|_| "The embedded video area is unavailable")?;
        let overlay = player_overlay();
        container.remove(main);
        overlay.add(main);
        container.pack_start(&overlay, true, true, 0);
        overlay.show();
        overlay
    } else {
        return Ok(());
    };
    let existing = overlay
        .children()
        .into_iter()
        .find(|child| child.widget_name() == NAME);
    if matches!(action, Action::Place(_) | Action::Hide) {
        if let Some(view) = existing.and_then(|child| child.downcast::<webkit2gtk::WebView>().ok())
        {
            if let Action::Place(bounds) = action {
                place(&overlay, &view, bounds, main.zoom_level());
            } else {
                hide(&view);
            }
        }
        return Ok(());
    }
    if let Some(child) = existing {
        cancel_hidden_pause();
        if let Some(view) = child.downcast_ref::<webkit2gtk::WebView>() {
            view.stop_loading();
        }
        overlay.remove(&child);
        PLAYER_BOUNDS.set(None);
    }
    let Action::Open {
        url,
        session,
        bounds,
    } = action
    else {
        return Ok(());
    };

    // An independent WebKit context has no Tauri IPC handlers, protocols, or app storage.
    let context = webkit2gtk::WebContext::new_ephemeral();
    context.connect_download_started(|_, download| download.cancel());
    let view = webkit2gtk::WebView::with_context(&context);
    if let Some(settings) = WebViewExt::settings(&view) {
        settings.set_allow_file_access_from_file_urls(false);
        settings.set_allow_universal_access_from_file_urls(false);
        settings.set_enable_developer_extras(false);
        settings.set_enable_fullscreen(false);
        settings.set_media_playback_requires_user_gesture(true);
    }
    view.connect_permission_request(|_, request| {
        use webkit2gtk::PermissionRequestExt;
        request.deny();
        true
    });
    let initial_path = url.path().to_owned();
    view.connect_decide_policy(move |_, decision, kind| {
        use webkit2gtk::PolicyDecisionType::{NavigationAction, NewWindowAction};
        if !matches!(kind, NavigationAction | NewWindowAction) {
            return false;
        }
        let action = decision
            .downcast_ref::<webkit2gtk::NavigationPolicyDecision>()
            .and_then(NavigationPolicyDecisionExt::navigation_action);
        if let Some((action, target)) = action.and_then(|a| {
            let target = url::Url::parse(&a.request()?.uri()?).ok()?;
            Some((a, target))
        }) {
            if target.scheme() == "https"
                && target.host_str() == Some("www.youtube.com")
                && target.port().is_none()
                && target.username().is_empty()
                && target.password().is_none()
            {
                if kind == NavigationAction && target.path() == initial_path {
                    return false;
                }
                if action.is_user_gesture() && matches!(target.path(), "/watch" | "/") {
                    let _ = gio::AppInfo::launch_default_for_uri(
                        target.as_str(),
                        None::<&gio::AppLaunchContext>,
                    );
                }
            }
        }
        decision.ignore();
        true
    });
    let failed = window.clone();
    view.connect_load_failed(move |_, _, _, error| {
        if !error.matches(webkit2gtk::NetworkError::Cancelled) {
            let _ = failed.emit("online-video-failed", session);
        }
        true
    });
    let loaded = window.clone();
    view.connect_load_changed(move |view, event| {
        use webkit2gtk::{URIResponseExt, WebResourceExt};
        if event != webkit2gtk::LoadEvent::Finished {
            return;
        }
        if let Some(response) = view
            .main_resource()
            .and_then(|resource| resource.response())
        {
            let name = if response.status_code() >= 400 {
                "online-video-failed"
            } else {
                "online-video-loaded"
            };
            let _ = loaded.emit(name, session);
        }
    });
    let terminated = window.clone();
    view.connect_web_process_terminated(move |_, _| {
        let _ = terminated.emit("online-video-failed", session);
    });
    view.set_widget_name(NAME);
    place(&overlay, &view, bounds, main.zoom_level());
    let request = webkit2gtk::URIRequest::new(url.as_str());
    let headers = request
        .http_headers()
        .ok_or("Unable to identify Unzeit to YouTube")?;
    headers.replace(
        "Referer",
        &format!("https://{}/", window.config().identifier),
    );
    overlay.add_overlay(&view);
    view.load_request(&request);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GTK display"]
    fn embedded_player_shrinks_after_window_resize_at_125_percent() {
        gtk::init().unwrap();
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        let overlay = player_overlay();
        let view = webkit2gtk::WebView::with_context(&webkit2gtk::WebContext::new_ephemeral());
        view.set_widget_name(NAME);
        overlay.add(&gtk::DrawingArea::new());
        overlay.add_overlay(&view);
        window.add(&overlay);
        window.show_all();
        let page = view.page_id();
        for (viewport, width, height) in [(2560, 744, 418), (1902, 674, 379), (2560, 744, 418)] {
            hide(&view);
            assert!(!view.is_visible());
            place(
                &overlay,
                &view,
                Bounds {
                    x: 240,
                    y: 270,
                    width,
                    height,
                },
                1.25,
            );
            assert!(view.is_visible());
            assert_eq!(view.page_id(), page);
            assert!(HIDDEN_PAUSE.with(|timer| timer.borrow().is_none()));
            overlay.size_allocate(&gtk::Allocation::new(0, 0, viewport, 1051));
            let actual = view.allocation();
            assert_eq!(
                (actual.width(), actual.height()),
                (
                    (f64::from(width) * 1.25).round() as i32,
                    (f64::from(height) * 1.25).round() as i32
                )
            );
        }
    }
}
