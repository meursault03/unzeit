import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return Promise.reject(new Error('Open Unzeit as a desktop app with npm run tauri dev. Browser preview cannot access your files.'));
  return invoke<T>(command, args);
}
export async function chooseFolder(title = 'Choose folder'): Promise<string | null> {
  if (!isTauri()) throw new Error('Folder selection is available in the desktop app.');
  return await open({ directory: true, multiple: false, title });
}
export async function chooseFile(title: string, extensions: string[]): Promise<string | null> {
  if (!isTauri()) throw new Error('File selection is available in the desktop app.');
  return await open({ directory: false, multiple: false, title, filters: [{ name: title, extensions }] });
}
