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
UNZEIT_TEST_DIR="$fixture_root" src-tauri/target/release/unzeit
cat "$fixture_root/webview-result.json"
