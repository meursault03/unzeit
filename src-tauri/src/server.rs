use crate::{
    db::{self, Db},
    resources, subtitles,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::{cors::CorsLayer, services::ServeFile};

#[derive(Clone)]
pub struct MediaState {
    pub db: Db,
    pub data: PathBuf,
    pub token: String,
}

pub fn router(state: MediaState) -> Router {
    Router::new()
        .route("/{token}/media/{id}", get(media))
        .route("/{token}/caption/{id}", get(caption))
        .route("/{token}/resource/{id}", get(resource))
        .layer(
            CorsLayer::new()
                .allow_origin([
                    HeaderValue::from_static("tauri://localhost"),
                    HeaderValue::from_static("http://tauri.localhost"),
                    HeaderValue::from_static("https://tauri.localhost"),
                    HeaderValue::from_static("http://127.0.0.1:1420"),
                ])
                .allow_methods([Method::GET, Method::HEAD])
                .allow_headers([header::RANGE])
                .expose_headers([
                    header::CONTENT_RANGE,
                    header::ACCEPT_RANGES,
                    header::CONTENT_LENGTH,
                ]),
        )
        .with_state(state)
}

async fn media(
    State(state): State<MediaState>,
    Path((token, id)): Path<(String, i64)>,
    request: Request<Body>,
) -> Response {
    if token != state.token {
        return StatusCode::FORBIDDEN.into_response();
    }
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        db::lesson_file(&*db.lock().map_err(|e| e.to_string())?, id)
    })
    .await;
    match result {
        Ok(Ok((_, _, file))) => match ServeFile::new(file).oneshot(request).await {
            Ok(response) => response.map(Body::new),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn resource(
    State(state): State<MediaState>,
    Path((token, id)): Path<(String, i64)>,
    request: Request<Body>,
) -> Response {
    if token != state.token {
        return StatusCode::FORBIDDEN.into_response();
    }
    let result = tokio::task::spawn_blocking(move || {
        resources::file(
            &*state.db.lock().map_err(|e| e.to_string())?,
            id,
            &state.data,
        )
    })
    .await;
    match result {
        Ok(Ok(file)) => {
            let (preview, mime) = resources::format(&file);
            if !["pdf", "image"].contains(&preview) {
                return StatusCode::FORBIDDEN.into_response();
            }
            match ServeFile::new(file).oneshot(request).await {
                Ok(response) => {
                    let mut response = response.map(Body::new);
                    response
                        .headers_mut()
                        .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
                    response.headers_mut().insert(
                        header::X_CONTENT_TYPE_OPTIONS,
                        HeaderValue::from_static("nosniff"),
                    );
                    response
                        .headers_mut()
                        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                    response
                }
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn caption(
    State(state): State<MediaState>,
    Path((token, id)): Path<(String, i64)>,
    request: Request<Body>,
) -> Response {
    if token != state.token {
        return StatusCode::FORBIDDEN.into_response();
    }
    let result = tokio::task::spawn_blocking(move || {
        let (file, kind) = db::asset_file(
            &*state.db.lock().map_err(|e| e.to_string())?,
            id,
            &state.data,
        )?;
        if kind != "caption" {
            return Err("Not a caption".into());
        }
        if std::fs::metadata(&file).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
            return Err("Caption is larger than 8 MiB".into());
        }
        subtitles::to_vtt(&std::fs::read_to_string(file).map_err(|e| e.to_string())?)
    })
    .await;
    match result {
        Ok(Ok(text)) => (
            [
                (header::CONTENT_TYPE, "text/vtt; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            if request.method() == Method::HEAD {
                String::new()
            } else {
                text
            },
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    #[tokio::test]
    async fn resources_require_indexed_ids_and_never_serve_active_documents() {
        let dir = tempfile::tempdir().unwrap();
        for (name, data) in [
            ("1.mp4", "video"),
            ("guide.pdf", "%PDF-1.7 test"),
            ("page.html", "<script>alert(1)</script>"),
        ] {
            std::fs::write(dir.path().join(name), data).unwrap();
        }
        let db = db::open(&dir.path().join("db.sqlite")).unwrap();
        db::import(
            &mut db.lock().unwrap(),
            dir.path(),
            &[crate::scanner::scan_course(dir.path(), dir.path()).unwrap()],
        )
        .unwrap();
        let library = db::library(&db.lock().unwrap()).unwrap();
        let pdf = library[0]
            .resources
            .iter()
            .find(|r| r.title == "guide.pdf")
            .unwrap()
            .id;
        let html = library[0]
            .resources
            .iter()
            .find(|r| r.title == "page.html")
            .unwrap()
            .id;
        let app = router(MediaState {
            db: db.clone(),
            data: dir.path().into(),
            token: "test".into(),
        });
        for (token, id, status) in [
            ("test", pdf, 206),
            ("wrong", pdf, 403),
            ("test", html, 403),
            ("test", 99999, 404),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/{token}/resource/{id}"))
                        .header(header::RANGE, "bytes=0-3")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status);
            if status == 206 {
                assert_eq!(response.headers()[header::CONTENT_TYPE], "application/pdf");
                assert_eq!(
                    response.headers()[header::X_CONTENT_TYPE_OPTIONS],
                    "nosniff"
                );
                assert_eq!(
                    response.into_body().collect().await.unwrap().to_bytes(),
                    "%PDF"
                );
            }
        }
        db.lock()
            .unwrap()
            .execute("UPDATE assets SET path='../escape.pdf' WHERE id=?1", [pdf])
            .unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/test/resource/{pdf}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    #[tokio::test]
    async fn streams_ranges_head_and_checks_authorization() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("1.mp4"), b"0123456789").unwrap();
        let db = db::open(&dir.path().join("db.sqlite")).unwrap();
        db::import(
            &mut db.lock().unwrap(),
            dir.path(),
            &[crate::scanner::scan_course(dir.path(), dir.path()).unwrap()],
        )
        .unwrap();
        let id = db::library(&db.lock().unwrap()).unwrap()[0].lessons[0].id;
        let app = router(MediaState {
            db: db.clone(),
            data: dir.path().into(),
            token: "test".into(),
        });
        for (range, status, expected) in [
            (None, 200, "0123456789"),
            (Some("bytes=2-4"), 206, "234"),
            (Some("bytes=8-"), 206, "89"),
            (Some("bytes=-3"), 206, "789"),
            (Some("bytes=100-"), 416, ""),
        ] {
            let mut request = Request::builder().uri(format!("/test/media/{id}"));
            if let Some(range) = range {
                request = request.header(header::RANGE, range);
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status);
            assert_eq!(
                response.into_body().collect().await.unwrap().to_bytes(),
                expected
            );
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("HEAD")
                    .uri(format!("/test/media/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()[header::CONTENT_LENGTH], "10");
        assert!(response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/wrong/media/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        db.lock()
            .unwrap()
            .execute("UPDATE lessons SET path='../outside.mp4'", [])
            .unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/test/media/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
