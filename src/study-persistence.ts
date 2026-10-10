import { call } from './api';
import type { StudyInterval } from './types';

// Keep failed writes across lesson changes; retry the same immutable ID and payload.
const pending: StudyInterval[] = [];
let writing: Promise<void> | null = null;
export async function flushPendingStudy(): Promise<void> {
  if (writing) {
    await writing;
    if (pending.length) await flushPendingStudy();
    return;
  }
  if (!pending.length) return;
  writing = (async () => {
    while (pending.length) {
      await call('record_study_interval', { interval: pending[0] });
      pending.shift();
    }
  })().finally(() => { writing = null });
  await writing;
  if (pending.length) await flushPendingStudy();
}
export function saveStudyInterval(interval: StudyInterval): Promise<void> {
  pending.push(interval);
  return flushPendingStudy();
}
