import { createContext, useContext, useMemo } from 'react';
import pt from './locales/pt-BR.json';
import es from './locales/es.json';
export type Language = 'pt-BR' | 'en' | 'es';
export const LanguageContext = createContext<Language>('pt-BR');
export function translator(language: Language) {
  const catalogue: Record<string,string> = language === 'pt-BR' ? pt : language === 'es' ? es : {};
  const t = (key: string, values: Record<string,string | number> = {}) => (catalogue[key] ?? key).replace(/\{(\w+)\}/g,(match,name: string) => String(values[name] ?? match));
  const count = (value: number, label: string) => t(`{count} ${label}${value === 1 ? '' : 's'}`, { count: value });
  return { t, count };
}
export function useI18n() { const language = useContext(LanguageContext); return useMemo(() => translator(language), [language]) }
