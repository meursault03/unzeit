use crate::models::{Preview, PreviewAsset, PreviewLesson, PreviewSection};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn path_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "A filename contains invalid UTF-8; rename it before importing".into())
}

pub fn approved_file(root: &Path, file: &Path) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let file = file.canonicalize().map_err(|e| e.to_string())?;
    if !file.starts_with(&root) || !file.is_file() {
        return Err("File is outside the approved course folder".into());
    }
    Ok(file)
}

pub fn clean_title(name: &str) -> String {
    let title = name.trim_start_matches(|c: char| c.is_ascii_digit() || " .-_".contains(c));
    if title.is_empty() {
        name.to_owned()
    } else {
        title.replace('_', " ")
    }
}

pub fn scan_library(root: &Path) -> Result<Vec<Preview>, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("Cannot open library: {e}"))?;
    let mut courses = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir()
            && !entry.file_name().to_string_lossy().starts_with('.')
        {
            let course = scan_course(&root, &entry.path())?;
            if !course.sections.is_empty() || !course.warnings.is_empty() {
                courses.push(course);
            }
        }
    }
    courses.sort_by(|a, b| natord::compare(&a.title, &b.title));
    Ok(courses)
}

pub fn scan_course(library: &Path, folder: &Path) -> Result<Preview, String> {
    let library = library.canonicalize().map_err(|e| e.to_string())?;
    let folder = folder.canonicalize().map_err(|e| e.to_string())?;
    if !folder.is_dir() || !folder.starts_with(library) {
        return Err("Choose a course folder inside the library root".into());
    }
    let mut preview = Preview {
        path: path_string(&folder)?,
        title: clean_title(&folder.file_name().unwrap_or_default().to_string_lossy()),
        sections: Vec::new(),
        warnings: Vec::new(),
        resources: Vec::new(),
    };
    walk(&folder, &folder, &mut preview, 0);
    Ok(preview)
}

fn walk(root: &Path, folder: &Path, preview: &mut Preview, depth: usize) {
    if depth > 32 {
        preview.warnings.push(format!(
            "Directory nesting exceeds 32 levels: {}",
            folder.display()
        ));
        return;
    }
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) => {
            preview
                .warnings
                .push(format!("Cannot read {}: {e}", folder.display()));
            return;
        }
    };
    let mut files = Vec::new();
    let mut folders = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                preview.warnings.push(e.to_string());
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if let Err(e) = path_string(&entry.path()) {
            preview.warnings.push(e);
            continue;
        }
        match entry.file_type() {
            Ok(t) if t.is_symlink() => preview
                .warnings
                .push(format!("Skipped symlink: {}", entry.path().display())),
            Ok(t) if t.is_dir() => folders.push(entry.path()),
            Ok(t) if t.is_file() => files.push(entry.path()),
            Err(e) => preview.warnings.push(e.to_string()),
            _ => (),
        }
    }
    files.sort_by(|a, b| natord::compare(&a.to_string_lossy(), &b.to_string_lossy()));
    folders.sort_by(|a, b| natord::compare(&a.to_string_lossy(), &b.to_string_lossy()));
    let videos: Vec<_> = files.iter().filter(|p| extension(p) == "mp4").collect();
    let mut captions: Vec<_> = files
        .iter()
        .filter(|p| matches!(extension(p).as_str(), "srt" | "vtt"))
        .cloned()
        .collect();
    for sub in &folders {
        if is_subtitle_dir(sub) {
            match fs::read_dir(sub) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(e)
                                if e.file_type().is_ok_and(|t| t.is_file())
                                    && matches!(extension(&e.path()).as_str(), "srt" | "vtt") =>
                            {
                                captions.push(e.path())
                            }
                            Err(e) => preview.warnings.push(e.to_string()),
                            _ => (),
                        }
                    }
                }
                Err(e) => preview.warnings.push(format!("Cannot read captions: {e}")),
            }
        }
    }
    captions.sort_by(|a, b| natord::compare(&a.to_string_lossy(), &b.to_string_lossy()));
    let mut lessons: Vec<PreviewLesson> = videos
        .iter()
        .filter_map(|p| match relative(root, p) {
            Ok(path) => Some(PreviewLesson {
                title: clean_title(&stem(p)),
                path,
                captions: Vec::new(),
                resources: Vec::new(),
            }),
            Err(e) => {
                preview.warnings.push(e);
                None
            }
        })
        .collect();
    for file in files.iter().filter(|p| {
        !matches!(
            extension(p).as_str(),
            "mp4"
                | "srt"
                | "vtt"
                | "part"
                | "tmp"
                | "crdownload"
                | "torrent"
                | "nfo"
                | "url"
                | "lnk"
        )
    }) {
        let matches: Vec<_> = videos
            .iter()
            .enumerate()
            .filter(|(_, video)| {
                stem(file) == stem(video)
                    || leading_number(&stem(file))
                        .is_some_and(|n| leading_number(&stem(video)) == Some(n))
            })
            .map(|(i, _)| i)
            .collect();
        if let Ok(asset) = asset(root, file, "resource") {
            if matches.len() == 1 && matches[0] < lessons.len() {
                lessons[matches[0]].resources.push(asset);
            } else {
                preview.resources.push(asset);
            }
        }
    }
    for caption in captions {
        if extension(&caption) == "srt" && caption.with_extension("vtt").is_file() {
            continue;
        }
        let matches: Vec<_> = videos
            .iter()
            .enumerate()
            .filter(|(_, video)| caption_matches(&stem(&caption), &stem(video)))
            .map(|(i, _)| i)
            .collect();
        if matches.len() == 1 && matches[0] < lessons.len() {
            if let Ok(asset) = asset(root, &caption, "caption") {
                lessons[matches[0]].captions.push(asset);
            }
        } else {
            preview
                .warnings
                .push(format!("Unassigned caption: {}", caption.display()));
        }
    }
    if !lessons.is_empty() {
        let rel = relative(root, folder).unwrap_or_default();
        let title = if rel.is_empty() {
            "General".into()
        } else {
            folder
                .strip_prefix(root)
                .unwrap()
                .components()
                .map(|c| clean_title(&c.as_os_str().to_string_lossy()))
                .collect::<Vec<_>>()
                .join(" / ")
        };
        preview.sections.push(PreviewSection {
            title,
            path: rel,
            lessons,
        });
    }
    for sub in folders {
        if !is_subtitle_dir(&sub) {
            walk(root, &sub, preview, depth + 1);
        }
    }
}

fn is_subtitle_dir(p: &Path) -> bool {
    matches!(
        p.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .as_str(),
        "subs" | "subtitles"
    )
}
fn relative(root: &Path, path: &Path) -> Result<String, String> {
    path_string(path.strip_prefix(root).map_err(|e| e.to_string())?)
}
fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}
fn stem(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
fn leading_number(s: &str) -> Option<u64> {
    s.chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}
fn asset(root: &Path, path: &Path, kind: &str) -> Result<PreviewAsset, String> {
    Ok(PreviewAsset {
        path: relative(root, path)?,
        title: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        kind: kind.into(),
    })
}
fn caption_matches(caption: &str, video: &str) -> bool {
    caption == video
        || caption.strip_prefix(video).is_some_and(|tail| {
            tail.starts_with('.') || tail.starts_with(" - ") || tail.starts_with('_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn file(root: &Path, name: &str) {
        let p = root.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "").unwrap();
    }
    #[test]
    fn nested_mixed_courses_keep_section_numbering_and_captions() {
        let d = tempdir().unwrap();
        let course = d.path().join("Course");
        for name in [
            "1 intro.mp4",
            "02 Basics/10 fin.mp4",
            "02 Basics/2 café.mp4",
            "02 Basics/2 café - English.srt",
            "02 Basics/2 slides.pdf",
            "03 Advanced/01 Deep/1 intro.mp4",
            "ignored.part",
        ] {
            file(&course, name);
        }
        let p = scan_course(d.path(), &course).unwrap();
        assert_eq!(p.sections.len(), 3);
        assert_eq!(p.sections[1].lessons[0].title, "café");
        assert_eq!(p.sections[1].lessons[0].captions.len(), 1);
        assert_eq!(p.sections[1].lessons[0].resources.len(), 1);
        assert_eq!(scan_library(d.path()).unwrap().len(), 1);
    }
    #[test]
    fn ambiguous_captions_are_not_guessed() {
        let d = tempdir().unwrap();
        for n in ["1.mp4", "1.en.mp4", "1.en.srt"] {
            file(d.path(), n);
        }
        let p = scan_course(d.path(), d.path()).unwrap();
        assert_eq!(p.warnings.len(), 1);
        assert!(p.sections[0].lessons.iter().all(|l| l.captions.is_empty()));
    }
    #[test]
    fn rejects_external_paths_and_reports_symlinks() {
        let d = tempdir().unwrap();
        let other = tempdir().unwrap();
        file(other.path(), "x.mp4");
        assert!(scan_course(d.path(), other.path()).is_err());
        assert!(approved_file(d.path(), &other.path().join("x.mp4")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(other.path(), d.path().join("linked")).unwrap();
            assert_eq!(scan_course(d.path(), d.path()).unwrap().warnings.len(), 1);
        }
    }
}
