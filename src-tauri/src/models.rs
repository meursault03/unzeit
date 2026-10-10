use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: String,
    pub title: String,
    pub sections: Vec<PreviewSection>,
    pub warnings: Vec<String>,
    pub resources: Vec<PreviewAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSection {
    pub title: String,
    pub path: String,
    pub lessons: Vec<PreviewLesson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLesson {
    pub title: String,
    pub path: String,
    pub captions: Vec<PreviewAsset>,
    pub resources: Vec<PreviewAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAsset {
    pub path: String,
    pub title: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: i64,
    pub title: String,
    pub kind: String,
    pub lesson_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: i64,
    pub title: String,
    pub section: String,
    pub path: String,
    pub position: f64,
    pub duration: f64,
    pub completed: bool,
    pub missing: bool,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub id: i64,
    pub title: String,
    pub path: String,
    pub lessons: Vec<Lesson>,
    pub resources: Vec<Asset>,
    pub last_lesson_id: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub lesson_id: i64,
    pub position: f64,
    pub text: String,
    pub bookmark: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: i64,
    pub lesson_id: i64,
    pub title: String,
    pub status: String,
    pub stage: String,
    pub progress: i32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub library_root: String,
    pub model_path: String,
    pub speed: f64,
    pub autoplay: bool,
    pub theme: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "pt-BR".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            library_root: String::new(),
            model_path: String::new(),
            speed: 1.0,
            autoplay: true,
            theme: "system".into(),
            language: default_language(),
        }
    }
}
