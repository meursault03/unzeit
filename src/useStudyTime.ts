import { useCallback, useEffect, useRef, type RefObject } from 'react';
import { studyElapsed, type StudySample } from './study';
import { flushPendingStudy, saveStudyInterval } from './study-persistence';
import type { StudyInterval } from './types';

export function useStudyTime(video: RefObject<HTMLVideoElement | null>, lessonId: number | undefined, enabled: boolean, onError: (error: unknown) => void) {
  const flush = useRef<() => Promise<void>>(flushPendingStudy);
  const error = useRef(onError); error.current = onError;
  useEffect(() => {
    const node = video.current;
    if (!node || !lessonId || !enabled) { flush.current = flushPendingStudy; return }
    let active = !node.paused && !node.seeking && node.readyState >= 3;
    let previous: StudySample | null = null;
    let interval: StudyInterval | null = null;
    const persist = () => {
      const saved = interval; interval = null;
      return saved ? saveStudyInterval(saved) : flushPendingStudy();
    };
    const sample = () => {
      const current: StudySample = { monotonic: performance.now(), wall: Date.now(), position: node.currentTime, rate: node.playbackRate, active, seeking: node.seeking };
      const seconds = studyElapsed(previous, current);
      previous = current;
      if (!seconds) {
        if (interval) void persist().catch(error.current);
        return;
      }
      const endedAt = current.wall / 1000;
      const startedAt = endedAt - seconds;
      if (interval && (startedAt - interval.endedAt > 0.01 || endedAt - interval.startedAt > 15)) void persist().catch(error.current);
      if (!interval) interval = { id: crypto.randomUUID(), lessonId, startedAt, endedAt, seconds };
      else { interval.endedAt = endedAt; interval.seconds += seconds }
    };
    const start = () => { sample(); active = true; previous = null; sample() };
    const resume = () => { if (!node.paused && node.readyState >= 3) start() };
    const stop = () => { sample(); active = false; previous = null; void persist().catch(error.current) };
    const rateChange = () => { sample() };
    const flushNow = async () => { sample(); await persist() };
    flush.current = flushNow;
    node.addEventListener('playing', start);
    node.addEventListener('seeked', resume);
    for (const name of ['pause', 'waiting', 'seeking', 'ended', 'error']) node.addEventListener(name, stop);
    node.addEventListener('ratechange', rateChange);
    const timer = window.setInterval(sample, 250);
    sample();
    return () => {
      window.clearInterval(timer);
      node.removeEventListener('playing', start);
      node.removeEventListener('seeked', resume);
      for (const name of ['pause', 'waiting', 'seeking', 'ended', 'error']) node.removeEventListener(name, stop);
      node.removeEventListener('ratechange', rateChange);
      void flushNow().catch(error.current);
      flush.current = flushPendingStudy;
    };
  }, [video, lessonId, enabled]);
  return useCallback(() => flush.current(), []);
}
