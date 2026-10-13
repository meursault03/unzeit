# Unzeit

English · [Português](README.pt.md)

A desktop library for courses you have already downloaded. Watch lessons, resume
where you stopped, and keep notes, captions, and course resources together.
Videos stay in their folders; study data stays on your computer.

[Install](#install-on-linux) · [First course](#import-your-first-course) ·
[Study](#study-a-lesson) · [Routine](#study-routine) · [Captions](#generate-captions) ·
[Troubleshooting](#troubleshooting) · [Contributing](#contributing)

![Unzeit study mode with video, captions, and notes side by side](docs/images/unzeit-study.png)

*Linux app screenshot with a demonstration course.*

## Install on Linux

Unzeit is in development, with local verification on Linux x86_64.
Download an installer from [release 0.4.0](https://github.com/meursault03/unzeit/releases/tag/v0.4.0):
AppImage, Arch package, Fedora RPM or Flatpak. Instructions follow below.
Windows and macOS have not been verified.

To use the downloaded AppImage, enable execution in its file
properties and open it. From a terminal in the file's folder:

```sh
chmod +x ./Unzeit_0.4.0_amd64.AppImage
./Unzeit_0.4.0_amd64.AppImage
```

Replace the filename with your file. The AppImage still requires GStreamer
codecs and your desktop's portals. FFmpeg and FFprobe are needed to generate
captions; import the Whisper model separately.

For a local package, use the manager for your installation:

| Format | Install |
| --- | --- |
| Arch `.pkg.tar.zst` | `sudo pacman -U /path/to/unzeit.pkg.tar.zst` |
| Fedora `.rpm` | `sudo dnf install /path/to/Unzeit.rpm` |
| Flatpak `.flatpak` | `flatpak install --user /path/to/Unzeit.flatpak` |

Replace paths with the package you received or built. From a repository checkout,
`bash packaging/install.sh /path/to/package.rpm` chooses the manager for all
three formats and keeps its installation confirmation. The script does not
download files. Fedora and Flatpak recipes are still experimental; their packaging instructions are in [CONTRIBUTING.md](CONTRIBUTING.md#english).
The first Flatpak installation may need to download its runtime from Flathub.

## Import your first course

1. Extract the course if it is a ZIP, RAR, or another archive.
2. Open **Import courses** and use **Browse** to choose the folder containing
   your courses. You can also paste its path into **Library root**.
3. Click **Scan library**. Review sections and lessons in the preview, and
   deselect any courses you do not want to add.
4. Click **Import**. Open the course in your library to watch its first lesson.

Use **Cancel** in the system picker to leave without choosing a folder, or
**Back to library** to leave import. Scanning only prepares the preview; the
import happens after confirmation.

An example folder layout:

```text
Courses/
  Rust basics/
    01 Introduction/
      01 Welcome.mp4
      01 Welcome.en.srt
      Slides.pdf
    02 Exercises/
      01 First program.mp4
      solution.rs
      reading.html
```

Choose `Courses` as the root in this example. Flat courses and nested sections
also work. Only MP4 videos are indexed; numbering can restart in each section.
Files are naturally sorted and never renamed or moved. Symlinks are skipped.

For a wrapper folder, use **Choose inner folder**. For a multi-course pack,
deselect the pack and use **Add individual course** for each course. The preview
shows warnings when captions cannot be matched safely to a lesson.

## Study a lesson

The player saves your playback position. Click **Mark complete** to finish a
lesson; reaching the end does not mark completion automatically. **Continue
learning** in the library opens your recent lesson.

- **Notes & bookmarks:** write and save a note, or use **Bookmark moment**.
  Select a saved timestamp to return to that moment. Deletion asks for confirmation.
- **Study mode:** places notes, transcripts, or resources beside the video.
  Normal mode keeps them below, with a separately scrolling curriculum.
- **Transcript:** find a phrase in the captions and select its cue to hear it.
  Global search also covers course titles, lessons, notes, and caption text.
- **Repeat interval:** expand the controls, mark A and B, and enable repetition.
  Active repeats remain indicated when the controls are collapsed.

### Course resources

Open **Resources** and select a file. Both lesson and course resources appear
in this list.

| Resource | How it works |
| --- | --- |
| PDF | Read and navigate pages inside the app. |
| Image | Preview inside the app. Images above 16 MiB use the external application. |
| UTF-8 text and code | Search, select lines, and copy an excerpt. Preview is limited to 1 MiB and never executes code. |
| HTML/HTM | Opens automatically in your default browser. Use **Open in browser** to retry. |
| SVG, executables, and formats without a preview | Use **Show in folder** or the external action named on the button. |

To select a code excerpt, click the first line number and Shift-click the last.
You can also enter line numbers and use **Copy selected lines**.

### Player shortcuts

| Key | Action |
| --- | --- |
| Space or K | Play or pause |
| J / L or left / right arrows | Seek backward or forward 10 seconds |
| F | Fullscreen |

Shortcuts do not intercept typing in text fields.

## Study routine

In the library, expand **Study time this week** to see your study minutes from
Monday through Sunday. Enter a **Weekly goal (minutes)** and click **Save goal**
to set a target. Goals are optional; use `0` to turn yours off.

Time is counted from real playback time, including at faster playback speeds.
Pauses, buffering, seeking, and computer suspension are excluded. This history
starts with version 0.4; earlier progress is not converted into estimated hours.
Lesson completion remains separate from your goal.

## Generate captions

Existing captions do not need Whisper. Give an SRT/VTT file the same video
basename, for example `01 Welcome.mp4` and `01 Welcome.en.srt`. `Subs` and
`Subtitles` directories are also recognized. After adding files to an imported
course, use **Rescan course**.

To transcribe a lesson without captions:

1. Download a **multilingual medium GGML** model from the
   [official whisper.cpp collection](https://huggingface.co/ggerganov/whisper.cpp/tree/main),
   such as `ggml-medium.bin` or `ggml-medium-q5_0.bin`. `.en` models are not accepted.
2. In **Settings**, click **Import medium model** and choose the `.bin` file.
   Unzeit validates it and copies it into app data. The full model is about
   1.5 GB; quantized versions use less space.
3. Open the lesson and click **Generate captions**. For the course, use
   **Caption remaining lessons** in the curriculum.
4. Follow progress in **Transcription**. You can keep watching while the queue runs.

Transcription runs on the CPU, one lesson at a time. Depending on your processor,
generating captions can take longer than the lesson itself. **Cancel** requests a stop;
this also works during model loading. Status changes to cancelled after the
process stops. SRT/VTT files are written beside the video, with app-data fallback
for read-only course folders.
Existing captions and videos are never overwritten. Content is not translated
and videos are not rewritten.

### Use captions in another player

Keep the SRT beside the video or create an MP4 copy with a selectable caption
track. This step runs in a terminal with [FFmpeg](https://ffmpeg.org/ffmpeg.html):

```sh
ffmpeg -n -i 'lesson.mp4' -i 'lesson.generated.srt' \
  -map 0 -map 1:0 -c copy -c:s mov_text 'lesson.captioned.mp4'
```

Replace filenames with yours. `-n` prevents overwriting the output; audio and
video are copied without re-encoding. The player can enable or disable the
caption track; captions are not burned into the video image.

## Language, data, and updates

On first launch, Unzeit uses the first supported system language: Portuguese,
English, Spanish, Russian, Simplified Chinese, or French, with English as fallback.
Change it in **Settings → App
language**; that choice is saved and does not translate courses. Light, dark,
and system themes are also available in Settings.

In selection menus, use arrow keys to browse, Enter to confirm, or Escape to
close without changing the selection. You can also click outside to dismiss a list.

Progress, completion, notes, and preferences live in app data. Close the app
before copying the directory for a backup:

| Installation | Default Linux directory |
| --- | --- |
| Native / AppImage | `~/.local/share/local.unzeit.desktop/` |
| Flatpak | `~/.var/app/local.unzeit.desktop/data/local.unzeit.desktop/` |

XDG environment settings may change these paths. Flatpak data is separate:
import your library and model there. Older versions may not open an upgraded
database; back up before updating.

Rescans preserve progress and notes. If you move a course or rename a lesson,
use **Relink folder** or **Relink lesson** before rescanning. Only one instance
may use the same app-data directory.

## Troubleshooting

| Problem | What to check |
| --- | --- |
| App opens, but video does not play | Your distro's GStreamer codecs must support the video. FFmpeg alone does not supply player codecs. |
| Folder picker does not open | Install `xdg-desktop-portal` and the backend for your desktop: `-kde`, `-gnome`, or `-gtk`. |
| AppImage reports a FUSE error | Try `APPIMAGE_EXTRACT_AND_RUN=1 ./Unzeit_0.4.0_amd64.AppImage`, using your filename. [AppImage guide](https://docs.appimage.org/user-guide/run-appimages.html). |
| No courses found | Check the root, extract archives, and confirm there are MP4 files inside the course folders. |
| Video appears missing | Relink its course folder or lesson, then rescan the course. |
| Transcription does not start | Check FFmpeg/FFprobe in Settings and import a multilingual medium model. |
| HTML did not open | Use **Open in browser** and check your default browser. Without a registered browser, the system's HTML association is used. |

Report problems in an [issue](https://github.com/meursault03/unzeit/issues) with
the app version, distro, desktop, installation format, and reproduction steps.
An error excerpt helps; you do not need to attach your course.

## Contributing

For source builds, package recipes and development checks, see
[CONTRIBUTING.md](CONTRIBUTING.md#english).

## License

MIT. Google Sans Flex is bundled under SIL OFL 1.1, with its license and trademark
notices in `public/fonts/`. PDF.js uses Apache-2.0; support files and styles retain
upstream attribution.
