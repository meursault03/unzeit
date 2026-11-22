pub mod db;
#[cfg(feature = "desktop")]
mod desktop;
pub mod inference;
pub mod manifest;
pub mod models;
#[cfg(all(feature = "desktop", target_os = "linux"))]
mod online_player;
pub mod resources;
pub mod scanner;
pub mod search;
pub mod server;
#[cfg(feature = "webview-smoke")]
mod smoke;
pub mod subtitles;
pub mod transcript;
pub mod transcription;
#[cfg(feature = "desktop")]
pub use desktop::run;
