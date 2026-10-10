import { useI18n } from './i18n';
import { useCallback, useEffect, useRef, useState } from 'react';
import { ArrowLeftIcon, ArrowRightIcon, CheckCircleIcon, WarningCircleIcon, CircleIcon } from '@phosphor-icons/react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { call, chooseFile, chooseFolder } from './api';
import { LoopControls } from './LoopControls';
import { Transcript } from './Transcript';
import { Resources } from './Resources';
import { Notes } from './Notes';
import { resumeLesson, time } from './utils';
import type { Course, Lesson, Settings, Progress, LessonTarget } from './types';

interface Props { target: LessonTarget | null; onBack: () => void; course: Course; base: string; settings: Settings; onSettings: (s: Settings) => Promise<void>; onRefresh: () => Promise<void>; onProgress: (value: Progress) => void; onError: (e: unknown) => void }
export function Player({ target, onBack, course, base, settings, onSettings, onRefresh, onProgress, onError }: Props) {
  const { t } = useI18n();
  const [lessonId, setLessonId] = useState(target?.lessonId ?? resumeLesson(course)?.id ?? course.lessons[0]?.id);
  const lesson = course.lessons.find(l => l.id === lessonId);
  const video = useRef<HTMLVideoElement>(null);
  const [caption, setCaption] = useState(lesson?.assets.find(a => a.kind === 'caption')?.id ?? 0);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [playError, setPlayError] = useState(false);
  const [tab, setTab] = useState<'notes' | 'resources' | 'transcript'>('notes');
  const [study, setStudy] = useState(false);
  const loopActive = useRef(false);
  const [repeating, setRepeating] = useState(false);
  const setLoopActive = useCallback((value: boolean) => { loopActive.current = value; setRepeating(value) }, []);
  const [busy, setBusy] = useState(false);
  const position = useRef<number | null>(target?.position ?? null);
  const autoplay = useRef(false);
  const currentIndex = course.lessons.findIndex(l => l.id === lessonId);
  const next = course.lessons.slice(currentIndex + 1).find(l => !l.missing);
  const previous = course.lessons.slice(0, currentIndex).reverse().find(l => !l.missing);
  const save = useCallback(async (value: Progress) => {
    await call('save_progress', { ...value }); onProgress(value);
  }, [onProgress]);
  const persist = useCallback(async (completed?: boolean) => {
    const node = video.current;
    if (lessonId && node && Number.isFinite(node.duration) && node.duration > 0) await save({ id: lessonId, position: node.currentTime, duration: node.duration, completed: completed ?? null });
    else if (lesson && completed !== undefined) await save({ id: lesson.id, position: lesson.position, duration: lesson.duration, completed });
  }, [lessonId, lesson, save]);
  const select = useCallback(async (target: Lesson) => { await persist(); setCountdown(null); setPlayError(false); autoplay.current = true; setLessonId(target.id) }, [persist]);
  const captionIds = lesson?.assets.filter(a => a.kind === 'caption').map(a => a.id).join(',') ?? '';
  const captionLesson = useRef(lessonId);
  useEffect(() => {
    const changedLesson = captionLesson.current !== lessonId;
    captionLesson.current = lessonId;
    const ids = captionIds ? captionIds.split(',').map(Number) : [];
    setCaption(current => changedLesson || (current > 0 && !ids.includes(current)) ? ids[0] ?? 0 : current);
    setPlayError(false);
  }, [lessonId, captionIds]);
  useEffect(() => { if (video.current) video.current.playbackRate = settings.speed }, [settings.speed, lessonId]);
  useEffect(() => {
    const node = video.current; const id = lessonId;
    const flushProgress = async () => { if (node && id && Number.isFinite(node.duration) && node.duration > 0) await save({ id, position: node.currentTime, duration: node.duration, completed: null }) };
    const flush = () => { void flushProgress().catch(onError) };
    const timer = window.setInterval(flush, 5000); window.addEventListener('pagehide', flush);
    const closeListener = isTauri() ? getCurrentWindow().onCloseRequested(async event => {
      try { await flushProgress() } catch (e) { event.preventDefault(); onError(e) }
    }) : null;
    return () => { window.clearInterval(timer); window.removeEventListener('pagehide', flush); flush(); if (closeListener) void closeListener.then(unlisten => unlisten()) };
  }, [lessonId, onError, save]);
  const advance = useRef(() => {});
  advance.current = () => { if (next) void select(next).catch(onError) };
  useEffect(() => {
    if (countdown === null) return;
    if (countdown === 0) { setCountdown(null); advance.current(); return }
    const timer = window.setTimeout(() => setCountdown(n => n === null ? null : n - 1), 1000);
    return () => window.clearTimeout(timer);
  }, [countdown]);
  useEffect(() => {
    function key(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      if (target.closest('input,textarea,select,button,summary,a,[contenteditable=true],.resources,.transcript') || e.ctrlKey || e.metaKey || e.altKey) return;
      const node = video.current; if (!node) return;
      if (e.code === 'Space' || e.key.toLowerCase() === 'k') { e.preventDefault(); if (node.paused) void node.play().catch(onError); else node.pause() }
      else if (e.key.toLowerCase() === 'j' || e.key === 'ArrowLeft') { e.preventDefault(); node.currentTime = Math.max(0, node.currentTime - 10) }
      else if (e.key.toLowerCase() === 'l' || e.key === 'ArrowRight') { e.preventDefault(); node.currentTime = Math.min(node.duration || 0, node.currentTime + 10) }
      else if (e.key.toLowerCase() === 'f') { e.preventDefault(); if (document.fullscreenElement) void document.exitFullscreen().catch(onError); else void node.requestFullscreen().catch(onError) }
    }
    window.addEventListener('keydown', key); return () => window.removeEventListener('keydown', key);
  });
  async function task(action: () => Promise<unknown>) { setBusy(true); try { await action(); await onRefresh() } catch (e) { onError(e) } finally { setBusy(false) } }
  if (!lesson) return <section className="page"><h1>{course.title}</h1><p>{t("No lessons available. Rescan this course or relink its folder.")}</p></section>;
  const captions = lesson.assets.filter(a => a.kind === 'caption');
  const resources = [...lesson.assets.filter(a => a.kind === 'resource'), ...course.resources];
  const sections = [...new Set(course.lessons.map(l => l.section))];
  const remaining = course.lessons.filter(l => !l.missing && !l.assets.some(a => a.kind === 'caption')).map(l => l.id);
  const tools = <><div className="tabs" aria-label={t("Lesson tools")}><button aria-pressed={tab === 'notes'} onClick={() => setTab('notes')}>{t("Notes & bookmarks")}</button><button disabled={!captions.length} aria-pressed={tab === 'transcript'} onClick={() => setTab('transcript')}>{t("Transcript")}</button><button aria-pressed={tab === 'resources'} onClick={() => setTab('resources')}>{t('Resources ({count})', { count: resources.length })}</button></div>
      {tab === 'notes' ? <Notes key={lesson.id} lessonId={lesson.id} position={() => video.current?.currentTime ?? lesson.position} onSeek={p => { if (video.current) video.current.currentTime = p; else position.current = p }} onError={onError} /> : tab === 'transcript' ? captions.length ? <Transcript key={`${lesson.id}-${caption || captions[0].id}`} assetId={caption || captions[0].id} video={video} onSeek={p => { if (video.current) video.current.currentTime = p }} onError={onError} /> : <p className="muted">{t("Generate captions to read and search this lesson.")}</p> : <Resources key={lesson.id} resources={resources} base={base} onError={onError} />}</>;
  const curriculum = <aside className="curriculum"><div className="section-heading"><h2>{t("Curriculum")}</h2><span className="muted">{course.lessons.filter(l => l.completed).length}/{course.lessons.length}</span></div>
      {sections.map(section => <details key={section} open={section === lesson.section}><summary>{section}</summary><ol>{course.lessons.filter(l => l.section === section).map(l => <li key={l.id}><button aria-current={l.id === lesson.id ? 'true' : undefined} onClick={() => void select(l).catch(onError)}><span className="lesson-status" role="img" aria-label={l.completed ? t("Complete") : l.missing ? t("Missing file") : t("Incomplete")}>{l.completed ? <CheckCircleIcon aria-hidden="true" weight="fill" /> : l.missing ? <WarningCircleIcon aria-hidden="true" /> : <CircleIcon aria-hidden="true" />}</span><span>{l.title}<small>{l.missing ? t("Missing file") : l.position > 0 ? `${time(l.position)}${l.duration ? ` / ${time(l.duration)}` : ''}` : t("Not started")}</small></span></button></li>)}</ol></details>)}
      <div className="curriculum-actions"><button disabled={busy || !remaining.length} onClick={() => void task(() => call('enqueue_transcription', { lessonIds: remaining }))}>{t("Caption remaining lessons")}</button><button disabled={busy} onClick={() => void task(async () => { const warnings = await call<string[]>('rescan_course', { id: course.id }); if (warnings.length) onError(warnings.join('\n')) })}>{t("Rescan course")}</button><button disabled={busy} onClick={() => void task(async () => { const path = await chooseFolder(t("Relink course folder")); if (path) await call('relink_course', { id: course.id, path }) })}>{t("Relink folder")}</button></div>
    </aside>;
  return <section className="course-page">
    <button className="course-back quiet" onClick={onBack}><ArrowLeftIcon aria-hidden="true" /> {t("Library")}</button>
    <div className="course-heading"><div><p className="eyebrow">{course.title}</p><h1>{lesson.title}</h1><p className="muted">{t('{section} · Lesson {number} of {count}', { section: lesson.section, number: currentIndex + 1, count: course.lessons.length })}</p></div><div className="course-actions"><button aria-pressed={study} onClick={() => setStudy(value => !value)}>{t("Study mode")}</button></div></div>
    <div className={`learning-layout${study ? ' study-layout' : ''}`}><div className="lesson-column"><div className="lesson-main">
      <div className="video-shell">{lesson.missing ? <div className="video-message"><h2>{t("This file is missing")}</h2><p>{t("Relink the lesson to keep your progress and notes.")}</p><button onClick={() => void task(async () => { const path = await chooseFile(t("Relink lesson"), ['mp4']); if (path) await call('relink_lesson', { id: lesson.id, path }) })}>{t("Relink lesson")}</button></div> : <>
        <video key={lesson.id} ref={video} src={`${base}/media/${lesson.id}`} controls crossOrigin="anonymous" preload="metadata" aria-label={lesson.title}
          onLoadedMetadata={() => { const node = video.current; if (!node) return; node.currentTime = Math.min(position.current ?? lesson.position, Math.max(0, node.duration - 0.1)); position.current = null; node.playbackRate = settings.speed; if (autoplay.current) { autoplay.current = false; void node.play().catch(onError) } }}
          onPause={event => { const node = event.currentTarget; if (Number.isFinite(node.duration) && node.duration > 0) void save({ id: lesson.id, position: node.currentTime, duration: node.duration, completed: null }).catch(onError) }} onEnded={() => { void persist().catch(onError); if (!loopActive.current && settings.autoplay && next) setCountdown(5) }} onError={() => setPlayError(true)}>
          {caption > 0 && <track key={caption} ref={node => { if (node) node.track.mode = 'showing' }} kind="subtitles" src={`${base}/caption/${caption}`} label={captions.find(a => a.id === caption)?.title ?? t("Captions")} srcLang="und" default onError={() => onError(t("Unable to load these captions. Rescan the course and check the subtitle file."))} />}
        </video>
        {playError && <div className="video-message"><h2>{t("Unable to play this video")}</h2><p>{t("The file may be unavailable, incomplete, or use a codec missing from WebKitGTK. Check GStreamer codec packages or relink the file.")}</p></div>}
      </>}</div>
      {countdown !== null && <div className="up-next" role="status">{t('Next: {title} in {seconds}s', { title: next?.title ?? '', seconds: countdown })}{' '}<button onClick={() => setCountdown(null)}>{t("Stay here")}</button></div>}
      <div className="player-toolbar"><label>{t("Speed")}<select aria-label={t("Playback speed")} value={settings.speed} onChange={e => void onSettings({ ...settings, speed: Number(e.target.value) }).catch(onError)}>{[0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3].map(n => <option key={n} value={n}>{n}×</option>)}</select></label><label>{t("Captions")}<select aria-label={t("Subtitle track")} value={caption} onChange={e => setCaption(Number(e.target.value))}><option value={0}>{t("Off")}</option>{captions.map(c => <option key={c.id} value={c.id}>{c.title}</option>)}</select></label>{!captions.length && <button disabled={busy || lesson.missing} onClick={() => void task(() => call('enqueue_transcription', { lessonIds: [lesson.id] }))}>{t("Generate captions")}</button>}<button className={lesson.completed ? '' : 'primary'} disabled={busy} onClick={() => void task(() => persist(!lesson.completed))}>{lesson.completed ? t("Mark incomplete") : t("Mark complete")}</button></div>
      <details key={lesson.id} className="playback-tools"><summary>{t("Repeat interval")}{repeating && <span role="status"> · {t('Repeating')}</span>}</summary><LoopControls video={video} onActive={setLoopActive} onError={onError} /></details>
      <div className="lesson-navigation"><button disabled={!previous} onClick={() => previous && void select(previous).catch(onError)}><ArrowLeftIcon aria-hidden="true" /> {t("Previous lesson")}</button><span className="muted">{t("Space / K · J / L · F")}</span><button disabled={!next} onClick={() => next && void select(next).catch(onError)}>{t("Next lesson")}{' '}<ArrowRightIcon aria-hidden="true" /></button></div>
      {study && <details className="study-curriculum"><summary>{t("Curriculum")}</summary>{curriculum}</details>}
    </div><aside className={study ? 'study-tools' : 'lesson-tools'} aria-label={t("Study panel")}>{tools}</aside></div>
      {!study && curriculum}</div>
  </section>;
}
