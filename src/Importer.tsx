import { useI18n } from './i18n';
import { useState } from 'react';
import { call, chooseFolder } from './api';
import type { Preview, Settings } from './types';

interface Props { settings: Settings; onImported: () => Promise<void>; onError: (e: unknown) => void; onCancel: () => void }
export function Importer({ settings, onImported, onError, onCancel }: Props) {
  const { t, count: countLabel } = useI18n();
  const [root, setRoot] = useState(settings.libraryRoot);
  const [courses, setCourses] = useState<Preview[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [scanned, setScanned] = useState(false);
  async function browse() {
    setBusy(true);
    try {
      const path = await chooseFolder(t("Choose library root"), root);
      if (path) { setRoot(path); setCourses([]); setSelected(new Set()); setScanned(false) }
    } catch (e) { onError(e) } finally { setBusy(false) }
  }
  async function scan() {
    setBusy(true); setScanned(false); setCourses([]); setSelected(new Set());
    try { const result = await call<Preview[]>('scan_library', { root }); setCourses(result); setSelected(new Set(result.filter(c => c.sections.length).map(c => c.path))); setScanned(true) }
    catch (e) { onError(e) } finally { setBusy(false) }
  }
  async function addFolder(replace?: string) {
    setBusy(true);
    try {
      const path = await chooseFolder(t("Choose a course folder inside the library"), replace || root); if (!path) return;
      const preview = await call<Preview>('preview_course', { root, path });
      setCourses(current => [...current.filter(c => c.path !== replace && c.path !== preview.path), preview]);
      setSelected(current => { const next = new Set(current); if (replace) next.delete(replace); next.delete(preview.path); if (preview.sections.length) next.add(preview.path); return next });
      setScanned(true);
    } catch (e) { onError(e) } finally { setBusy(false) }
  }
  async function importSelected() {
    setBusy(true);
    try {
      const paths = [...selected];
      if (paths.some(a => paths.some(b => a !== b && a.startsWith(`${b}/`)))) throw new Error(t("Select either a collection folder or its individual courses, not both."));
      await call('import_courses', { root, paths }); await onImported();
    } catch (e) { onError(e) } finally { setBusy(false) }
  }
  return <section className="page" aria-busy={busy}>
    <div className="section-heading"><h1>{t("Import courses")}</h1><button disabled={busy} onClick={onCancel}>{t("Back to library")}</button></div>
    <p className="intro">{t("Choose the folder containing your courses. Preview the curriculum before adding anything.")}</p>
    <div className="panel import-root"><label htmlFor="library-root">{t("Library root")}</label><div className="field-row"><input id="library-root" value={root} onChange={e => { setRoot(e.target.value); setCourses([]); setSelected(new Set()); setScanned(false) }} placeholder="/home/you/Courses" disabled={busy} /><button disabled={busy} onClick={() => void browse()}>{t("Browse")}</button><button className="primary" disabled={busy || !root.trim()} onClick={() => void scan()}>{busy ? t("Working…") : t("Scan library")}</button></div></div>
    {scanned && <><div className="section-heading"><h2>{t("Import preview")}</h2><button disabled={busy} onClick={() => void addFolder()}>{t("Add individual course")}</button></div>
      <p className="muted">{t("For a wrapper folder, choose its inner course. For a multi-course pack, deselect the pack and add its courses individually.")}</p>
      {!courses.length && <div className="empty">{t("No courses found. Add an individual course folder, or choose another library root.")}</div>}
      {courses.map(course => <article key={course.path} className="panel preview"><div className="field-row"><input type="checkbox" aria-label={t('Import {course}', { course: course.title })} checked={selected.has(course.path)} disabled={busy || !course.sections.length} onChange={e => setSelected(current => { const next = new Set(current); if (e.target.checked) next.add(course.path); else next.delete(course.path); return next })} /><h3>{course.title}</h3><button className="push" disabled={busy} onClick={() => void addFolder(course.path)}>{t("Choose inner folder")}</button></div><p className="path">{course.path}</p>
        <p className="muted">{countLabel(course.sections.length, 'section')} · {countLabel(course.sections.reduce((n, s) => n + s.lessons.length, 0), 'lesson')} · {countLabel(course.resources.length + course.sections.reduce((n, s) => n + s.lessons.reduce((m, l) => m + l.resources.length, 0), 0), 'resource')}</p>
        <details><summary>{t("Review curriculum and subtitle matches")}</summary>{course.sections.map(section => <div key={section.path}><h4>{section.title}</h4><ol>{section.lessons.map(lesson => <li key={lesson.path}>{lesson.title}<span className="muted">{lesson.captions.length ? ` — ${lesson.captions.map(c => c.title).join(', ')}` : t(" — no captions")}</span></li>)}</ol></div>)}</details>
        {course.warnings.length > 0 && <details className="warnings"><summary>{countLabel(course.warnings.length, 'import warning')}</summary><ul>{course.warnings.map((w, i) => <li key={i}>{w}</li>)}</ul></details>}
      </article>)}
      {!!courses.length && !selected.size && <p className="muted" role="status">{t("Select at least one course to import.")}</p>}
      <button className="primary" disabled={busy || !selected.size} onClick={() => void importSelected()}>{t(selected.size === 1 ? 'Import {count} course' : 'Import {count} courses', { count: selected.size })}</button>
    </>}
  </section>;
}
