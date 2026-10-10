import { cp, mkdir, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const source = fileURLToPath(new URL('../node_modules/pdfjs-dist/', import.meta.url));
const destination = fileURLToPath(new URL('../public/pdfjs/', import.meta.url));
await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
for (const name of ['cmaps', 'standard_fonts', 'iccs', 'LICENSE']) {
  await cp(`${source}/${name}`, `${destination}/${name}`, { recursive: true });
}
