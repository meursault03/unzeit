import { useI18n } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { call } from './api';
import { time } from './utils';
import type { Note } from './types';

export function Notes({ lessonId, position, onSeek, onError }: { lessonId: number; position: () => number; onSeek: (position: number) => void; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [notes, setNotes] = useState<Note[]>([]);
  const [text, setText] = useState('');
  const [editing, setEditing] = useState<number | null>(null);
  const [editText, setEditText] = useState('');
  const [deleting, setDeleting] = useState<number | null>(null);
  const [busy, setBusy] = useState<'save' | 'delete' | null>(null);
  const [status, setStatus] = useState('');
  const pending = useRef(false);
  const revision = useRef(0);
  const error = useRef(onError);
  error.current = onError;
  const input = useRef<HTMLTextAreaElement>(null);
  const editInput = useRef<HTMLTextAreaElement>(null);
  const cancelDelete = useRef<HTMLButtonElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    const current = ++revision.current;
    pending.current = false;
    setNotes([]); setText(''); setEditing(null); setDeleting(null); setBusy(null); setStatus('');
    void call<Note[]>('get_notes', { lessonId }).then(n => { if (current === revision.current) setNotes(n) }).catch(e => { if (current === revision.current) error.current(e) });
    return () => { revision.current++ };
  }, [lessonId]);
  useEffect(() => {
    if (busy) return;
    if (deleting !== null) cancelDelete.current?.focus();
    else if (editing !== null) editInput.current?.focus();
    else if (returnFocus.current) (returnFocus.current.isConnected ? returnFocus.current : input.current)?.focus();
  }, [busy, deleting, editing]);
  async function change(command: 'save_note' | 'delete_note', args: Record<string, unknown>, saved: () => void) {
    if (pending.current) return;
    pending.current = true;
    const current = ++revision.current;
    setBusy(command === 'save_note' ? 'save' : 'delete'); setStatus('');
    try {
      await call(command, args);
      if (current !== revision.current) return;
      saved();
      setStatus(command === 'save_note' ? 'Saved' : 'Deleted');
      const updated = await call<Note[]>('get_notes', { lessonId });
      if (current === revision.current) setNotes(updated);
    } catch (e) { if (current === revision.current) error.current(e) }
    finally { if (current === revision.current) { pending.current = false; setBusy(null) } }
  }
  async function save(bookmark: boolean) {
    if (editing !== null || deleting !== null || (!bookmark && !text.trim())) return;
    returnFocus.current = input.current;
    const submitted = text;
    await change('save_note', { note: { id: 0, lessonId, position: position(), text: submitted, bookmark } }, () => setText(value => value === submitted ? '' : value));
  }
  const choosing = editing !== null || deleting !== null;
  return <section className="notes" aria-busy={busy !== null}><h2>{t("Notes & bookmarks")}</h2><form onSubmit={e => { e.preventDefault(); void save(false) }}><label htmlFor="note-text">{t("At this moment")}</label><textarea ref={input} id="note-text" value={text} readOnly={busy !== null} maxLength={5000} onChange={e => setText(e.target.value)} placeholder={t("An idea to remember, a question to revisit…")} /><div className="field-row"><button className="primary" disabled={!text.trim() || busy !== null || choosing}>{t("Save note")}</button><button type="button" disabled={busy !== null || choosing} onClick={() => void save(true)}>{t("Bookmark moment")}</button></div></form>
    <p className="muted" role="status">{busy ? t(busy === 'save' ? 'Saving…' : 'Deleting…') : status ? t(status) : ''}</p>
    {!notes.length && <p className="muted">{t("Your notes stay with this lesson. Select a timestamp to jump back.")}</p>}
    <ul className="note-list">{notes.map(note => <li key={note.id}><div className="field-row"><button className="timestamp" onClick={() => onSeek(note.position)} aria-label={t('Seek to {time}', { time: time(note.position) })}>{time(note.position)}</button><span className="muted">{note.bookmark ? t("Bookmark") : t("Note")}</span><button className="quiet push" disabled={busy !== null || choosing} onClick={e => { returnFocus.current = e.currentTarget; setEditing(note.id); setEditText(note.text); setStatus('') }}>{t("Edit")}</button><button className="quiet" disabled={busy !== null || choosing} aria-label={t(note.bookmark ? 'Delete bookmark at {time}' : 'Delete note at {time}', { time: time(note.position) })} onClick={e => { returnFocus.current = e.currentTarget; setDeleting(note.id); setStatus('') }}>{t("Delete")}</button></div>
      {editing === note.id ? <form onSubmit={e => { e.preventDefault(); if (!note.bookmark && !editText.trim()) return; void change('save_note', { note: { ...note, text: editText } }, () => setEditing(null)) }}><textarea ref={editInput} aria-label={t("Edit note text")} value={editText} readOnly={busy !== null} maxLength={5000} onChange={e => setEditText(e.target.value)} /><div className="field-row"><button disabled={busy !== null || (!note.bookmark && !editText.trim())}>{t("Save changes")}</button><button type="button" disabled={busy !== null} onClick={() => setEditing(null)}>{t("Cancel")}</button></div></form> : <p>{note.text || t("Saved moment")}</p>}
      {deleting === note.id && <div role="group" aria-label={t(note.bookmark ? 'Delete this bookmark?' : 'Delete this note?')} onKeyDown={e => { if (e.key === 'Escape' && !busy) { e.stopPropagation(); setDeleting(null) } }}><p>{t(note.bookmark ? 'Delete this bookmark?' : 'Delete this note?')}</p><div className="field-row"><button ref={cancelDelete} type="button" disabled={busy !== null} onClick={() => setDeleting(null)}>{t('Cancel')}</button><button type="button" disabled={busy !== null} onClick={() => void change('delete_note', { id: note.id }, () => { setDeleting(null); setNotes(value => value.filter(n => n.id !== note.id)) })}>{t('Delete')}</button></div></div>}
    </li>)}</ul>
  </section>;
}
