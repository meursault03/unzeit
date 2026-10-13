import { useEffect, useId, useRef, useState, type KeyboardEvent } from 'react';
import { CaretDownIcon } from '@phosphor-icons/react';

type Option = { value: string | number; label: string };
export function Select({ id, label, value, options, disabled = false, onChange }: {
  id?: string; label: string; value: string | number; options: readonly Option[];
  disabled?: boolean; onChange: (value: string) => void;
}) {
  const generated = useId();
  const name = id ?? generated;
  const trigger = useRef<HTMLButtonElement>(null);
  const popup = useRef<HTMLSpanElement>(null);
  const selected = Math.max(0, options.findIndex(option => String(option.value) === String(value)));
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(selected);
  const search = useRef({ text: '', time: 0 });
  function hide() {
    popup.current?.hidePopover();
    search.current = { text: '', time: 0 };
    setOpen(false);
  }
  function commit(index = active) {
    if (options[index] && String(options[index].value) !== String(value)) onChange(String(options[index].value));
    hide();
  }
  function show(index = selected) {
    const button = trigger.current; const menu = popup.current;
    if (!button || !menu || disabled || !options.length) return;
    const rect = button.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 8;
    const above = rect.top - 8;
    menu.style.width = `${Math.min(rect.width, window.innerWidth - 16)}px`;
    menu.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - rect.width - 8))}px`;
    menu.showPopover();
    const down = below >= Math.min(menu.scrollHeight, 288) || below >= above;
    menu.style.maxHeight = `${Math.max(32, Math.min(288, down ? below : above))}px`;
    menu.style.top = `${down ? rect.bottom + 4 : Math.max(8, rect.top - menu.getBoundingClientRect().height - 4)}px`;
    setActive(index); setOpen(true);
  }
  useEffect(() => {
    if (open) popup.current?.children[active]?.scrollIntoView({ block: 'nearest' });
  }, [active, open]);
  useEffect(() => {
    if (disabled) { popup.current?.hidePopover(); setOpen(false) }
    if (!open) return;
    const dismiss = (event: Event) => {
      if (event.target instanceof Node && (popup.current?.contains(event.target) || trigger.current?.contains(event.target))) return;
      popup.current?.hidePopover(); setOpen(false);
    };
    window.addEventListener('blur', dismiss);
    window.addEventListener('resize', dismiss);
    window.addEventListener('scroll', dismiss, true);
    window.addEventListener('pointerdown', dismiss);
    return () => {
      window.removeEventListener('blur', dismiss);
      window.removeEventListener('resize', dismiss);
      window.removeEventListener('scroll', dismiss, true);
      window.removeEventListener('pointerdown', dismiss);
    };
  }, [open, disabled]);
  function key(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.ctrlKey || event.metaKey || !options.length) return;
    const index = open ? active : selected;
    if (event.key === 'Tab') { if (open) commit(); return }
    if (event.key === 'Escape') { if (open) { event.preventDefault(); event.stopPropagation(); hide() } return }
    if (event.altKey && event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    if (event.altKey && event.key === 'ArrowUp' && open) { event.preventDefault(); commit(); return }
    let next = index;
    if (event.key === 'ArrowDown') next = open ? Math.min(index + 1, options.length - 1) : index;
    else if (event.key === 'ArrowUp') next = open ? Math.max(0, index - 1) : 0;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = options.length - 1;
    else if (event.key === 'PageDown') next = Math.min(index + 10, options.length - 1);
    else if (event.key === 'PageUp') next = Math.max(0, index - 10);
    else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault(); if (open) commit(); else show(); return;
    } else if (event.key.length === 1) {
      const time = Date.now();
      const text = (time - search.current.time > 700 ? '' : search.current.text) + event.key.toLocaleLowerCase();
      search.current = { text, time };
      const term = [...text].every(character => character === text[0]) ? text[0] : text;
      const start = term.length === 1 ? index + 1 : 0;
      const match = options.findIndex((_, offset) => options[(start + offset) % options.length].label.toLocaleLowerCase().startsWith(term));
      if (match < 0) return;
      next = (start + match) % options.length;
    } else return;
    event.preventDefault(); if (open) setActive(next); else show(next);
  }
  return <span className="select-control">
    <button ref={trigger} id={name} type="button" className="select-trigger" role="combobox"
      disabled={disabled || !options.length} aria-label={label} aria-haspopup="listbox" aria-expanded={open}
      aria-controls={`${name}-options`} aria-activedescendant={open ? `${name}-option-${active}` : undefined}
      onClick={() => { if (open) hide(); else show() }} onKeyDown={key} onBlur={() => { if (popup.current?.matches(':popover-open')) commit() }}>
      <span>{options.find(option => String(option.value) === String(value))?.label ?? String(value)}</span><CaretDownIcon aria-hidden="true" />
    </button>
    <span ref={popup} id={`${name}-options`} popover="manual" role="listbox" aria-label={label} className="select-options"
      onToggle={event => setOpen(event.newState === 'open')}>
      {options.map((option, index) => <span key={String(option.value)} id={`${name}-option-${index}`} role="option"
        data-value={String(option.value)} aria-selected={open ? index === active : index === selected}
        onPointerDown={event => event.preventDefault()} onPointerMove={() => setActive(index)} onClick={event => { event.preventDefault(); commit(index) }}>
        {option.label}
      </span>)}
    </span>
  </span>;
}
