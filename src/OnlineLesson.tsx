import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { ArrowSquareOutIcon, PlayIcon } from '@phosphor-icons/react';
import { call } from './api';
import { useI18n } from './i18n';
import { Select } from './Select';
import type { Lesson, Progress } from './types';

let nextSession = 0;
let pending = Promise.resolve();
function playerCommand(command: string, args: Record<string, unknown>) {
  const result = pending.then(() => call<void>(command, args));
  pending = result.catch(() => {});
  return result;
}

export function OnlineLesson({ lesson, busy, onSource, onProgress, children }: {
  lesson: Lesson; busy: boolean; onSource: () => void; onProgress: (progress: Progress) => void; children: ReactNode;
}) {
  const { t } = useI18n();
  const [index, setIndex] = useState(0);
  const [session, setSession] = useState<number | null>(null);
  const [startedSession, setStartedSession] = useState<number | null>(null);
  const [loadedSession, setLoadedSession] = useState<number | null>(null);
  const [error, setError] = useState(false);
  const slot = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (session === null) return;
    let listening = true;
    function fail() { setError(true); setSession(null) }
    const handlers = [
      listen<number>('online-video-failed', event => { if (listening && event.payload === session) fail() }),
      listen<number>('online-video-loaded', event => { if (listening && event.payload === session) setLoadedSession(session) }),
    ];
    return () => {
      listening = false;
      for (const handler of handlers) void handler.then(stop => stop()).catch(() => {});
    };
  }, [session]);

  useEffect(() => {
    if (session === null || startedSession !== session || loadedSession === session) return;
    const timeout = window.setTimeout(() => { setError(true); setSession(null) }, 20000);
    return () => window.clearTimeout(timeout);
  }, [loadedSession, session, startedSession]);

  useLayoutEffect(() => {
    const node = slot.current;
    if (!node || session === null) return;
    let active = true;
    let geometry: string | null | undefined;
    let opened = false;
    let waiting = false;
    let frame = 0;
    node.scrollIntoView({ block: 'nearest' });
    function update() {
      if (!node || !active || waiting) return;
      const { left, top, width, height, right, bottom } = node.getBoundingClientRect();
      const fits = !document.hidden && left >= -1 && top >= -1 && right <= innerWidth + 1 && bottom <= innerHeight + 1 && width >= 200 && height >= 200;
      const bounds = fits ? { x: Math.max(0, Math.round(left)), y: Math.max(0, Math.round(top)), width: Math.round(width), height: Math.round(height) } : null;
      const nextGeometry = bounds ? JSON.stringify(bounds) : null;
      if ((!opened && !bounds) || nextGeometry === geometry) return;
      const first = !opened;
      opened = true;
      geometry = nextGeometry;
      waiting = true;
      void (first ? playerCommand('start_online_video', { id: lesson.id, index, session, bounds }) : playerCommand('position_online_video', { bounds }))
        .then(() => {
          if (active && first) {
            setStartedSession(session);
            onProgress({ id: lesson.id, position: 0, duration: 0, completed: null });
          }
        })
        .catch(() => { if (active) { active = false; setError(true); setSession(null) } })
        .finally(() => { waiting = false; if (document.hidden) update() });
    }
    function trackLayout() {
      update();
      if (active) frame = requestAnimationFrame(trackLayout);
    }
    function keepVisibleOnResize() {
      if (geometry) node?.scrollIntoView({ block: 'nearest' });
      update();
    }
    document.addEventListener('visibilitychange', update);
    window.addEventListener('resize', keepVisibleOnResize);
    frame = requestAnimationFrame(trackLayout);
    return () => {
      active = false;
      cancelAnimationFrame(frame);
      document.removeEventListener('visibilitychange', update);
      window.removeEventListener('resize', keepVisibleOnResize);
      if (opened) void playerCommand('close_online_video', {}).catch(() => {});
    };
  }, [index, lesson.id, onProgress, session]);

  const embedded = lesson.videos.length > 0 && !lesson.missing;
  return <>
    <div className="panel field-row linked-lesson-actions">
      {embedded && <button className="primary" onClick={() => { setError(false); setSession(session === null ? ++nextSession : null) }}>
        {session === null && <PlayIcon aria-hidden="true" />} {t(session === null ? "Play in Unzeit" : "Close video")}
      </button>}
      {embedded && lesson.videos.length > 1 && <Select openAbove label={t("Lesson video")} value={index}
        onChange={value => { setIndex(Number(value)); setError(false); if (session !== null) setSession(++nextSession) }}
        options={lesson.videos.map((video, value) => ({ value, label: video.title ?? t('Video {number}', { number: value + 1 }) }))} />}
      <button className={embedded ? '' : 'primary'} disabled={busy || lesson.missing} onClick={() => { setSession(null); onSource() }}><ArrowSquareOutIcon aria-hidden="true" /> {t("Open lesson in browser")}</button>
      {lesson.missing && <span className="muted">{t("Lesson link is unavailable")}</span>}
      {children}
    </div>
    {session !== null && <div className="online-video-slot" ref={slot} role="region" aria-label={t("Lesson video")} />}
    {session !== null && loadedSession !== session && <p className="muted" role="status">{t("Loading YouTube player…")}</p>}
    {error && <p className="error-text" role="alert">{t("Unable to load the embedded video. Open the lesson in your browser.")}</p>}
  </>;
}
