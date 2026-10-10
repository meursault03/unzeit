import { useEffect, useMemo, useRef, useState } from 'react';
import { useI18n } from './i18n';
import { codeExcerpt } from './utils';
import { isTauri } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

export function CodePreview({ text, title }: { text: string; title: string }) {
  const { t, count } = useI18n();
  const lines = useMemo(() => text.split(/\r\n|\n|\r/), [text]);
  const [query, setQuery] = useState('');
  const matches = useMemo(() => query ? lines.flatMap((line, i) => line.toLocaleLowerCase().includes(query.toLocaleLowerCase()) ? [i] : []) : [], [lines, query]);
  const [match, setMatch] = useState(0);
  const [page, setPage] = useState(0);
  const [start, setStart] = useState(1);
  const [end, setEnd] = useState(1);
  const [message, setMessage] = useState('');
  const current = useRef<HTMLDivElement>(null);
  const pages = Math.ceil(lines.length / 200);
  const selected = Number.isInteger(start) && Number.isInteger(end) && start >= 1 && end >= start && end <= lines.length;
  useEffect(() => {
    const node = current.current; const frame = node?.closest('.code-lines');
    if (!node || !frame || !query) return;
    frame.scrollTop += node.getBoundingClientRect().top - frame.getBoundingClientRect().top;
  }, [match, page, query]);
  function find(value: string) {
    setQuery(value); setMatch(0);
    const index = lines.findIndex(line => line.toLocaleLowerCase().includes(value.toLocaleLowerCase()));
    setPage(Math.max(0, Math.floor(index / 200)));
  }
  function next(direction: number) {
    const index = (match + direction + matches.length) % matches.length;
    setMatch(index); setPage(Math.floor(matches[index] / 200));
  }
  async function copy(content: string) {
    try { if (isTauri()) await writeText(content); else await navigator.clipboard.writeText(content); setMessage(t('Copied.')) }
    catch { setMessage(t('Could not copy. Select the text and press Ctrl+C.')) }
  }
  return <section className="code-preview" aria-label={title}>
    <div className="field-row"><input type="search" aria-label={t('Search this file')} placeholder={t('Find in file…')} value={query} onChange={e => find(e.target.value)} /><button disabled={!matches.length} onClick={() => next(-1)}>{t('Previous match')}</button><button disabled={!matches.length} onClick={() => next(1)}>{t('Next match')}</button></div>
    {query && <p className="muted" role="status">{count(matches.length, 'matching line')}{matches.length > 0 && ` · ${match + 1}/${matches.length}`}</p>}
    <div className="code-selection"><label>{t('First line')}<input type="number" min={1} max={lines.length} value={Number.isFinite(start) ? start : ''} onChange={e => { setStart(e.target.valueAsNumber); setMessage('') }} /></label><label>{t('Last line')}<input type="number" min={Number.isFinite(start) ? start : 1} max={lines.length} value={Number.isFinite(end) ? end : ''} onChange={e => { setEnd(e.target.valueAsNumber); setMessage('') }} /></label><button disabled={!selected} onClick={() => void copy(codeExcerpt(lines, start, end))}>{t('Copy selected lines')}</button><button onClick={() => void copy(text)}>{t('Copy file')}</button></div>
    <p className="muted code-hint">{t('Select a line; Shift-click to select a range.')}</p><p role="status">{message}</p>
    <div className="resource-text code-lines" tabIndex={0} aria-label={title}>{lines.slice(page * 200, (page + 1) * 200).map((line, index) => {
      const number = page * 200 + index + 1;
      return <div key={number} aria-current={query && number - 1 === matches[match] ? 'true' : undefined} ref={query && number - 1 === matches[match] ? current : undefined} className={`code-line${selected && number >= start && number <= end ? ' selected' : ''}${query && matches.includes(number - 1) ? ' match' : ''}`}><button className="line-number" aria-label={t('Select line {line}', { line: number })} aria-pressed={selected && number >= start && number <= end} onClick={e => { if (e.shiftKey) { setStart(Math.min(start,number)); setEnd(Math.max(start,number)) } else { setStart(number); setEnd(number) } setMessage('') }}>{number}</button><code>{line || '\u00a0'}</code></div>;
    })}</div>
    <div className="transcript-pages"><button disabled={page <= 0} onClick={() => setPage(n => n - 1)}>{t('Previous page')}</button><small>{t('Lines {start}–{end} of {count}', { start: page * 200 + 1, end: Math.min(lines.length, (page + 1) * 200), count: lines.length })}</small><button disabled={page >= pages - 1} onClick={() => setPage(n => n + 1)}>{t('Next page')}</button></div>
  </section>;
}
