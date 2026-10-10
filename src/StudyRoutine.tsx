import { useCallback, useEffect, useId, useRef, useState } from 'react';
import { call } from './api';
import { useI18n } from './i18n';
import { studyWeek } from './study';
import { flushPendingStudy } from './study-persistence';
import type { Settings } from './types';
import './study.css';

interface Props { settings: Settings; onSave: (settings: Settings) => Promise<void> }
export function StudyRoutine({ settings, onSave }: Props) {
  const { t } = useI18n();
  const [days, setDays] = useState(() => studyWeek());
  const [seconds, setSeconds] = useState<number[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [goal, setGoal] = useState(String(settings.weeklyGoalMinutes));
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');
  const savingRef = useRef(false);
  const field = useId();
  const load = useCallback(async (boundaries: number[]) => {
    await flushPendingStudy();
    return call<number[]>('get_study_time', { dayStarts: boundaries });
  }, []);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let live = true;
    setLoading(true); setError('');
    void load(days).then(value => { if (live) setSeconds(value) }).catch(e => { if (live) setError(String(e)) }).finally(() => { if (live) setLoading(false) });
    return () => { live = false };
  }, [days, load, retry]);
  useEffect(() => {
    const refresh = () => {
      const next = studyWeek();
      setDays(current => current[0] === next[0] ? current : next);
    };
    const timer = window.setInterval(refresh, 60000);
    window.addEventListener('focus', refresh);
    return () => { window.clearInterval(timer); window.removeEventListener('focus', refresh) };
  }, []);
  useEffect(() => { setGoal(String(settings.weeklyGoalMinutes)) }, [settings.weeklyGoalMinutes]);
  const duration = (value: number) => {
    const minutes = Math.floor(value / 60);
    if (value > 0 && minutes === 0) return t('Less than a minute');
    return minutes < 60 ? t('{minutes} min', { minutes }) : t('{hours} h {minutes} min', { hours: Math.floor(minutes / 60), minutes: minutes % 60 });
  };
  const total = seconds.reduce((sum, value) => sum + value, 0);
  const maximum = Math.max(60, ...seconds);
  const weekday = new Intl.DateTimeFormat(settings.language, { weekday: 'short' });
  const fullDate = new Intl.DateTimeFormat(settings.language, { weekday: 'long', month: 'long', day: 'numeric' });
  async function saveGoal(event: React.FormEvent) {
    event.preventDefault();
    if (savingRef.current) return;
    const minutes = Number(goal);
    if (!Number.isInteger(minutes) || minutes < 0 || minutes > 10080) { setMessage(t('Use a whole number from 0 to 10080.')); return }
    savingRef.current = true; setSaving(true); setMessage('');
    try { await onSave({ ...settings, weeklyGoalMinutes: minutes }); setMessage(t('Saved')) }
    catch (e) { setMessage(String(e)) }
    finally { savingRef.current = false; setSaving(false) }
  }
  return <details className="study-routine">
    <summary><span>{t('Study time this week')}</span><span className="muted">{loading ? t('Loading…') : error ? t('Unavailable') : duration(total)}</span></summary>
    <div className="study-routine-content" aria-busy={loading}>
      {error ? <div role="alert"><p>{t('Unable to load study time.')} {t(error)}</p><button onClick={() => setRetry(value => value + 1)}>{t('Retry')}</button></div> : <>
        <ol className="study-days" aria-label={t('Daily study time')}>
          {days.slice(0, 7).map((start, index) => <li key={start} aria-label={t('{day}: {time}', { day: fullDate.format(new Date(start * 1000)), time: duration(seconds[index] ?? 0) })}>
            <span className="study-day-time">{duration(seconds[index] ?? 0)}</span>
            <span className="study-bar-track" aria-hidden="true"><span className="study-bar" style={{ height: `${Math.max(2, (seconds[index] ?? 0) / maximum * 100)}%` }} /></span>
            <span>{weekday.format(new Date(start * 1000))}</span>
          </li>)}
        </ol>
        <p className="muted study-time-help">{t('Counts time spent playing lessons, excluding pauses and seeking.')}</p>
        {settings.weeklyGoalMinutes > 0 && <p>{t('{time} of {goal} this week', { time: duration(total), goal: duration(settings.weeklyGoalMinutes * 60) })}</p>}
      </>}
      <form className="study-goal" onSubmit={event => void saveGoal(event)}>
        <label htmlFor={field}>{t('Weekly goal (minutes)')}</label>
        <div><input id={field} type="number" min={0} max={10080} step={1} value={goal} disabled={saving} aria-describedby={`${field}-help`} onChange={event => { setGoal(event.target.value); setMessage('') }} /><button disabled={saving}>{saving ? t('Saving…') : t('Save goal')}</button></div>
        <p id={`${field}-help`} className="muted">{t('Optional. Set to 0 to turn off the goal.')}</p>
        <p role="status">{message}</p>
      </form>
    </div>
  </details>;
}
