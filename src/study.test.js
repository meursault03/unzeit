import { describe, expect, it } from 'vitest';
import { studyElapsed, studyWeek } from './study';

const sample = (values = {}) => ({ monotonic: 1000, wall: 100000, position: 2, rate: 1, active: true, seeking: false, ...values });

describe('real study time', () => {
  it('counts elapsed wall time at double speed and the final active moment before pause', () => {
    const before = sample({ rate: 2 });
    expect(studyElapsed(before, sample({ monotonic: 2000, wall: 101000, position: 4, rate: 2 }))).toBe(1);
    expect(studyElapsed(before, sample({ monotonic: 1250, wall: 100250, position: 2.5, rate: 2, active: false }))).toBe(.25);
  });
  it('does not count paused, buffering, seeking or suspended playback', () => {
    const next = sample({ monotonic: 2000, wall: 101000, position: 3 });
    expect(studyElapsed(sample({ active: false }), next)).toBe(0);
    expect(studyElapsed(sample(), { ...next, position: 2 })).toBe(0);
    expect(studyElapsed(sample(), { ...next, seeking: true })).toBe(0);
    expect(studyElapsed(sample(), { ...next, position: 15 })).toBe(0);
    expect(studyElapsed(sample(), { ...next, monotonic: 7000, wall: 106000, position: 8 })).toBe(0);
    expect(studyElapsed(sample(), { ...next, monotonic: 1250, wall: 106000 })).toBe(0);
    expect(studyElapsed(sample(), { ...next, wall: 99000 })).toBe(0);
  });
  it('caps credited time at actual media advancement and handles playback rate changes', () => {
    expect(studyElapsed(sample(), sample({ monotonic: 2000, wall: 101000, position: 2.5 }))).toBe(.5);
    expect(studyElapsed(sample({ rate: 2 }), sample({ monotonic: 1500, wall: 100500, position: 3, rate: 1 }))).toBe(.5);
    expect(studyElapsed(null, sample())).toBe(0);
  });
});

describe('study week', () => {
  it('uses eight local-midnight boundaries from Monday through next Monday', () => {
    const week = studyWeek(new Date(2026, 9, 10, 12));
    expect(week).toHaveLength(8);
    expect(new Date(week[0] * 1000).getDay()).toBe(1);
    expect(new Date(week[0] * 1000).getHours()).toBe(0);
    expect(new Date(week[0] * 1000).getDate()).toBe(5);
    expect(new Date(week[7] * 1000).getDate()).toBe(12);
  });
  it('keeps local days through daylight-saving changes', () => {
    const previous = process.env.TZ;
    process.env.TZ = 'America/New_York';
    try {
      const week = studyWeek(new Date(2026, 10, 1, 12));
      expect(week[7] - week[6]).toBe(25 * 3600);
      expect(week.every(value => new Date(value * 1000).getHours() === 0)).toBe(true);
    } finally {
      if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous;
    }
  });
});
