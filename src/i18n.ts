import { createContext, useContext, useMemo } from 'react';
import pt from './locales/pt-BR.json';
import es from './locales/es.json';
export type Language = 'pt-BR' | 'en' | 'es';
export function systemLanguage(languages: readonly string[] = typeof navigator === 'undefined' ? [] : navigator.languages.length ? navigator.languages : [navigator.language]): Language {
  for (const locale of languages) {
    const language = locale.split(/[-_]/)[0].toLowerCase();
    if (language === 'pt') return 'pt-BR';
    if (language === 'es' || language === 'en') return language;
  }
  return 'en';
}
export const LanguageContext = createContext<Language>(systemLanguage());
export function translator(language: Language) {
  const catalogue: Record<string,string> = language === 'pt-BR' ? pt : language === 'es' ? es : {};
  const t = (key: string, values: Record<string,string | number> = {}) => (catalogue[key] ?? key).replace(/\{(\w+)\}/g,(match,name: string) => String(values[name] ?? match));
  const count = (value: number, label: string) => t(`{count} ${label}${value === 1 ? '' : 's'}`, { count: value });
  return { t, count };
}
export function useI18n() { const language = useContext(LanguageContext); return useMemo(() => translator(language), [language]) }
