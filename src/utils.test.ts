import { describe, expect, it } from 'vitest';
import { codeExcerpt, validLoop, applyProgress, resumeLesson, completion, time } from './utils';
import type { Course, Lesson } from './types';
const lesson = (id: number, completed = false, missing = false): Lesson => ({ id, completed, missing, title: String(id), section: 'General', path: '', duration: 0, position: 0, assets: [] });
const course: Course = { id: 1, title: 'Course', path: '', resources: [], updatedAt: 0, lastLessonId: 2, lessons: [lesson(1), lesson(2, false, true), lesson(3, true)] };
describe('learning state', () => {
  it('never resumes a missing file', () => expect(resumeLesson(course)?.id).toBe(1));
  it('completion follows explicit lesson state', () => expect(completion(course)).toBe(33));
  it('updates resume position without clearing explicit completion', () => {
    const updated = applyProgress([course], { id: 3, position: 20, duration: 100, completed: null })[0];
    expect(resumeLesson(updated)?.id).toBe(3); expect(updated.lessons[2].position).toBe(20); expect(updated.lessons[2].completed).toBe(true); expect(course.lessons[2].position).toBe(0);
  });
  it('rejects invalid repeat intervals', () => { expect(validLoop(2,3)).toBe(true); for (const [a,b] of [[-1,2],[2,1],[2,2.4],[NaN,3],[0,Infinity]]) expect(validLoop(a,b)).toBe(false) });
  it('formats long timestamps and invalid values', () => { expect(time(3661)).toBe('1:01:01'); expect(time(NaN)).toBe('0:00') });
  it('copies exactly the selected source lines and rejects invalid ranges', () => {
    const lines = ['def café():', '    return "<script>"', '', 'café()'];
    expect(codeExcerpt(lines, 1, 2)).toBe('def café():\n    return "<script>"');
    expect(codeExcerpt(lines, 3, 3)).toBe('');
    for (const [a,b] of [[0,2],[2,1],[1,5],[1.5,3],[NaN,3]]) expect(() => codeExcerpt(lines, a, b)).toThrow();
  });
});
