import type { Course, Lesson, Progress } from './types';
export function countLabel(count: number, label: string): string { return `${count} ${label}${count === 1 ? '' : 's'}` }

export function time(seconds: number): string {
  const n = Math.max(0, Math.floor(Number.isFinite(seconds) ? seconds : 0));
  return n >= 3600 ? `${Math.floor(n / 3600)}:${String(Math.floor(n / 60) % 60).padStart(2, '0')}:${String(n % 60).padStart(2, '0')}` : `${Math.floor(n / 60)}:${String(n % 60).padStart(2, '0')}`;
}
export function completion(course: Course): number { return course.lessons.length ? Math.round(course.lessons.filter(l => l.completed).length / course.lessons.length * 100) : 0 }
export function resumeLesson(course: Course): Lesson | undefined {
  return course.lessons.find(l => l.id === course.lastLessonId && !l.missing) ?? course.lessons.find(l => !l.completed && !l.missing) ?? course.lessons.find(l => !l.missing);
}

export function applyProgress(courses: Course[], progress: Progress): Course[] {
  return courses.map(course => course.lessons.some(l => l.id === progress.id) ? {
    ...course, lastLessonId: progress.id, updatedAt: Date.now() / 1000,
    lessons: course.lessons.map(l => l.id === progress.id ? { ...l, position: progress.position, duration: progress.duration, completed: progress.completed ?? l.completed } : l),
  } : course).sort((a, b) => b.updatedAt - a.updatedAt);
}

export function validLoop(start: number, end: number): boolean { return Number.isFinite(start) && Number.isFinite(end) && start >= 0 && end - start >= 0.5 }

export function codeExcerpt(lines: string[], start: number, end: number): string {
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || end < start || end > lines.length) throw new Error('Invalid line selection');
  return lines.slice(start - 1, end).join('\n');
}
