import { useI18n } from './i18n';
import { lazy, Suspense, useEffect, useState } from 'react';
import { ArrowSquareOutIcon, FileIcon } from '@phosphor-icons/react';
import { CodePreview } from './CodePreview';
import { call } from './api';
import type { Asset, ResourceInfo } from './types';

const PdfPreview = lazy(() => import('./PdfPreview'));
export function Resources({ resources, base, onError }: { resources: Asset[]; base: string; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [selected, setSelected] = useState<number | null>(null);
  const asset = resources.find(r => r.id === selected);
  return <section className="resources"><h2>{t("Resources")}</h2>{!resources.length ? <p className="muted">{t("No attached resources for this lesson.")}</p> : <><div className="resource-list">{resources.map(r => <button key={r.id} aria-pressed={r.id === selected} onClick={() => setSelected(r.id)}><FileIcon aria-hidden="true" /><span>{r.title}</span></button>)}</div>{asset ? <ResourcePreview key={asset.id} asset={asset} base={base} onError={onError} /> : <p className="resource-hint muted">{t("Choose a file to preview it here.")}</p>}</>}</section>;
}
function ResourcePreview({ asset, base, onError }: { asset: Asset; base: string; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [info, setInfo] = useState<ResourceInfo | null>(null);
  const [text, setText] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    void (async () => {
      const value = await call<ResourceInfo>('get_resource', { id: asset.id });
      const content = value.preview === 'text' ? await call<string>('read_resource_text', { id: asset.id }) : '';
      if (active) { setText(content); setInfo(value) }
    })().catch(e => { if (active) setError(String(e)) });
    return () => { active = false };
  }, [asset.id]);
  return <article className="resource-preview"><div className="section-heading"><div><h3>{asset.title}</h3>{info && <small className="muted">{info.size < 1024 * 1024 ? `${Math.ceil(info.size / 1024)} KiB` : `${(info.size / (1024 * 1024)).toFixed(1)} MiB`}</small>}</div><button onClick={() => void call('open_resource', { id: asset.id }).catch(onError)}><ArrowSquareOutIcon aria-hidden="true" /> {info?.preview === 'external' ? t("Open or reveal") : t("Open externally")}</button></div>
    {error ? <p className="error-text" role="alert">{t(error)}</p> : !info ? <p role="status">{t("Opening resource…")}</p> : info.preview === 'pdf' ? <Suspense fallback={<p role="status">{t("Loading PDF viewer…")}</p>}><PdfPreview url={`${base}/resource/${asset.id}`} title={asset.title} /></Suspense> : info.preview === 'image' ? <ImagePreview url={`${base}/resource/${asset.id}`} title={asset.title} /> : info.preview === 'text' ? <CodePreview text={text} title={asset.title} /> : <p className="muted">{t("Preview unavailable for this format or size. Active formats are revealed in their folder.")}</p>}
  </article>;
}

function ImagePreview({ url, title }: { url: string; title: string }) {
  const { t } = useI18n();
  const [source, setSource] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    const controller = new AbortController(); const reader = new FileReader();
    reader.onload = () => setSource(String(reader.result));
    reader.onerror = () => setError(t("Unable to read this image. Try opening it externally."));
    void fetch(url, { signal: controller.signal }).then(async response => {
      if (!response.ok) throw new Error(t("This image is unavailable. Rescan the course."));
      const blob = await response.blob();
      if (blob.size > 16 * 1024 * 1024) throw new Error(t("Image preview is limited to 16 MiB."));
      if (!controller.signal.aborted) reader.readAsDataURL(blob);
    }).catch(e => { if (!controller.signal.aborted) setError(String(e)) });
    return () => { controller.abort(); reader.abort() };
  }, [url]);
  return error ? <p className="error-text" role="alert">{t(error)}</p> : source ? <img className="resource-image" src={source} alt={title} onError={() => setError(t("Unable to decode this image. Try opening it externally."))} /> : <p role="status">{t("Loading image…")}</p>;
}
