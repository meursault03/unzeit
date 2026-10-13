import { Select } from './Select';
import { useI18n } from './i18n';
import { useEffect, useState } from 'react';
import { call, chooseFile, chooseFolder } from './api';
import type { Settings } from './types';

export function Preferences({ settings, onSave, onError, onModelImported }: { settings: Settings; onSave: (s: Settings) => Promise<void>; onError: (e: unknown) => void; onModelImported: (path: string) => void }) {
  const { t } = useI18n();
  const [draft, setDraft] = useState(settings);
  const [deps, setDeps] = useState<[string, boolean][]>([]);
  const [operation, setOperation] = useState<'folder' | 'model-picker' | 'model' | 'save' | null>(null);
  const busy = operation !== null;
  const [saved, setSaved] = useState(false);
  useEffect(() => { void call<[string, boolean][]>('dependencies').then(setDeps).catch(onError) }, [onError]);
  async function browse() {
    setOperation('folder');
    try {
      const path = await chooseFolder(t('Choose library root'), draft.libraryRoot);
      if (path) { setDraft(d => ({ ...d, libraryRoot: path })); setSaved(false) }
    } catch (e) { onError(e) } finally { setOperation(null) }
  }
  async function model() {
    setOperation('model-picker');
    try {
      const path = await chooseFile(t("Choose a medium Whisper GGML model"), ['bin']); if (!path) return;
      setOperation('model');
      const modelPath = await call<string>('import_model', { path }); setDraft(d => ({ ...d, modelPath }));
      onModelImported(modelPath);
    } catch (e) { onError(e) } finally { setOperation(null) }
  }
  return <section className="page" aria-busy={busy}><h1>{t("Settings")}</h1><p className="intro">{t("Playback, appearance, and local transcription.")}</p>
    <form className="panel preferences" onChange={() => setSaved(false)} onSubmit={e => { e.preventDefault(); setOperation('save'); void onSave(draft).then(() => setSaved(true)).catch(onError).finally(() => setOperation(null)) }}>
      <h2>{t("Library & playback")}</h2><label htmlFor="root-setting">{t("Library root")}</label><div className="field-row"><input disabled={busy} id="root-setting" value={draft.libraryRoot} onChange={e => setDraft({ ...draft, libraryRoot: e.target.value })} /><button disabled={busy} type="button" onClick={() => void browse()}>{t("Browse")}</button></div>
      <label htmlFor="app-language">{t('App language')}</label><Select disabled={busy} id="app-language" label={t('App language')} value={draft.language} onChange={language => { setDraft({ ...draft, language: language as Settings['language'] }); setSaved(false) }} options={[{ value: 'pt-BR', label: 'Português (Brasil)' }, { value: 'en', label: 'English' }, { value: 'es', label: 'Español' }, { value: 'ru', label: 'Русский' }, { value: 'zh-CN', label: '简体中文' }, { value: 'fr', label: 'Français' }]} />
      <label htmlFor="theme">{t("Appearance")}</label><Select disabled={busy} id="theme" label={t('Appearance')} value={draft.theme} onChange={theme => { setDraft({ ...draft, theme: theme as Settings['theme'] }); setSaved(false) }} options={[{ value: 'system', label: t('Follow system') }, { value: 'light', label: t('Light') }, { value: 'dark', label: t('Dark') }]} />
      <label htmlFor="speed-setting">{t("Default playback speed")}</label><Select disabled={busy} id="speed-setting" label={t('Default playback speed')} value={draft.speed} onChange={speed => { setDraft({ ...draft, speed: Number(speed) }); setSaved(false) }} options={[0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3].map(n => ({ value: n, label: `${n}×` }))} />
      <label className="check"><input disabled={busy} type="checkbox" checked={draft.autoplay} onChange={e => setDraft({ ...draft, autoplay: e.target.checked })} /> {t("Autoplay the next lesson after a five-second countdown")}</label>
      <button className="primary" disabled={busy} type="submit">{t("Save preferences")}</button>{saved && <p role="status">{t("Preferences saved.")}</p>}
    </form>
    <div className="panel preferences"><h2>{t("Local transcription")}</h2><p>{t("Import a multilingual medium model in whisper.cpp GGML format. The model is validated and copied into Unzeit’s app-data folder.")}</p><p className="path">{draft.modelPath || t("No model imported")}</p><button disabled={busy} onClick={() => void model()}>{operation === 'model' ? t("Validating model…") : t("Import medium model")}</button><p className="muted">{t("Requires approximately 1.5 GiB of disk space for the full model. Quantized medium models are also supported. CPU inference; one lesson at a time.")}</p></div>
    <div className="panel preferences"><h2>{t("System dependencies")}</h2>{deps.length ? <ul className="dependency-list">{deps.map(([name, ok]) => <li key={name}><code>{name}</code><span>{ok ? t("Available") : t("Missing — install with your package manager")}</span></li>)}</ul> : <p className="muted">{t("Checking installed tools…")}</p>}<p className="muted">{t("Linux playback also needs WebKitGTK and the GStreamer plugins for your videos’ codecs.")}</p></div>
  </section>;
}
