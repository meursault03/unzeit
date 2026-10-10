import { useI18n } from './i18n';
import { useEffect, useState } from 'react';
import { call } from './api';
import { time } from './utils';
import type { Note } from './types';

export function Notes({ lessonId, position, onSeek, onError }: { lessonId: number; position: () => number; onSeek: (position: number) => void; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [notes, setNotes] = useState<Note[]>([]);
  const [text, setText] = useState('');
  const [editing, setEditing] = useState<number | null>(null);
  const [editText, setEditText] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => { let active = true; setNotes([]); setText(''); setEditing(null); void call<Note[]>('get_notes', { lessonId }).then(n => { if (active) setNotes(n) }).catch(onError); return () => { active = false } }, [lessonId, onError]);
  const refresh = () => call<Note[]>('get_notes', { lessonId }).then(setNotes);
  async function save(bookmark: boolean) {
    setBusy(true);
    try { await call('save_note', { note: { id: 0, lessonId, position: position(), text, bookmark } }); setText(''); await refresh() }
    catch (e) { onError(e) } finally { setBusy(false) }
  }
  return <section className="notes"><h2>{t("Notes & bookmarks")}</h2><form onSubmit={e => { e.preventDefault(); void save(false) }}><label htmlFor="note-text">{t("At this moment")}</label><textarea id="note-text" value={text} maxLength={5000} onChange={e => setText(e.target.value)} placeholder={t("An idea to remember, a question to revisit…")} /><div className="field-row"><button className="primary" disabled={!text.trim() || busy}>{t("Save note")}</button><button type="button" disabled={busy} onClick={() => void save(true)}>{t("Bookmark moment")}</button></div></form>
    {!notes.length && <p className="muted">{t("Your notes stay with this lesson. Select a timestamp to jump back.")}</p>}
    <ul className="note-list">{notes.map(note => <li key={note.id}><div className="field-row"><button className="timestamp" onClick={() => onSeek(note.position)} aria-label={t('Seek to {time}', { time: time(note.position) })}>{time(note.position)}</button><span className="muted">{note.bookmark ? t("Bookmark") : t("Note")}</span><button className="quiet push" onClick={() => { setEditing(note.id); setEditText(note.text) }}>{t("Edit")}</button><button className="quiet" aria-label={t(note.bookmark ? 'Delete bookmark at {time}' : 'Delete note at {time}', { time: time(note.position) })} onClick={() => void call('delete_note', { id: note.id }).then(refresh).catch(onError)}>{t("Delete")}</button></div>
      {editing === note.id ? <form onSubmit={e => { e.preventDefault(); void call('save_note', { note: { ...note, text: editText } }).then(() => { setEditing(null); return refresh() }).catch(onError) }}><textarea aria-label={t("Edit note text")} value={editText} maxLength={5000} onChange={e => setEditText(e.target.value)} /><div className="field-row"><button>{t("Save changes")}</button><button type="button" onClick={() => setEditing(null)}>{t("Cancel")}</button></div></form> : <p>{note.text || t("Saved moment")}</p>}
    </li>)}</ul>
  </section>;
}
