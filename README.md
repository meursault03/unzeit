# Unzeit

A desktop library for downloaded courses. Import MP4 lessons, resume playback,
keep timestamped notes, and generate captions locally with Whisper.

## Development

Requires Node.js 22.12+, Rust 1.90+, CMake, Clang/libclang, WebKitGTK 4.1,
GTK 3, FFmpeg/FFprobe, and GStreamer codecs for your videos.
See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
cd ~/Projects/unzeit
npm ci
npm run tauri dev
```

`npm run dev` provides a browser preview; local files require the desktop app.

## Library

```text
Courses/
  Example course/
    01 Introduction/
      01 Welcome.mp4
      01 Welcome.en.srt
      Slides.pdf
    02 Practice/
      01 Exercise.mp4
      solution.py
```

Choose `Courses` as your root and review the import preview. Flat courses and
nested sections work; numbering can restart in each section. Only MP4 videos
are indexed. Archives must be extracted. Files are naturally sorted and never
renamed or moved. Symlinks are skipped.

Captions match the video basename, optionally with a language suffix, or live in
`Subs`/`Subtitles`. Uncertain matches produce warnings. Other files become lesson
or course resources. HTML, SVG, and executable files open outside the app.

Progress, completion, notes, and settings live in the app-data directory,
typically `~/.local/share/local.unzeit.desktop/` on Linux. Back up this directory.
Rescans preserve your progress; relink moved folders or renamed lessons before
rescanning. Only one instance may use the same app-data directory.

## Study

Choose Portuguese, English, or Spanish in Settings. Interface language does not
translate course content. Google Sans Flex is bundled locally.

- Notes and bookmarks link to exact lesson timestamps. Completion is explicit.
- Search covers course/lesson titles, notes, and caption text. Select a result to
  open its lesson at the matching timestamp.
- Study mode places notes, transcripts, or resources beside the video. Normal
  mode keeps these below the player, with a separately scrolling curriculum.
- Expand Repeat interval to mark A and B. Active repeats remain indicated when
  the controls are collapsed.
- Preview PDFs, images, and UTF-8 source files locally. Source previews include
  line numbers, file search, and copying selected lines; code is never executed.
- Space/K toggles playback, J/L or arrows seek ten seconds, and F opens fullscreen.

Version 0.3.0 upgrades the database to schema 3 without resetting progress or
notes. Earlier executables cannot open schema 3; back up app data before upgrading.

## Captions

Import a multilingual medium GGML model from the official
[whisper.cpp model collection](https://huggingface.co/ggerganov/whisper.cpp/tree/main)
in Settings, then generate captions for a lesson or queue the course. Quantized
medium models are supported; English-only `.en` models are not.

FFmpeg extracts 16 kHz mono audio and whisper-rs runs on the CPU, one lesson at a
time. Jobs can be cancelled, though model loading must finish first. Generated
SRT/VTT files are written beside the video, with app-data fallback for read-only
folders. Existing captions and videos are never overwritten. No MP4 rewriting.

## Checks and packaging

```sh
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm run tauri build
```

The Linux AppImage is in `src-tauri/target/release/bundle/appimage/`. FFmpeg,
GStreamer codecs, and a separately imported model are still required.

`bash tests/webview-smoke.sh` tests the native Linux webview with generated media
and isolated app data. It saves results and screenshots in a temporary directory.
Build normally again afterward; its `webview-smoke` feature is for tests only.
Set `UNZEIT_TEST_CLOSE_ONLY=1` to check player shutdown and final progress saving.

For the real medium inference and cancellation check, supply a multilingual
medium model and the official [JFK sample](https://github.com/ggml-org/whisper.cpp/blob/master/samples/jfk.wav):

```sh
UNZEIT_TEST_MODEL=/absolute/path/ggml-medium-q5_0.bin \
UNZEIT_TEST_AUDIO=/absolute/path/jfk.wav \
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib \
  real_medium_transcription -- --ignored
```

## License

MIT. Google Sans Flex is bundled under SIL OFL 1.1; its license and trademark
notices are in `public/fonts/`. PDF.js is Apache-2.0; its TextLayer CSS and bundled
support files retain upstream attribution.
