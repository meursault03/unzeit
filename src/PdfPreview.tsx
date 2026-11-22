import { Select } from './Select';
import { useI18n } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { ArrowLeftIcon, ArrowRightIcon } from '@phosphor-icons/react';
import { getDocument, GlobalWorkerOptions, PDFWorker, TextLayer, type PDFDocumentProxy } from 'pdfjs-dist/legacy/build/pdf.mjs';
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.mjs?url';
import PdfWorker from 'pdfjs-dist/legacy/build/pdf.worker.mjs?worker';
import './pdf-text-layer.css';

GlobalWorkerOptions.workerSrc = workerUrl;

export default function PdfPreview({ url, title, initialPage = 1 }: { url: string; title: string; initialPage?: number }) {
  const { t } = useI18n();
  const [document, setDocument] = useState<PDFDocumentProxy | null>(null);
  const [page, setPage] = useState(initialPage);
  const [zoom, setZoom] = useState(1);
  const [error, setError] = useState('');
  const [rendering, setRendering] = useState(true);
  const canvas = useRef<HTMLCanvasElement>(null);
  const layer = useRef<HTMLDivElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(600);
  useEffect(() => {
    const node = frame.current; if (!node) return;
    const observer = new ResizeObserver(() => setWidth(node.clientWidth));
    observer.observe(node); return () => observer.disconnect();
  }, []);
  useEffect(() => {
    let active = true;
    let port: Worker | undefined;
    let worker: PDFWorker | undefined;
    let loading: ReturnType<typeof getDocument> | undefined;
    setError(''); setDocument(null); setRendering(true);
    const assets = new URL('/pdfjs/', window.location.href).href;
    try {
      // A departing preview can still be closing while the next PDF opens.
      port = new PdfWorker();
      worker = PDFWorker.create({ port });
    } catch { port?.terminate() }
    void (async () => {
      // Render passive page content only; never process scripts, XFA, or links.
      loading = getDocument({ url, worker, cMapUrl: `${assets}cmaps/`, cMapPacked: true, standardFontDataUrl: `${assets}standard_fonts/`, wasmUrl: `${assets}wasm/`, iccUrl: `${assets}iccs/`, enableXfa: false, useWasm: false, maxImageSize: 16_777_216, disableStream: true, disableAutoFetch: true });
      const value = await loading.promise;
      if (!active) return;
      setPage(Math.min(initialPage, value.numPages));
      setDocument(value);
    })().catch(e => { if (active) { setError(String(e)); setRendering(false) } });
    return () => {
      active = false;
      void (async () => {
        try { await loading?.destroy() }
        finally { worker?.destroy(); port?.terminate() }
      })().catch(() => {});
    };
  }, [url, initialPage]);
  useEffect(() => {
    if (!document || !canvas.current || !layer.current) return;
    let active = true;
    let render: ReturnType<Awaited<ReturnType<PDFDocumentProxy['getPage']>>['render']> | undefined;
    let text: TextLayer | undefined;
    const node = canvas.current; const textNode = layer.current;
    setRendering(true); setError('');
    void (async () => {
      const pdfPage = await document.getPage(page); if (!active) return;
      const natural = pdfPage.getViewport({ scale: 1 });
      const scale = Math.min(Math.max(1, width - 32) / natural.width * zoom, 3, Math.sqrt(4_000_000 / (natural.width * natural.height)));
      const viewport = pdfPage.getViewport({ scale });
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      node.width = Math.ceil(viewport.width * ratio); node.height = Math.ceil(viewport.height * ratio);
      node.style.width = `${viewport.width}px`; node.style.height = `${viewport.height}px`;
      textNode.replaceChildren(); textNode.style.width = `${viewport.width}px`; textNode.style.height = `${viewport.height}px`;
      textNode.style.setProperty('--total-scale-factor', String(scale));
      const textContentSource = await pdfPage.getTextContent();
      if (!active) return;
      text = new TextLayer({ textContentSource, container: textNode, viewport });
      render = pdfPage.render({ canvas: node, viewport, transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0] });
      await Promise.all([render.promise, text.render()]);
      if (active) setRendering(false);
    })().catch(e => { if (active) { setError(String(e)); setRendering(false) } });
    return () => { active = false; render?.cancel(); text?.cancel() };
  }, [document, page, zoom, width]);
  return <div className="pdf-preview">
    <div className="pdf-toolbar"><button aria-label={t("Previous PDF page")} disabled={!document || page <= 1} onClick={() => setPage(n => n - 1)}><ArrowLeftIcon aria-hidden="true" /></button><span>{t('Page {page} of {count}', { page, count: document?.numPages ?? '…' })}</span><button aria-label={t("Next PDF page")} disabled={!document || page >= document.numPages} onClick={() => setPage(n => n + 1)}><ArrowRightIcon aria-hidden="true" /></button><label className="push">{t("Zoom")}<Select label={t('PDF zoom')} value={zoom} onChange={value => setZoom(Number(value))} options={[1, 1.25, 1.5, 2].map(n => ({ value: n, label: n === 1 ? t('Fit width') : `${n * 100}%` }))} /></label></div>
    <p className={error ? 'error-text' : 'muted'} role="status">{error || (rendering ? t("Rendering page…") : '')}</p>
    <div className="pdf-frame" tabIndex={0} aria-label={t("PDF page scroll area")} ref={frame} aria-busy={rendering}><div className="pdf-page"><canvas ref={canvas} aria-label={t('{title}, page {page}', { title, page })} /><div ref={layer} className="textLayer" role="document" aria-label={t('Page {page} text', { page })} /></div></div>
  </div>;
}
