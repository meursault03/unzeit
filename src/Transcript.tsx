import { useI18n } from './i18n';
import { useEffect, useRef, useState, type RefObject } from 'react';
import { call } from './api';
import { time } from './utils';
import type { CaptionCue } from './types';

export function Transcript({ assetId, video, onSeek, onError }: { assetId: number; video: RefObject<HTMLVideoElement | null>; onSeek: (position: number) => void; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [cues, setCues] = useState<CaptionCue[]>([]);
  const [query, setQuery] = useState('');
  const [current, setCurrent] = useState(-1);
  const [page, setPage] = useState(0);
  const [follow, setFollow] = useState(true);
  const [loading, setLoading] = useState(true);
  const active = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    let mounted = true; setLoading(true);
    void call<CaptionCue[]>('get_caption_cues', { id: assetId }).then(value => { if (mounted) setCues(value) }).catch(onError).finally(() => { if (mounted) setLoading(false) });
    return () => { mounted = false };
  }, [assetId, onError]);
  useEffect(() => {
    const node = video.current; if (!node) return;
    const update = () => setCurrent(cues.findIndex(c => node.currentTime >= c.position && node.currentTime < c.end));
    update(); node.addEventListener('timeupdate', update); node.addEventListener('seeked', update);
    return () => { node.removeEventListener('timeupdate', update); node.removeEventListener('seeked', update) };
  }, [cues, video]);
  useEffect(() => { if (follow && !query && current >= 0) setPage(Math.floor(current / 100)) }, [current, follow, query]);
  useEffect(() => {
    const node = active.current; const list = node?.closest('.transcript-cues');
    if (!follow || query || !node || !list) return;
    const row = node.getBoundingClientRect(); const frame = list.getBoundingClientRect();
    if (row.top < frame.top) list.scrollTop += row.top - frame.top;
    else if (row.bottom > frame.bottom) list.scrollTop += row.bottom - frame.bottom;
  }, [current, page, follow, query]);
  const matches = cues.map((cue,index) => ({ cue,index })).filter(({ cue }) => cue.text.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const pages = Math.max(1, Math.ceil(matches.length / 100));
  return <section className="transcript"><h2>{t("Transcript")}</h2><div className="field-row"><input type="search" aria-label={t("Search this transcript")} placeholder={t("Find a phrase…")} value={query} onChange={e => { setQuery(e.target.value); setPage(0) }} /><label className="check"><input type="checkbox" checked={follow} onChange={e => setFollow(e.target.checked)} /> {t("Follow playback")}</label></div>
    {loading ? <p role="status">{t("Loading transcript…")}</p> : !matches.length ? <p className="muted">{t("No matching caption text.")}</p> : <ol className="transcript-cues">{matches.slice(page * 100,(page + 1) * 100).map(({ cue,index }) => <li key={index}><button ref={index === current ? active : undefined} aria-current={index === current ? 'true' : undefined} onClick={() => onSeek(cue.position)}><span className="timestamp">{time(cue.position)}</span><span>{cue.text}</span></button></li>)}</ol>}
    {pages > 1 && <div className="transcript-pages"><button disabled={page <= 0} onClick={() => { setFollow(false); setPage(n => n - 1) }}>{t("Previous page")}</button><span>{page + 1} / {pages}</span><button disabled={page >= pages - 1} onClick={() => { setFollow(false); setPage(n => n + 1) }}>{t("Next page")}</button></div>}
  </section>;
}
