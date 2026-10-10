import { useI18n } from './i18n';
import { useEffect, useState } from 'react';
import { BooksIcon, NoteIcon, PlayCircleIcon, MagnifyingGlassIcon, ArrowUpRightIcon } from '@phosphor-icons/react';
import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { call } from './api';
import { time } from './utils';
import type { SearchResult } from './types';

export function Search({ onOpen }: { onOpen: (result: SearchResult) => void }) {
  const { t, count: countLabel } = useI18n();
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!isTauri()) return;
    const sub = listen('captions-indexed', () => setRevision(n => n + 1));
    const jobs = listen<{ status: string }>('transcription-progress', e => { if (e.payload.status === 'completed') setRevision(n => n + 1) });
    return () => { void sub.then(f => f()); void jobs.then(f => f()) };
  }, []);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    setPending(Boolean(query.trim())); setError(''); setResults([]);
    const timer = window.setTimeout(() => {
      if (!query.trim() || !isTauri()) { setPending(false); return }
      void call<SearchResult[]>('search_library', { query }).then(value => { if (active) setResults(value) })
        .catch(e => { if (active) setError(String(e)) }).finally(() => { if (active) setPending(false) });
    }, 200);
    return () => { active = false; window.clearTimeout(timer) };
  }, [query, revision]);
  return <section className="page search-page"><h1>{t("Search")}</h1><p className="intro">{t("Find lessons, notes, and spoken content.")}</p>
    <label className="search-field"><MagnifyingGlassIcon aria-hidden="true" /><input type="search" aria-label={t("Search courses, lessons, notes, and transcripts")} maxLength={256} value={query} onChange={e => setQuery(e.target.value)} placeholder={t("Search your library…")} /></label>
    <p className="search-status muted" role="status">{error || (pending ? t("Searching…") : !query.trim() ? t("Search titles, notes, and spoken content.") : results.length === 100 ? t("First 100 results") : countLabel(results.length, 'result'))}</p>
    {!pending && query.trim() && !error && !results.length && <div className="empty"><h2>{t("No matches")}</h2><p>{t("Try a shorter word or a different phrase.")}</p></div>}
    <div className="search-results" aria-busy={pending}>{(['course', 'lesson', 'note', 'caption'] as const).map(kind => {
      const matches = results.filter(r => r.kind === kind);
      return matches.length > 0 && <section key={kind}><h2>{kind === 'course' ? t("Courses") : kind === 'lesson' ? t("Lessons") : kind === 'note' ? t("Notes") : t("Transcripts")} <small className="muted">{matches.length}</small></h2>{matches.map(result => <button key={`${result.kind}-${result.id}`} className="search-result" onClick={() => onOpen(result)}>
      {result.kind === 'course' ? <BooksIcon aria-hidden="true" /> : result.kind === 'lesson' ? <PlayCircleIcon aria-hidden="true" /> : <NoteIcon aria-hidden="true" />}
      <span><small className="muted">{(result.kind === 'note' || result.kind === 'caption') ? `${result.kind === 'note' ? t("Note") : t("Transcript")} · ${time(result.position ?? 0)}` : result.kind === 'course' ? t("Course") : t("Lesson")}{result.missing ? ` · ${t('Missing file')}` : ''}</small><strong>{result.kind === 'course' ? result.courseTitle : result.lessonTitle}</strong>{(result.kind === 'note' || result.kind === 'caption') && <span className="search-snippet">{result.snippet}</span>}{result.kind !== 'course' && <small className="muted">{result.courseTitle}</small>}</span><ArrowUpRightIcon aria-hidden="true" />
    </button>)}</section>;
    })}</div>
  </section>;
}
