import { useI18n } from './i18n';
import { useEffect, useState, type RefObject } from 'react';
import { RepeatIcon } from '@phosphor-icons/react';
import { time, validLoop } from './utils';

export function LoopControls({ video, onActive, onError }: { video: RefObject<HTMLVideoElement | null>; onActive: (active: boolean) => void; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [start, setStart] = useState<number | null>(null);
  const [end, setEnd] = useState<number | null>(null);
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    onActive(enabled);
    const node = video.current;
    if (!node || !enabled || start === null || end === null) return () => onActive(false);
    const repeat = () => { if (node.currentTime >= end || node.currentTime < start) node.currentTime = start };
    const restart = () => { node.currentTime = start; void node.play().catch(onError) };
    repeat(); node.addEventListener('timeupdate',repeat); node.addEventListener('seeked',repeat); node.addEventListener('ended',restart);
    return () => { onActive(false); node.removeEventListener('timeupdate',repeat); node.removeEventListener('seeked',repeat); node.removeEventListener('ended',restart) };
  }, [enabled, start, end, video, onActive, onError]);
  function markEnd() {
    const point = video.current?.currentTime ?? 0;
    if (start === null || !validLoop(start, point)) { onError(t("The repeat interval must be at least half a second.")); return }
    setEnd(point); setEnabled(true);
  }
  return <div className="loop-controls" aria-label={t("Repeat video interval")}><RepeatIcon aria-hidden="true" /><button aria-label={t("Mark repeat start")} onClick={() => { setStart(video.current?.currentTime ?? 0); setEnd(null); setEnabled(false) }}>A{' '}{start === null ? '—' : time(start)}</button><button aria-label={t("Mark repeat end")} disabled={start === null} onClick={markEnd}>B{' '}{end === null ? '—' : time(end)}</button><button disabled={end === null} aria-pressed={enabled} onClick={() => setEnabled(value => !value)}>{enabled ? t("Stop repeating") : t("Repeat interval")}</button><button className="quiet" disabled={start === null} onClick={() => { setEnabled(false); setStart(null); setEnd(null) }}>{t("Clear")}</button></div>;
}
