import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import ts from 'typescript';
import { systemLanguage, translator } from './i18n';
import pt from './locales/pt-BR.json';
import es from './locales/es.json';

describe('interface languages', () => {
  it('uses the first supported system language and falls back to English', () => {
    expect(systemLanguage(['pt-BR', 'en-US'])).toBe('pt-BR');
    expect(systemLanguage(['pt_PT'])).toBe('pt-BR');
    expect(systemLanguage(['es-MX', 'en-US'])).toBe('es');
    expect(systemLanguage(['en-GB', 'pt-BR'])).toBe('en');
    expect(systemLanguage(['fr-FR', 'ES-es'])).toBe('es');
    expect(systemLanguage(['fr-FR', 'de-DE'])).toBe('en');
    expect(systemLanguage([])).toBe('en');
  });
  it('interpolates user text literally and handles singular, plural and zero', () => {
    const { t, count } = translator('pt-BR');
    expect(t('No courses match “{query}”.', { query: '{count} <script> café' })).toBe('Nenhum curso corresponde a “{count} <script> café”.');
    expect(count(1, 'lesson')).toBe('1 aula');
    expect(count(0, 'lesson')).toBe('0 aulas');
    expect(translator('es').count(2, 'lesson')).toBe('2 lecciones');
    expect(translator('en').count(2, 'lesson')).toBe('2 lessons');
    expect(t('Unknown diagnostic')).toBe('Unknown diagnostic');
  });
  it('covers interface keys in both catalogues and preserves placeholders', () => {
    const keys = new Set();
    function visit(node) {
      if (ts.isCallExpression(node) && node.expression.getText() === 't') {
        const first = node.arguments[0];
        if (first && ts.isStringLiteral(first)) keys.add(first.text);
        else if (first && ts.isConditionalExpression(first)) {
          for (const branch of [first.whenTrue, first.whenFalse]) if (ts.isStringLiteral(branch)) keys.add(branch.text);
        }
      }
      ts.forEachChild(node, visit);
    }
    for (const file of readdirSync('src').filter(f => f.endsWith('.tsx'))) visit(ts.createSourceFile(file, readFileSync(`src/${file}`, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX));
    expect(Object.keys(pt).sort()).toEqual(Object.keys(es).sort());
    for (const key of keys) expect(pt, key).toHaveProperty(key);
    const placeholders = (text) => text.match(/\{\w+\}/g)?.sort() ?? [];
    for (const catalogue of [pt, es]) for (const [key, value] of Object.entries(catalogue)) {
      expect(value.trim().length, key).toBeGreaterThan(0);
      expect(placeholders(value), key).toEqual(placeholders(key));
    }
  });
});
