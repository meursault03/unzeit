use crate::{
    db::{self, Db},
    models::{Job, PreviewAsset},
    scanner::path_string,
    subtitles,
};
use ffmpeg_sidecar::command::FfmpegCommand;
use rusqlite::params;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use whisper_rs::{WhisperContext, WhisperContextParameters};

pub type Reporter = Arc<dyn Fn(Job) + Send + Sync>;
pub struct Worker {
    pub db: Db,
    pub data: PathBuf,
    active: Mutex<Option<(i64, Arc<AtomicBool>)>>,
    stopped: AtomicBool,
    reporter: Reporter,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}
impl Worker {
    pub fn start(db: Db, data: PathBuf, reporter: Reporter) -> Arc<Self> {
        let worker = Arc::new(Self {
            db,
            data,
            active: Mutex::new(None),
            stopped: AtomicBool::new(false),
            reporter,
            thread: Mutex::new(None),
        });
        let running = worker.clone();
        *worker.thread.lock().expect("New worker lock") =
            Some(thread::spawn(move || running.run()));
        worker
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Ok(active) = self.active.lock() {
            if let Some((_, flag)) = &*active {
                flag.store(true, Ordering::SeqCst);
            }
        }
        if let Ok(mut handle) = self.thread.lock() {
            if let Some(handle) = handle.take() {
                handle.thread().unpark();
                let _ = handle.join();
            }
        }
    }
    pub fn cancel(&self, id: i64) -> Result<(), String> {
        // Match the worker's lock order so cancellation cannot miss a newly claimed job.
        let active = self.active.lock().map_err(|e| e.to_string())?;
        if let Some((active_id, flag)) = &*active {
            if *active_id == id {
                flag.store(true, Ordering::SeqCst);
            }
        }
        self.db.lock().map_err(|e|e.to_string())?.execute("UPDATE jobs SET status='cancelled',stage='cancelled' WHERE id=?1 AND status='queued'",[id]).map_err(|e|e.to_string())?;
        Ok(())
    }
    fn run(self: Arc<Self>) {
        while !self.stopped.load(Ordering::SeqCst) {
            let claimed = (|| -> Result<Option<(Job, Arc<AtomicBool>)>, String> {
                let mut active = self.active.lock().map_err(|e| e.to_string())?;
                if self.stopped.load(Ordering::SeqCst) {
                    return Ok(None);
                }
                let conn = self.db.lock().map_err(|e| e.to_string())?;
                let job = db::jobs(&conn)?
                    .into_iter()
                    .filter(|j| j.status == "queued")
                    .min_by_key(|j| j.id);
                if let Some(job) = job {
                    let flag = Arc::new(AtomicBool::new(false));
                    conn.execute("UPDATE jobs SET status='running',stage='extracting',progress=0,error=NULL WHERE id=?1",[job.id]).map_err(|e|e.to_string())?;
                    *active = Some((job.id, flag.clone()));
                    return Ok(Some((job, flag)));
                }
                Ok(None)
            })();
            if let Ok(Some((job, flag))) = claimed {
                self.update(job.id, "running", "extracting", 0, None);
                let result = self.transcribe(&job, flag.clone());
                match result {
                    Ok(()) => self.update(job.id, "completed", "completed", 100, None),
                    Err(e) => {
                        let status = if flag.load(Ordering::SeqCst) {
                            "cancelled"
                        } else {
                            "failed"
                        };
                        self.update(job.id, status, status, 0, Some(e));
                    }
                }
                if let Ok(mut active) = self.active.lock() {
                    *active = None;
                }
            } else {
                thread::park_timeout(Duration::from_millis(300));
            }
        }
    }
    fn update(&self, id: i64, status: &str, stage: &str, progress: i32, error: Option<String>) {
        let event = (|| -> Result<Option<Job>, String> {
            let conn = self.db.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE jobs SET status=?2,stage=?3,progress=?4,error=?5 WHERE id=?1",
                params![id, status, stage, progress, error],
            )
            .map_err(|e| e.to_string())?;
            Ok(db::jobs(&conn)?.into_iter().find(|j| j.id == id))
        })();
        if let Ok(Some(job)) = event {
            (self.reporter)(job);
        }
    }
    fn transcribe(self: &Arc<Self>, job: &Job, cancel: Arc<AtomicBool>) -> Result<(), String> {
        let (course, root, video, model) = {
            let conn = self.db.lock().map_err(|e| e.to_string())?;
            let (course, root, video) = db::lesson_file(&conn, job.lesson_id)?;
            (course, root, video, db::settings(&conn)?.model_path)
        };
        if model.is_empty() {
            return Err("Import a medium Whisper model in Settings first".into());
        }
        let temp = self.data.join("audio");
        fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
        let wav = temp.join(format!("{}.wav", job.id));
        let result = (|| {
            let duration = probe_duration(&video).unwrap_or(0.0);
            extract(&video, &wav, cancel.clone(), |seconds| {
                if duration > 0.0 {
                    self.update(
                        job.id,
                        "running",
                        "extracting",
                        ((seconds / duration * 100.0) as i32).clamp(0, 99),
                        None,
                    );
                }
            })?;
            cancelled(&cancel)?;
            let srt = crate::inference::run(
                Path::new(&model),
                &wav,
                cancel.clone(),
                |stage, progress| {
                    self.update(job.id, "running", stage, progress, None);
                },
            )?;
            cancelled(&cancel)?;
            let vtt = subtitles::to_vtt(&srt)?;
            self.update(job.id, "running", "writing-captions", 100, None);
            cancelled(&cancel)?;
            let (srt_path, vtt_path, external) =
                publish_captions(&video, &self.data, job.lesson_id, &srt, &vtt)?;
            let mut conn = self.db.lock().map_err(|e| e.to_string())?;
            let caption_path = if external {
                vtt_path.strip_prefix(&self.data)
            } else {
                vtt_path.strip_prefix(&root)
            }
            .map_err(|e| e.to_string())?;
            db::upsert_asset(
                &conn,
                course,
                Some(job.lesson_id),
                &PreviewAsset {
                    path: path_string(caption_path)?,
                    title: "Generated captions".into(),
                    kind: "caption".into(),
                },
                external,
            )?;
            let asset = conn
                .query_row(
                    "SELECT id FROM assets WHERE course_id=?1 AND path=?2 AND kind='caption'",
                    params![course, path_string(caption_path)?],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?;
            crate::transcript::store(&mut conn, asset, job.lesson_id, &subtitles::cues(&srt)?)?;
            let _ = srt_path;
            Ok(())
        })();
        let _ = fs::remove_file(wav);
        result
    }
}
fn cancelled(flag: &AtomicBool) -> Result<(), String> {
    if flag.load(Ordering::SeqCst) {
        Err("Cancelled".into())
    } else {
        Ok(())
    }
}
pub fn load_medium(path: &Path) -> Result<WhisperContext, String> {
    let context = WhisperContext::new_with_params(
        path,
        WhisperContextParameters {
            use_gpu: false,
            ..Default::default()
        },
    )
    .map_err(|e| format!("Cannot load Whisper model: {e}"))?;
    if !context.is_multilingual() || context.model_n_audio_layer() != 24 {
        return Err(
            "Choose a multilingual medium GGML model (ggml-medium.bin or a medium quantization)"
                .into(),
        );
    }
    Ok(context)
}
pub fn probe_duration(video: &Path) -> Result<f64, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| "Cannot probe duration".into())
}
pub fn extract(
    video: &Path,
    wav: &Path,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(f64),
) -> Result<(), String> {
    cancelled(&cancel)?;
    let mut command = FfmpegCommand::new();
    command
        .args([
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-n",
            "-protocol_whitelist",
            "file,pipe",
            "-i",
        ])
        .arg(video)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
            "-progress",
            "pipe:1",
            "-stats_period",
            "0.5",
        ])
        .arg(wav);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Install FFmpeg: {e}"))?;
    let stdout = child.take_stdout().ok_or("Missing FFmpeg output")?;
    let stderr = child.take_stderr().ok_or("Missing FFmpeg diagnostics")?;
    let child = Arc::new(Mutex::new(child));
    let monitor = child.clone();
    let flag = cancel.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let done = finished.clone();
    let watcher = thread::spawn(move || {
        while !done.load(Ordering::SeqCst) {
            if flag.load(Ordering::SeqCst) {
                if let Ok(mut child) = monitor.lock() {
                    let _ = child.kill();
                }
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    let diagnostics = thread::spawn(move || {
        let mut text = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if text.len() < 16_000 {
                text.push_str(&line);
                text.push('\n');
            }
        }
        text
    });
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(value) = line
            .strip_prefix("out_time_us=")
            .and_then(|v| v.parse::<f64>().ok())
        {
            progress(value / 1_000_000.0);
        }
    }
    let status = child
        .lock()
        .map_err(|e| e.to_string())?
        .wait()
        .map_err(|e| e.to_string());
    finished.store(true, Ordering::SeqCst);
    let _ = watcher.join();
    let error = diagnostics.join().unwrap_or_default();
    cancelled(&cancel)?;
    if status?.success() {
        Ok(())
    } else {
        Err(format!("Audio extraction failed: {error}"))
    }
}
pub(crate) fn read_audio(path: &Path) -> Result<Vec<f32>, String> {
    let reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.channels != 1
        || spec.sample_rate != 16000
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err("Expected 16 kHz mono 16-bit PCM WAV".into());
    }
    // shortcut: entire lesson audio is held in memory; chunk inference if multi-hour lessons exhaust RAM.
    reader
        .into_samples::<i16>()
        .map(|s| s.map(|s| f32::from(s) / 32768.0).map_err(|e| e.to_string()))
        .collect()
}
pub fn publish_captions(
    video: &Path,
    data: &Path,
    id: i64,
    srt: &str,
    vtt: &str,
) -> Result<(PathBuf, PathBuf, bool), String> {
    let local_srt = video.with_extension("generated.srt");
    let local_vtt = video.with_extension("generated.vtt");
    if local_srt.exists() || local_vtt.exists() {
        return Err(
            "Generated captions already exist; import or remove them before retrying".into(),
        );
    }
    let publish = |srt_path: &Path, vtt_path: &Path| -> Result<(), String> {
        subtitles::atomic_write_new(srt_path, srt)?;
        if let Err(e) = subtitles::atomic_write_new(vtt_path, vtt) {
            let _ = fs::remove_file(srt_path);
            return Err(e);
        }
        Ok(())
    };
    if publish(&local_srt, &local_vtt).is_ok() {
        return Ok((local_srt, local_vtt, false));
    }
    let folder = data.join("captions");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let srt_path = folder.join(format!("{id}.generated.srt"));
    let vtt_path = folder.join(format!("{id}.generated.vtt"));
    publish(&srt_path, &vtt_path)?;
    Ok((srt_path, vtt_path, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_worker_shutdown_does_not_wait_for_polling_interval() {
        let data = tempfile::tempdir().unwrap();
        let db = db::open(&data.path().join("test.sqlite")).unwrap();
        let worker = Worker::start(db, data.path().to_path_buf(), Arc::new(|_| {}));
        thread::sleep(Duration::from_millis(50));
        let started = std::time::Instant::now();
        worker.stop();
        assert!(
            started.elapsed() < Duration::from_millis(150),
            "Idle worker waits for its polling interval"
        );
        worker.stop();
    }
    #[test]
    #[ignore = "requires UNZEIT_TEST_MODEL and UNZEIT_TEST_AUDIO official fixtures"]
    fn real_medium_transcription_preserves_video_and_reports_progress() {
        let model = std::env::var("UNZEIT_TEST_MODEL").expect("UNZEIT_TEST_MODEL");
        let audio = std::env::var("UNZEIT_TEST_AUDIO").expect("UNZEIT_TEST_AUDIO");
        let course = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let video = course.path().join("01 JFK.mp4");
        assert!(Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=320x180:r=5",
                "-i"
            ])
            .arg(audio)
            .args([
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest"
            ])
            .arg(&video)
            .status()
            .unwrap()
            .success());
        let original = fs::read(&video).unwrap();
        let db = db::open(&data.path().join("test.sqlite")).unwrap();
        {
            let mut conn = db.lock().unwrap();
            db::import(
                &mut conn,
                course.path(),
                &[crate::scanner::scan_course(course.path(), course.path()).unwrap()],
            )
            .unwrap();
            let mut settings = db::settings(&conn).unwrap();
            settings.model_path = model;
            db::save_settings(&conn, &settings).unwrap();
            let lesson = db::library(&conn).unwrap()[0].lessons[0].id;
            db::enqueue(&mut conn, &[lesson]).unwrap();
        }
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = events.clone();
        let worker = Worker::start(
            db.clone(),
            data.path().to_path_buf(),
            Arc::new(move |job| recorded.lock().unwrap().push(job)),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(240);
        let job = loop {
            let job = db::jobs(&db.lock().unwrap()).unwrap().remove(0);
            if !["running", "queued"].contains(&job.status.as_str()) {
                break job;
            }
            if std::time::Instant::now() > deadline {
                worker.stop();
                panic!("Transcription timed out");
            }
            thread::sleep(Duration::from_millis(100));
        };
        worker.stop();
        assert_eq!(job.status, "completed", "{:?}", job.error);
        assert_eq!(fs::read(&video).unwrap(), original);
        let srt = fs::read_to_string(video.with_extension("generated.srt")).unwrap();
        assert!(srt.to_lowercase().contains("country"), "{srt}");
        let spoken = crate::search::search(&db.lock().unwrap(), "country").unwrap();
        assert!(spoken
            .iter()
            .any(|r| r.kind == "caption" && r.position.is_some()));
        assert!(fs::read_to_string(video.with_extension("generated.vtt"))
            .unwrap()
            .starts_with("WEBVTT"));
        let exported = course.path().join("captioned.mp4");
        assert!(Command::new("ffmpeg")
            .args(["-v", "error", "-n", "-i"])
            .arg(&video)
            .arg("-i")
            .arg(video.with_extension("generated.srt"))
            .args(["-map", "0", "-map", "1:0", "-c", "copy", "-c:s", "mov_text"])
            .arg(&exported)
            .status()
            .unwrap()
            .success());
        let codec = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "s",
                "-show_entries",
                "stream=codec_name",
                "-of",
                "csv=p=0",
            ])
            .arg(&exported)
            .output()
            .unwrap();
        assert!(codec.status.success());
        assert_eq!(String::from_utf8_lossy(&codec.stdout).trim(), "mov_text");
        assert_eq!(fs::read(&video).unwrap(), original);
        fs::remove_file(exported).unwrap();
        assert_eq!(fs::read_dir(data.path().join("audio")).unwrap().count(), 0);
        let events = events.lock().unwrap();
        for stage in [
            "extracting",
            "loading-model",
            "transcribing",
            "writing-captions",
            "completed",
        ] {
            assert!(events.iter().any(|e| e.stage == stage), "Missing {stage}");
        }
        drop(events);
        let second = course.path().join("02 Cancel.mp4");
        fs::write(&second, &original).unwrap();
        let lesson = {
            let mut conn = db.lock().unwrap();
            db::import(
                &mut conn,
                course.path(),
                &[crate::scanner::scan_course(course.path(), course.path()).unwrap()],
            )
            .unwrap();
            let lesson = db::library(&conn).unwrap()[0]
                .lessons
                .iter()
                .find(|l| l.path == "02 Cancel.mp4")
                .unwrap()
                .id;
            db::enqueue(&mut conn, &[lesson]).unwrap();
            lesson
        };
        let worker = Worker::start(db.clone(), data.path().to_path_buf(), Arc::new(|_| {}));
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut requested = false;
        let job = loop {
            let job = db::jobs(&db.lock().unwrap())
                .unwrap()
                .into_iter()
                .find(|j| j.lesson_id == lesson)
                .unwrap();
            if !requested && job.stage == "transcribing" {
                worker.cancel(job.id).unwrap();
                requested = true;
            }
            if !["running", "queued"].contains(&job.status.as_str()) {
                break job;
            }
            if std::time::Instant::now() > deadline {
                worker.stop();
                panic!("Cancellation timed out");
            }
            thread::sleep(Duration::from_millis(10));
        };
        worker.stop();
        assert!(requested);
        assert_eq!(job.status, "cancelled");
        assert!(!second.with_extension("generated.srt").exists());
        assert!(!second.with_extension("generated.vtt").exists());
        assert_eq!(fs::read_dir(data.path().join("audio")).unwrap().count(), 0);
        let third = course.path().join("03 Close while loading.mp4");
        fs::write(&third, &original).unwrap();
        let lesson = {
            let mut conn = db.lock().unwrap();
            db::import(
                &mut conn,
                course.path(),
                &[crate::scanner::scan_course(course.path(), course.path()).unwrap()],
            )
            .unwrap();
            let lesson = db::library(&conn).unwrap()[0]
                .lessons
                .iter()
                .find(|lesson| lesson.path == "03 Close while loading.mp4")
                .unwrap()
                .id;
            db::enqueue(&mut conn, &[lesson]).unwrap();
            lesson
        };
        let worker = Worker::start(db.clone(), data.path().to_path_buf(), Arc::new(|_| {}));
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            let job = db::jobs(&db.lock().unwrap())
                .unwrap()
                .into_iter()
                .find(|job| job.lesson_id == lesson)
                .unwrap();
            if job.stage == "loading-model" {
                break;
            }
            if job.status == "failed" || std::time::Instant::now() > deadline {
                worker.stop();
                panic!("Model-loading stage not observed: {:?}", job.error);
            }
            thread::sleep(Duration::from_millis(1));
        }
        let started = std::time::Instant::now();
        worker.stop();
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "Closing waits for the model to load"
        );
        assert_eq!(
            db::jobs(&db.lock().unwrap())
                .unwrap()
                .into_iter()
                .find(|job| job.lesson_id == lesson)
                .unwrap()
                .status,
            "cancelled"
        );
        assert!(!third.with_extension("generated.srt").exists());
        assert!(!third.with_extension("generated.vtt").exists());
        assert_eq!(fs::read(&third).unwrap(), original);
        assert_eq!(fs::read_dir(data.path().join("audio")).unwrap().count(), 0);
    }
    #[cfg(unix)]
    #[test]
    fn read_only_course_uses_app_data_captions() {
        use std::os::unix::fs::PermissionsExt;
        let course = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let video = course.path().join("lesson.mp4");
        fs::write(&video, "original").unwrap();
        fs::set_permissions(course.path(), fs::Permissions::from_mode(0o555)).unwrap();
        let result = publish_captions(&video, data.path(), 42, "srt", "vtt");
        fs::set_permissions(course.path(), fs::Permissions::from_mode(0o755)).unwrap();
        let (srt, vtt, external) = result.unwrap();
        assert!(external && srt.starts_with(data.path()) && vtt.starts_with(data.path()));
        assert_eq!(fs::read_to_string(video).unwrap(), "original");
    }
    #[test]
    fn caption_publication_preserves_source_and_rejects_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let video = dir.path().join("lesson.mp4");
        fs::write(&video, "original").unwrap();
        publish_captions(&video, data.path(), 1, "srt", "vtt").unwrap();
        assert!(publish_captions(&video, data.path(), 1, "new", "new").is_err());
        assert_eq!(fs::read_to_string(video).unwrap(), "original");
    }
    #[test]
    fn pre_cancelled_extraction_does_not_spawn_or_write() {
        let d = tempfile::tempdir().unwrap();
        let wav = d.path().join("audio.wav");
        assert!(extract(
            Path::new("missing.mp4"),
            &wav,
            Arc::new(AtomicBool::new(true)),
            |_| {}
        )
        .is_err());
        assert!(!wav.exists());
    }
    #[test]
    fn real_ffmpeg_extracts_pcm_and_handles_failure() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("tone.wav");
        let out = d.path().join("mono.wav");
        assert!(Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-ac",
                "2",
                "-ar",
                "44100"
            ])
            .arg(&source)
            .status()
            .unwrap()
            .success());
        extract(&source, &out, Arc::new(AtomicBool::new(false)), |_| {}).unwrap();
        assert!(read_audio(&out).unwrap().len() >= 16000);
        assert!(extract(
            &d.path().join("missing"),
            &d.path().join("bad.wav"),
            Arc::new(AtomicBool::new(false)),
            |_| {}
        )
        .is_err());
    }
}
