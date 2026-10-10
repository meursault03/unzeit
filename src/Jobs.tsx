import { useI18n } from './i18n';
import { call } from './api';
import type { Job } from './types';
export function Jobs({ jobs, onRefresh, onError }: { jobs: Job[]; onRefresh: () => Promise<void>; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  return <section className="page"><h1>{t("Transcription")}</h1><p className="intro">{t("Generate captions from a lesson or a course. You can keep watching while the queue runs.")}</p>
    {!jobs.length && <div className="empty"><h2>{t("No jobs yet")}</h2><p>{t("Import a medium model in Settings, then choose “Generate captions” inside a course.")}</p></div>}
    {jobs.map(job => <article className="panel job" key={job.id}><div className="section-heading"><h2>{job.title}</h2><span className="badge">{t(job.status)}</span></div><p className="muted">{t(job.stage)}{job.status === 'running' ? ` · ${job.progress}%` : ''}</p>{job.status === 'running' && <progress aria-label={t('{lesson} transcription progress', { lesson: job.title })} value={job.progress} max={100} />}{job.error && <p className="error-text">{t(job.error)}</p>}
      {['running', 'queued'].includes(job.status) && <button onClick={() => void call('cancel_transcription', { id: job.id }).then(onRefresh).catch(onError)}>{t("Cancel")}</button>}
      {['failed', 'cancelled', 'interrupted'].includes(job.status) && <button onClick={() => void call('enqueue_transcription', { lessonIds: [job.lessonId] }).then(onRefresh).catch(onError)}>{t("Retry lesson")}</button>}
    </article>)}
  </section>;
}
