import { beforeEach, expect, it, vi } from 'vitest';
const { open, isTauri } = vi.hoisted(() => ({ open: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: open, isTauri }));
import { chooseFile, chooseFolder } from './api';
beforeEach(() => { open.mockReset(); isTauri.mockReturnValue(true) });
it('returns cancellation unchanged and reopens at the supplied directory', async () => {
  open.mockResolvedValueOnce(null).mockResolvedValueOnce('/courses/selected');
  expect(await chooseFolder('Library', '/courses')).toBeNull();
  expect(open).toHaveBeenCalledWith('choose_path', { title: 'Library', directory: true, defaultPath: '/courses', extensions: [] });
  expect(await chooseFolder('Library', '/courses')).toBe('/courses/selected');
});
it('allows only one native picker at a time, then releases it', async () => {
  let cancel!: (path: null) => void;
  open.mockImplementationOnce(() => new Promise(resolve => { cancel = resolve })).mockResolvedValue(null);
  const first = chooseFolder('Library');
  expect(await chooseFile('Model', ['bin'])).toBeNull();
  expect(open).toHaveBeenCalledTimes(1);
  cancel(null); await first;
  await chooseFile('Model', ['bin']);
  expect(open).toHaveBeenCalledTimes(2);
});
it('releases the picker after an error, without swallowing the error', async () => {
  open.mockRejectedValueOnce(new Error('Portal unavailable')).mockResolvedValueOnce('/model.bin');
  await expect(chooseFolder()).rejects.toThrow('Portal unavailable');
  expect(await chooseFile('Model', ['bin'])).toBe('/model.bin');
});
