import { describe, expect, it, vi } from 'vitest';
const call = vi.hoisted(() => vi.fn());
vi.mock('./api', () => ({ call }));
describe('study persistence', () => {
  it('retries failed immutable intervals without losing the following interval', async () => {
    vi.resetModules(); call.mockReset();
    const { saveStudyInterval, flushPendingStudy } = await import('./study-persistence');
    const first = { id: 'first', lessonId: 1, startedAt: 0, endedAt: 1, seconds: 1 };
    const second = { ...first, id: 'second', startedAt: 1, endedAt: 2 };
    call.mockRejectedValueOnce(new Error('disk full')).mockResolvedValue(undefined);
    await expect(saveStudyInterval(first)).rejects.toThrow('disk full');
    await saveStudyInterval(second);
    await flushPendingStudy();
    expect(call.mock.calls.map(([command, args]) => [command, args.interval])).toEqual([
      ['record_study_interval', first], ['record_study_interval', first], ['record_study_interval', second],
    ]);
  });
  it('serializes concurrent saves and awaits intervals added during a flush', async () => {
    vi.resetModules(); call.mockReset();
    const { saveStudyInterval, flushPendingStudy } = await import('./study-persistence');
    let finish;
    call.mockImplementationOnce(() => new Promise(resolve => { finish = resolve })).mockResolvedValue(undefined);
    const first = saveStudyInterval({ id: 'one' });
    const second = saveStudyInterval({ id: 'two' });
    const close = flushPendingStudy();
    expect(call).toHaveBeenCalledTimes(1);
    finish();
    await Promise.all([first, second, close]);
    expect(call.mock.calls.map(([, args]) => args.interval.id)).toEqual(['one', 'two']);
  });
});
