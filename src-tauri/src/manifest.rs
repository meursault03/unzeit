use crate::{
    models::{OnlineVideo, Preview, PreviewAsset, PreviewLesson, PreviewSection},
    scanner::{approved_file, path_string},
};
use serde::Deserialize;
use std::{collections::HashSet, fs, io::Read, path::Path};

pub const FILE_NAME: &str = "course.json";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    version: u32,
    title: String,
    author: Option<String>,
    workbook: Option<Workbook>,
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Workbook {
    title: String,
    path: String,
}

#[derive(Deserialize)]
struct Section {
    title: String,
    lessons: Vec<LinkedLesson>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkedLesson {
    id: String,
    title: String,
    url: String,
    pdf_page: Option<u32>,
    #[serde(default)]
    videos: Vec<OnlineVideo>,
}

pub fn youtube_id(value: &str) -> bool {
    value.len() == 11
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn https_url(value: &str) -> Result<url::Url, String> {
    if value.len() <= 2048 && !value.chars().any(char::is_control) {
        if let Ok(url) = url::Url::parse(value) {
            if url.scheme() == "https"
                && url.host().is_some()
                && url.username().is_empty()
                && url.password().is_none()
            {
                return Ok(url);
            }
        }
    }
    Err("The lesson link must be an HTTPS URL without credentials".into())
}

fn label(value: String) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 300 || value.chars().any(char::is_control) {
        return Err("Invalid course manifest title or author".into());
    }
    Ok(value.to_owned())
}

pub fn read(folder: &Path) -> Result<Preview, String> {
    let path = folder.join(FILE_NAME);
    if !fs::symlink_metadata(&path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("Choose a regular course.json file".into());
    }
    let path = approved_file(folder, &path)?;
    const LIMIT: u64 = 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(LIMIT + 1).read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("The course manifest exceeds 1 MiB".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid course manifest: {e}"))?;
    if manifest.version != 1 || manifest.sections.is_empty() {
        return Err("Use a version 1 course manifest with at least one section".into());
    }
    let mut preview = Preview {
        path: path_string(folder)?,
        title: label(manifest.title)?,
        author: manifest.author.map(label).transpose()?,
        sections: Vec::new(),
        warnings: Vec::new(),
        resources: Vec::new(),
    };
    if let Some(workbook) = manifest.workbook {
        let relative = Path::new(&workbook.path);
        if !relative
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
            || !relative
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            return Err("The workbook must be a PDF inside the course folder".into());
        }
        approved_file(folder, &folder.join(relative))?;
        preview.resources.push(PreviewAsset {
            path: workbook.path,
            title: label(workbook.title)?,
            kind: "resource".into(),
        });
    }
    let mut ids = HashSet::new();
    let mut sections = HashSet::new();
    for (index, section) in manifest.sections.into_iter().enumerate() {
        let title = label(section.title)?;
        if section.lessons.is_empty() || !sections.insert(title.clone()) {
            return Err("Course sections must have distinct titles and at least one lesson".into());
        }
        let mut lessons = Vec::new();
        for lesson in section.lessons {
            if lesson.id.is_empty()
                || lesson.id.len() > 100
                || !lesson
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !ids.insert(lesson.id.clone())
            {
                return Err("Course lesson IDs must be distinct and use letters, numbers, hyphens or underscores".into());
            }
            if lesson.pdf_page == Some(0)
                || (lesson.pdf_page.is_some() && preview.resources.is_empty())
            {
                return Err("A lesson page requires a workbook and a positive page number".into());
            }
            let mut video_ids = HashSet::new();
            if lesson.videos.len() > 50
                || lesson
                    .videos
                    .iter()
                    .any(|v| !youtube_id(&v.youtube_id) || !video_ids.insert(&v.youtube_id))
            {
                return Err("Use up to 50 distinct YouTube video IDs per lesson".into());
            }
            let videos = lesson
                .videos
                .into_iter()
                .map(|mut video| {
                    video.title = video.title.map(label).transpose()?;
                    Ok(video)
                })
                .collect::<Result<Vec<_>, String>>()?;
            lessons.push(PreviewLesson {
                path: format!("{FILE_NAME}/{}", lesson.id),
                title: label(lesson.title)?,
                source_url: Some(https_url(&lesson.url)?.to_string()),
                pdf_page: lesson.pdf_page,
                videos,
                captions: Vec::new(),
                resources: Vec::new(),
            });
        }
        preview.sections.push(PreviewSection {
            title,
            path: index.to_string(),
            lessons,
        });
    }
    Ok(preview)
}
