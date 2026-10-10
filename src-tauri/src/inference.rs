use crate::{subtitles, transcription};
use serde::{Deserialize, Serialize};
use std::{
    ffi::c_void,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use whisper_rs::{FullParams, SamplingStrategy};

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
enum Message {
    Loading,
    Progress { percent: i32 },
    Complete { srt: String },
    Error { error: String },
}
fn emit(message: Message) {
    if let Ok(json) = serde_json::to_string(&message) {
        let mut output = std::io::stdout().lock();
        let _ = writeln!(output, "{json}");
        let _ = output.flush();
    }
}
unsafe extern "C" fn progress(
    _: *mut whisper_rs::WhisperSysContext,
    _: *mut whisper_rs::WhisperSysState,
    percent: i32,
    data: *mut c_void,
) {
    // Keep panics inside Rust; the counter lives until synchronous inference returns.
    let _ = std::panic::catch_unwind(|| {
        let last = unsafe { &*data.cast::<AtomicI32>() };
        if last.swap(percent, Ordering::Relaxed) != percent {
            emit(Message::Progress { percent });
        }
    });
}
fn transcribe(model: &Path, audio: &Path) -> Result<String, String> {
    let samples = transcription::read_audio(audio)?;
    emit(Message::Loading);
    let context = transcription::load_medium(model)?;
    let mut state = context.create_state().map_err(|e| e.to_string())?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(
        thread::available_parallelism()
            .map(|n| n.get().saturating_sub(2).clamp(1, 8) as i32)
            .unwrap_or(2),
    );
    params.set_language(None);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    let last = AtomicI32::new(-1);
    // whisper-rs 0.16 leaks safe callback allocations; use its synchronous raw callback.
    unsafe {
        params.set_progress_callback(Some(progress));
        params.set_progress_callback_user_data((&last as *const AtomicI32).cast_mut().cast());
    }
    emit(Message::Progress { percent: 0 });
    state.full(params, &samples).map_err(|e| e.to_string())?;
    let mut srt = String::new();
    for segment in state.as_iter() {
        let text = segment.to_str().map_err(|e| e.to_string())?.trim();
        let start = segment.start_timestamp() * 10;
        let end = segment.end_timestamp() * 10;
        if !text.is_empty() && end > start {
            srt.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                segment.segment_index() + 1,
                subtitles::timestamp(start, ','),
                subtitles::timestamp(end, ','),
                text
            ));
        }
    }
    Ok(srt)
}

pub fn child_entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--unzeit-transcribe")) {
        return None;
    }
    let result = match (args.next(), args.next(), args.next()) {
        (Some(model), Some(audio), None) => transcribe(Path::new(&model), Path::new(&audio)),
        _ => Err("Expected model and WAV paths".into()),
    };
    match result {
        Ok(srt) => {
            emit(Message::Complete { srt });
            Some(0)
        }
        Err(error) => {
            emit(Message::Error { error });
            Some(1)
        }
    }
}

pub fn run(
    model: &Path,
    audio: &Path,
    cancel: Arc<AtomicBool>,
    progress: impl FnMut(&str, i32),
) -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(test)]
    let executable = std::env::var_os("UNZEIT_TEST_EXECUTABLE")
        .map(std::path::PathBuf::from)
        .unwrap_or(executable);
    let mut command = Command::new(executable);
    command.arg("--unzeit-transcribe").arg(model).arg(audio);
    receive(command, cancel, progress)
}
fn receive(
    mut command: Command,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(&str, i32),
) -> Result<String, String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Cannot start transcription: {e}"))?;
    let stdout = child.stdout.take().expect("Piped inference output");
    let stderr = child.stderr.take().expect("Piped inference diagnostics");
    let child = Arc::new(Mutex::new(child));
    let done = Arc::new(AtomicBool::new(false));
    let monitor = child.clone();
    let flag = cancel.clone();
    let finished = done.clone();
    let watcher = thread::spawn(move || {
        while !finished.load(Ordering::SeqCst) {
            if flag.load(Ordering::SeqCst) {
                if let Ok(mut child) = monitor.lock() {
                    let _ = child.kill();
                }
                break;
            }
            thread::sleep(Duration::from_millis(25));
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
    let result = (|| {
        let mut output = None;
        for line in BufReader::new(stdout).lines() {
            match serde_json::from_str::<Message>(&line.map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid transcription response: {e}"))?
            {
                Message::Loading => progress("loading-model", 0),
                Message::Progress { percent } => progress("transcribing", percent.clamp(0, 100)),
                Message::Complete { srt } => output = Some(srt),
                Message::Error { error } => return Err(error),
            }
        }
        output.ok_or_else(|| "Transcription stopped without a result".to_string())
    })();
    let status = {
        let mut child = child.lock().map_err(|e| e.to_string())?;
        if result.is_err() {
            let _ = child.kill();
        }
        child.wait().map_err(|e| e.to_string())
    };
    done.store(true, Ordering::SeqCst);
    let _ = watcher.join();
    let diagnostics = diagnostics.join().unwrap_or_default();
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    if !status?.success() && result.is_ok() {
        return Err(format!("Transcription failed: {diagnostics}"));
    }
    result
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn cancellation_kills_a_process_blocked_loading_the_model() {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "printf '%s\\n' '{\"event\":\"progress\",\"percent\":0}'; exec sleep 20",
        ]);
        let flag = Arc::new(AtomicBool::new(false));
        let cancel = flag.clone();
        let started = std::time::Instant::now();
        assert_eq!(
            receive(command, flag, move |_, _| cancel
                .store(true, Ordering::SeqCst))
            .unwrap_err(),
            "Cancelled"
        );
        assert!(started.elapsed() < Duration::from_millis(500));
    }
    #[test]
    fn malformed_child_output_is_an_error_and_the_child_is_reaped() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf '%s\\n' invalid; exec sleep 20"]);
        let started = std::time::Instant::now();
        assert!(
            receive(command, Arc::new(AtomicBool::new(false)), |_, _| {})
                .unwrap_err()
                .contains("Invalid transcription response")
        );
        assert!(started.elapsed() < Duration::from_millis(500));
    }
    #[test]
    fn returns_child_result_and_reports_progress() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf '%s\\n' '{\"event\":\"progress\",\"percent\":50}' '{\"event\":\"complete\",\"srt\":\"captions\"}'"]);
        let mut reported = Vec::new();
        assert_eq!(
            receive(command, Arc::new(AtomicBool::new(false)), |_, p| reported
                .push(p))
            .unwrap(),
            "captions"
        );
        assert_eq!(reported, [50]);
    }
}
