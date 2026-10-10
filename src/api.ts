import { invoke, isTauri } from '@tauri-apps/api/core';

export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return Promise.reject(new Error('Open Unzeit as a desktop app with npm run tauri dev. Browser preview cannot access your files.'));
  return invoke<T>(command, args);
}
let dialogOpen = false;
async function choose(options: { title: string; directory: boolean; defaultPath?: string; extensions?: string[] }): Promise<string | null> {
  if (dialogOpen) return null;
  dialogOpen = true;
  try { return await call<string | null>('choose_path', { ...options, defaultPath: options.defaultPath || null, extensions: options.extensions ?? [] }) } finally { dialogOpen = false }
}
export async function chooseFolder(title = 'Choose folder', defaultPath?: string): Promise<string | null> {
  if (!isTauri()) throw new Error('Folder selection is available in the desktop app.');
  return await choose({ directory: true, title, defaultPath: defaultPath || undefined });
}
export async function chooseFile(title: string, extensions: string[], defaultPath?: string): Promise<string | null> {
  if (!isTauri()) throw new Error('File selection is available in the desktop app.');
  return await choose({ directory: false, title, defaultPath: defaultPath || undefined, extensions });
}
