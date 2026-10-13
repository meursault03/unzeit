#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fixture_root=$(mktemp -d "${TMPDIR:-/tmp}/unzeit-webview.XXXXXX")
course_root="$fixture_root/courses/Test course/01 Basics"
mkdir -p "$course_root"
ffmpeg -hide_banner -loglevel error -f lavfi -i color=c=0x48513c:s=640x360:d=8 \
  -f lavfi -i sine=frequency=440:duration=8 -c:v libx264 -pix_fmt yuv420p \
  -c:a aac -shortest "$course_root/01 First lesson.mp4"
cat > "$course_root/01 First lesson.srt" <<'CAPTION'
1
00:00:00,000 --> 00:00:07,000
Hello from Unzeit
CAPTION
cp "$course_root/01 First lesson.mp4" "$course_root/02 Second lesson.mp4"
python3 - "$course_root" <<'FIXTURES'
from pathlib import Path
import sys, struct, zlib, shutil
root = Path(sys.argv[1])
(root.parent.parent/'Empty course').mkdir()
for i in range(2, 27):
    section=root.parent/f'{i:02d} Topic {i}'
    section.mkdir()
    shutil.copyfile(root/'01 First lesson.mp4', section/f'01 Topic {i}.mp4')
(root/'guide.txt').write_text('<script>window.UNZEIT_UNTRUSTED = true</script>\nPlain course notes — café')
(root/'solution.py').write_text('def café():\n    return \"<script>passive</script>\"\n' + '# exercise\n' * 248 + 'answer = 42\n# another exercise\nanswer = 43')
(root/'broken.pdf').write_text('This is not a PDF')
(root/'page.html').write_text('<script>window.UNZEIT_UNTRUSTED = true</script>')
def chunk(kind, data):
    return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
(root/'diagram.png').write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,32,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+bytes([72,81,60])*32)*32))+chunk(b'IEND',b''))
objects = [b'<< /Type /Catalog /Pages 2 0 R >>', b'<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>', b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>', None, b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>', None, b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>']
for index,label in [(3,'Course guide - first page'),(5,'Course guide - second page')]:
    stream=f'BT /F1 18 Tf 30 240 Td ({label}) Tj ET'.encode()
    objects[index]=f'<< /Length {len(stream)} >>\nstream\n'.encode()+stream+b'\nendstream'
pdf=b'%PDF-1.4\n'; offsets=[0]
for index,obj in enumerate(objects,1):
    offsets.append(len(pdf)); pdf+=f'{index} 0 obj\n'.encode()+obj+b'\nendobj\n'
xref=len(pdf); pdf+=f'xref\n0 {len(objects)+1}\n0000000000 65535 f \n'.encode()
for offset in offsets[1:]: pdf+=f'{offset:010d} 00000 n \n'.encode()
pdf+=f'trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF'.encode()
(root/'Guia — café.pdf').write_bytes(pdf)
FIXTURES
printf 'Test reports and native screenshots: %s\n' "$fixture_root"
npm run tauri -- build --features webview-smoke --no-bundle
python3 - "$fixture_root" <<'VERIFY'
import json, os, pathlib, sqlite3, subprocess, sys, time
from urllib.parse import unquote, urlparse
root = pathlib.Path(sys.argv[1])
xdg_data = root / 'xdg-data'
applications = xdg_data / 'applications'
applications.mkdir(parents=True)
xdg_config = root / 'xdg-config'
xdg_config.mkdir()
browser = root / 'browser-log.py'
browser.write_text('import pathlib, sys\nwith pathlib.Path(__file__).with_name("browser.log").open("a") as log:\n    log.write(sys.argv[1] + "\\n")\n')
(applications / 'unzeit-test-browser.desktop').write_text(f'[Desktop Entry]\nType=Application\nName=Unzeit test browser\nNoDisplay=true\nTerminal=false\nExec=/usr/bin/python3 {browser} %u\nMimeType=x-scheme-handler/https;x-scheme-handler/http;text/html;\n')
(xdg_config / 'mimeapps.list').write_text('[Default Applications]\nx-scheme-handler/https=unzeit-test-browser.desktop\nx-scheme-handler/http=unzeit-test-browser.desktop\ntext/html=unzeit-test-browser.desktop\n')
process = subprocess.Popen(['src-tauri/target/release/unzeit'], env={**os.environ, 'UNZEIT_TEST_DIR': str(root), 'XDG_DATA_HOME': str(xdg_data), 'XDG_CONFIG_HOME': str(xdg_config), 'XDG_CACHE_HOME': str(root / 'xdg-cache')})
def cancel_picker():
    import gi
    gi.require_version('Atspi', '2.0')
    from gi.repository import Atspi
    request = root / 'picker-request.json'
    title = json.loads(request.read_text())['title']
    def walk(node):
        yield node
        for index in range(node.get_child_count()):
            child = node.get_child_at_index(index)
            if child is not None:
                yield from walk(child)
    desktop = Atspi.get_desktop(0)
    for app_index in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(app_index)
        if app is None:
            continue
        for frame_index in range(app.get_child_count()):
            frame = app.get_child_at_index(frame_index)
            if frame is None or frame.get_name() != title:
                continue
            for node in walk(frame):
                if node.get_name() not in ['Cancel', 'Cancelar'] or node.get_role() != Atspi.Role.PUSH_BUTTON:
                    continue
                bounds = frame.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
                cancel = node.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
                geometry = {'title': title, 'dialog': [bounds.x, bounds.y, bounds.width, bounds.height], 'cancel': [cancel.x, cancel.y, cancel.width, cancel.height]}
                (root/'picker-geometry.json').write_text(json.dumps(geometry))
                assert node.get_action_iface().do_action(0), 'Native Cancel action failed'
                request.unlink()
                return
try:
    deadline = time.monotonic() + 150
    report = root / 'webview-result.json'
    while not report.exists() and process.poll() is None and time.monotonic() < deadline:
        if (root/'picker-request.json').exists():
            cancel_picker()
        time.sleep(0.1)
    assert report.exists(), 'Native test did not produce a report'
    result = json.loads(report.read_text())
    assert result['ok'], result
    if result.get('selectClose'):
        import re
        window = subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(process.pid)], text=True).splitlines()[0]
        subprocess.run(['xdotool','windowactivate','--sync',window],check=True)
        geometry = dict(line.split('=',1) for line in subprocess.check_output(['xdotool','getwindowgeometry','--shell',window],text=True).splitlines())
        x,y,width,height = [int(geometry[key]) for key in ['X','Y','WIDTH','HEIGHT']]
        frame = subprocess.check_output(['xprop','-id',window,'_NET_FRAME_EXTENTS'],text=True)
        extents = [int(value) for value in re.findall(r'\d+',frame.split('=',1)[-1])] if '=' in frame else [0,0,0,0]
        left,right,top,bottom = extents
        control_x,control_y,control_w,control_h = result['control']
        offset = max(0,height - result['viewport'][1]*result['ratio'])
        popup_x = round(x+(control_x+control_w/2)*result['ratio'])
        popup_y = round(y+offset+(control_y+control_h/2)*result['ratio'])
        subprocess.run(['xdotool','mousemove',str(popup_x),str(popup_y),'click','1'],check=True)
        time.sleep(.3)
        result['geometry'] = {'window': [x,y,width,height], 'frame':extents, 'controlClick':[popup_x,popup_y]}
        close_x = x+width+right-22
        close_y = round(y-top/2) if top else y+18
        started = time.monotonic()
        subprocess.run(['xdotool','mousemove',str(close_x),str(close_y),'click','1'],check=True)
        try:
            result['exitCode'] = process.wait(timeout=3)
            result['oneClickClosed'] = True
        except subprocess.TimeoutExpired:
            result['oneClickClosed'] = False
            subprocess.run(['xdotool','key','Escape'],check=True)
            time.sleep(.2)
            subprocess.run(['xdotool','mousemove',str(close_x),str(close_y),'click','1'],check=True)
            try:
                result['closedAfterEscape'] = process.wait(timeout=2) == 0
            except subprocess.TimeoutExpired:
                result['closedAfterEscape'] = False
        result['closeMs'] = round((time.monotonic()-started)*1000,1)
        result['ok'] = result['oneClickClosed'] and result.get('exitCode') == 0
        report.write_text(json.dumps(result)); print(json.dumps(result))
        assert result['oneClickClosed'], 'Titlebar close did not close the app with a select menu open'
        sys.exit(0)
    if os.environ.get('UNZEIT_TEST_UX_ONLY'):
        launches = (root / 'browser.log').read_text().splitlines()
        assert len(launches) == 1, 'HTML browser launch was duplicated'
        location = urlparse(launches[0])
        opened = pathlib.Path(unquote(location.path)) if location.scheme == 'file' else pathlib.Path(launches[0])
        assert opened.resolve() == (root / 'courses' / 'Test course' / '01 Basics' / 'page.html').resolve(), 'HTML did not reach the isolated default browser'
    close_started = time.monotonic()
    try:
        assert process.wait(timeout=5) == 0, 'App exited unsuccessfully'
    except subprocess.TimeoutExpired:
        raise AssertionError('App did not exit within five seconds of closing the player')
    with sqlite3.connect(root / 'app-data' / 'unzeit.sqlite') as db:
        position = db.execute('SELECT position FROM lessons WHERE id=?', [result['closeLessonId']]).fetchone()[0]
        if result.get('closeStudyMinimum'):
            study_seconds = db.execute('SELECT COALESCE(SUM(seconds),0) FROM study_intervals').fetchone()[0]
            assert study_seconds >= result['closeStudyMinimum'], f'Close lost study time: {study_seconds}'
            result['studySeconds'] = study_seconds
        if result.get('transcriptionLessonId'):
            status = db.execute('SELECT status FROM jobs WHERE lesson_id=?', [result['transcriptionLessonId']]).fetchone()[0]
            assert status == 'cancelled', f'Closing left transcription running: {status}'
            assert not list((root / 'app-data' / 'audio').iterdir()), 'Closing left extracted audio behind'
            phases = [json.loads(line) for line in (root / 'shutdown.jsonl').read_text().splitlines()]
            worker = {phase['phase']: phase['timestamp'] for phase in phases}
            elapsed = worker['worker-stop:end'] - worker['worker-stop:begin']
            assert elapsed < 500, f'Model loading delayed shutdown by {elapsed} ms'
            result['workerStopMs'] = elapsed
    assert abs(position - result['closePosition']) < 0.05, f'Close lost progress: {position}'
    result['checks'].append('Player close exits the process and saves the final playback position')
    result['closeElapsedMs'] = round((time.monotonic() - close_started) * 1000, 1)
    report.write_text(json.dumps(result))
    print(json.dumps(result))
finally:
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
VERIFY
