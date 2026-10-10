import { useI18n } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { call } from './api';
import type { Job } from './types';
export function Jobs({ jobs, onRefresh, onError }: { jobs: Job[]; onRefresh: () => Promise<void>; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [actions, setActions] = useState<Record<number, 'cancel' | 'retry'>>({});
  const requests = useRef(new Set<number>());
  const activeLessons = new Set(jobs.filter(job => ['running', 'queued'].includes(job.status)).map(job => job.lessonId));
  const pendingLessons = new Set(jobs.filter(job => actions[job.id]).map(job => job.lessonId));
  useEffect(() => {
    setActions(current => {
      const next = { ...current };
      for (const id of Object.keys(current)) {
        if (current[Number(id)] === 'cancel' && !jobs.some(job => job.id === Number(id) && ['running', 'queued'].includes(job.status))) delete next[Number(id)];
      }
      return Object.keys(next).length === Object.keys(current).length ? current : next;
    });
  }, [jobs]);
  async function act(job: Job, action: 'cancel' | 'retry') {
    if (requests.current.has(job.lessonId) || actions[job.id] || (action === 'retry' && activeLessons.has(job.lessonId))) return;
    requests.current.add(job.lessonId);
    setActions(current => ({ ...current, [job.id]: action }));
    try {
      await (action === 'cancel' ? call('cancel_transcription', { id: job.id }) : call('enqueue_transcription', { lessonIds: [job.lessonId] }));
      await onRefresh();
    } catch (e) {
      setActions(current => { const next = { ...current }; delete next[job.id]; return next });
      onError(e);
    } finally {
      requests.current.delete(job.lessonId);
      if (action === 'retry') setActions(current => { const next = { ...current }; delete next[job.id]; return next });
    }
  }
  return <section className="page"><h1>{t("Transcription")}</h1><p className="intro">{t("Generate captions from a lesson or a course. You can keep watching while the queue runs.")}</p>
    {!jobs.length && <div className="empty"><h2>{t("No jobs yet")}</h2><p>{t("Import a medium model in Settings, then choose “Generate captions” inside a course.")}</p></div>}
    {jobs.map(job => <article className="panel job" key={job.id}><div className="section-heading"><h2>{job.title}</h2><span className="badge">{t(job.status)}</span></div><p className="muted">{t(job.stage)}{job.status === 'running' ? ` · ${job.progress}%` : ''}</p>{job.status === 'running' && <progress aria-label={t('{lesson} transcription progress', { lesson: job.title })} value={job.progress} max={100} />}{job.error && <p className="error-text">{t(job.error)}</p>}
      {['running', 'queued'].includes(job.status) && <><button disabled={pendingLessons.has(job.lessonId)} onClick={() => void act(job, 'cancel')}>{actions[job.id] === 'cancel' ? t("Cancelling…") : t("Cancel")}</button>{actions[job.id] === 'cancel' && <p className="muted" role="status">{t("Cancellation requested. Waiting for the worker to stop.")}</p>}</>}
      {['failed', 'cancelled', 'interrupted'].includes(job.status) && !activeLessons.has(job.lessonId) && <button disabled={pendingLessons.has(job.lessonId)} onClick={() => void act(job, 'retry')}>{actions[job.id] === 'retry' ? t("Queuing lesson…") : t("Retry lesson")}</button>}
    </article>)}
  </section>;
}
