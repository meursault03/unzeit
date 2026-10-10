export interface StudySample { monotonic: number; wall: number; position: number; rate: number; active: boolean; seeking: boolean }

export function studyElapsed(previous: StudySample | null, current: StudySample): number {
  if (!previous?.active || current.seeking) return 0;
  const elapsed = (current.monotonic - previous.monotonic) / 1000;
  const wall = (current.wall - previous.wall) / 1000;
  const advance = current.position - previous.position;
  if (![elapsed, wall, advance, previous.rate].every(Number.isFinite) || previous.rate <= 0 || elapsed <= 0 || elapsed > 2 || wall <= 0 || wall > 2 || Math.abs(wall - elapsed) > .5 || advance <= 0 || advance > elapsed * previous.rate + .25) return 0;
  return Math.min(elapsed, wall, advance / previous.rate);
}

export function studyWeek(today = new Date()): number[] {
  const start = new Date(today);
  start.setHours(0, 0, 0, 0);
  start.setDate(start.getDate() - (start.getDay() + 6) % 7);
  return Array.from({ length: 8 }, (_, offset) => {
    const day = new Date(start); day.setDate(start.getDate() + offset);
    return day.getTime() / 1000;
  });
}
