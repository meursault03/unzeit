use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: String,
    pub title: String,
    pub author: Option<String>,
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
    pub source_url: Option<String>,
    pub pdf_page: Option<u32>,
    pub videos: Vec<OnlineVideo>,
    pub captions: Vec<PreviewAsset>,
    pub resources: Vec<PreviewAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineVideo {
    pub youtube_id: String,
    pub title: Option<String>,
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
    pub source_url: Option<String>,
    pub pdf_page: Option<u32>,
    pub videos: Vec<OnlineVideo>,
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
    pub author: Option<String>,
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
    #[serde(default = "default_content_scale")]
    pub content_scale: u16,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub weekly_goal_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyInterval {
    pub id: String,
    pub lesson_id: i64,
    pub started_at: f64,
    pub ended_at: f64,
    pub seconds: f64,
}

fn default_language() -> String {
    "pt-BR".into()
}

fn default_content_scale() -> u16 {
    100
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            library_root: String::new(),
            model_path: String::new(),
            speed: 1.0,
            autoplay: true,
            theme: "system".into(),
            content_scale: default_content_scale(),
            language: default_language(),
            weekly_goal_minutes: 0,
        }
    }
}
