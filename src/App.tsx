import { LanguageContext, systemLanguage, translator } from './i18n';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { BooksIcon, PlusIcon, WaveformIcon, GearSixIcon, ArrowUpRightIcon, ArrowRightIcon, XIcon, HardDrivesIcon, MagnifyingGlassIcon } from '@phosphor-icons/react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { call } from './api';
import { Importer } from './Importer';
import { Jobs } from './Jobs';
import { Search } from './Search';
import { Player } from './Player';
import { Preferences } from './Preferences';
import { flushPendingStudy } from './study-persistence';
import { StudyRoutine } from './StudyRoutine';
import { applyProgress, completion, resumeLesson, time } from './utils';
import type { Course, Job, Settings, Progress, LessonTarget } from './types';

const defaults: Settings = { libraryRoot: '', modelPath: '', speed: 1, autoplay: true, theme: 'system', language: systemLanguage(), weeklyGoalMinutes: 0 };
export function App() {
  const [page, setPage] = useState<'library' | 'import' | 'jobs' | 'settings' | 'search'>('library');
  const [courses, setCourses] = useState<Course[]>([]);
  const [courseId, setCourseId] = useState<number | null>(null);
  const [target, setTarget] = useState<LessonTarget | null>(null);
  const [settings, setSettings] = useState(defaults);
  const { t, count: countLabel } = useMemo(() => translator(settings.language), [settings.language]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [base, setBase] = useState('');
  const [query, setQuery] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const course = courses.find(c => c.id === courseId);
  const main = useRef<HTMLElement>(null);
  const fail = useCallback((e: unknown) => setError(t(e instanceof Error ? e.message : String(e))), [t]);
  const onProgress = useCallback((value: Progress) => setCourses(current => applyProgress(current, value)), []);
  const refreshJobs = useCallback(() => call<Job[]>('get_jobs').then(setJobs), []);
  const refresh = useCallback(async () => { const [library, prefs, queue] = await Promise.all([call<Course[]>('get_library'), call<Settings>('get_settings', { defaultLanguage: defaults.language }), call<Job[]>('get_jobs')]); setCourses(library); setSettings(prefs); setJobs(queue) }, []);
  useEffect(() => {
    if (!isTauri()) { setLoading(false); return }
    void Promise.all([refresh(), call<string>('media_base').then(setBase)]).catch(fail).finally(() => setLoading(false));
    const timer = window.setInterval(() => void refreshJobs().catch(fail), 2500);
    const subscription = listen<Job>('transcription-progress', event => { setJobs(current => [event.payload, ...current.filter(j => j.id !== event.payload.id)].sort((a, b) => b.id - a.id)); if (event.payload.status === 'completed') void refresh().catch(fail) });
    return () => { window.clearInterval(timer); void subscription.then(unlisten => unlisten()) };
  }, [refresh, refreshJobs, fail]);
  useEffect(() => { document.documentElement.dataset.theme = settings.theme; document.documentElement.lang = settings.language }, [settings.theme, settings.language]);
  useEffect(() => {
    if (!isTauri() || course) return;
    const listener = getCurrentWindow().onCloseRequested(async event => {
      try { await flushPendingStudy() } catch (e) { event.preventDefault(); fail(e) }
    });
    return () => { void listener.then(unlisten => unlisten()) };
  }, [fail, course?.id]);
  useEffect(() => { const heading = main.current?.querySelector<HTMLElement>('h1'); heading?.setAttribute('tabindex', '-1'); heading?.focus() }, [page, courseId, loading]);
  async function saveSettings(value: Settings) { await call('save_settings', { settings: value }); setSettings(value) }
  function navigate(next: typeof page) { setCourseId(null); setTarget(null); setPage(next); setError('') }
  function openCourse(id: number, destination: LessonTarget | null = null) { setTarget(destination); setCourseId(id); setPage('library'); setError('') }
  const activeJobs = jobs.filter(j => ['queued', 'running'].includes(j.status)).length;
  const continuing = courses.find(c => c.updatedAt > 0 && resumeLesson(c));
  const filtered = courses.filter(c => c.title.toLowerCase().includes(query.toLowerCase()));
  return <LanguageContext.Provider value={settings.language}><div className="app-shell"><a className="skip-link" href="#main">{t("Skip to content")}</a><aside className="app-sidebar"><button className="wordmark" onClick={() => navigate('library')} aria-label={t("Unzeit library")}>unzeit<span>{t("Offline courses")}</span></button><nav aria-label={t("Main navigation")}><button aria-current={page === 'library' ? 'page' : undefined} onClick={() => navigate('library')}><BooksIcon aria-hidden="true" /> {t("Library")}</button><button aria-current={page === 'search' ? 'page' : undefined} onClick={() => navigate('search')}><MagnifyingGlassIcon aria-hidden="true" /> {t("Search")}</button><button aria-current={page === 'import' ? 'page' : undefined} onClick={() => navigate('import')}><PlusIcon aria-hidden="true" /> {t("Import courses")}</button><button aria-current={page === 'jobs' ? 'page' : undefined} onClick={() => navigate('jobs')}><WaveformIcon aria-hidden="true" /> {t("Transcription")}{' '}{activeJobs > 0 && <span className="count">{activeJobs}</span>}</button><button aria-current={page === 'settings' ? 'page' : undefined} onClick={() => navigate('settings')}><GearSixIcon aria-hidden="true" /> {t("Settings")}</button></nav><div className="sidebar-footer"><HardDrivesIcon aria-hidden="true" /> {t("Stored on this device")}</div></aside>
    <main id="main" ref={main}>{error && <div className="error-banner" role="alert"><p>{error}</p><button aria-label={t("Dismiss error")} onClick={() => setError('')}><XIcon aria-hidden="true" /></button></div>}
      {!isTauri() && <div className="preview-banner">{t('Desktop preview · Run')}{' '}<code>npm run tauri dev</code> {t("to connect your local library.")}</div>}
      {loading ? <section className="page" aria-busy="true"><h1>{t("Opening your library…")}</h1><div className="loading-line" /></section> : course ? <Player key={course.id} course={course} target={target} onBack={() => navigate('library')} base={base} settings={settings} onSettings={saveSettings} onRefresh={refresh} onProgress={onProgress} onError={fail} /> : page === 'search' ? <Search onOpen={result => openCourse(result.courseId, { lessonId: result.lessonId, position: result.position })} /> : page === 'import' ? <Importer onCancel={() => navigate('library')} settings={settings} onImported={async () => { await refresh(); navigate('library') }} onError={fail} /> : page === 'settings' ? <Preferences onModelImported={modelPath => setSettings(current => ({ ...current, modelPath }))} settings={settings} onSave={saveSettings} onError={fail} /> : page === 'jobs' ? <Jobs jobs={jobs} onRefresh={refreshJobs} onError={fail} /> : <section className="page library-page"><div className="section-heading"><div><h1>{t("Library")}</h1><p className="intro">{t("Your courses, notes, and progress.")}</p></div><button className="primary" onClick={() => navigate('import')}><PlusIcon aria-hidden="true" /> {t("Import courses")}</button></div>
        <StudyRoutine settings={settings} onSave={saveSettings} />
        {continuing && <article className="continue-card"><div><p className="eyebrow">{t("Continue watching")}</p><h2>{continuing.title}</h2><p>{resumeLesson(continuing)?.title} · {time(resumeLesson(continuing)?.position ?? 0)}</p></div><button className="primary" onClick={() => openCourse(continuing.id)}>{t("Continue learning")}{' '}<ArrowRightIcon aria-hidden="true" /></button></article>}
        <div className="library-tools"><h2>{countLabel(courses.length, 'course')}</h2><input aria-label={t("Filter course titles")} type="search" value={query} onChange={e => setQuery(e.target.value)} placeholder={t("Filter courses…")} /></div>
        {!courses.length ? <div className="empty library-empty"><BooksIcon className="empty-icon" aria-hidden="true" /><h2>{t("No courses yet")}</h2><p>{t("Choose the folder containing your downloaded courses.")}<br />{t("Review the lessons before importing.")}</p><button className="primary" onClick={() => navigate('import')}>{t("Choose your library")}</button><p className="muted">{t("Your files stay exactly where they are.")}</p></div> : !filtered.length ? <div className="empty"><p>{t('No courses match “{query}”.', { query })}</p><button onClick={() => setQuery('')}>{t("Clear search")}</button></div> : <div className="course-list">{filtered.map((c, i) => <button className="course-card" key={c.id} onClick={() => openCourse(c.id)}><span className="course-number" aria-hidden="true">{String(i + 1).padStart(2, '0')}</span><span className="course-card-body"><span className="course-card-title">{c.title}</span><span className="muted">{countLabel(new Set(c.lessons.map(l => l.section)).size, 'section')} · {countLabel(c.lessons.length, 'lesson')}{c.lessons.some(l => l.missing) ? ` · ${t('Missing files')}` : ''}</span><progress value={completion(c)} max={100} aria-label={t('{course} completion', { course: c.title })} /></span><span className="course-completion">{completion(c)}%<small>{t("complete")}</small></span><ArrowUpRightIcon aria-hidden="true" /></button>)}</div>}
      </section>}
    </main>
  </div></LanguageContext.Provider>;
}
