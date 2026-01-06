/**
 * Locale exports and type definitions for HOBBS Web UI i18n
 */
import * as i18n from '@solid-primitives/i18n';
import { dict as jaDict } from './ja';
import { dict as enDict } from './en';

export type Locale = 'ja' | 'en';
export type RawDictionary = typeof jaDict;
export type Dictionary = i18n.Flatten<RawDictionary>;

/**
 * Available dictionaries by locale
 */
export const dictionaries: Record<Locale, RawDictionary> = {
  ja: jaDict,
  en: enDict,
};

/**
 * Get flattened dictionary for a locale
 */
export function getDictionary(locale: Locale): Dictionary {
  return i18n.flatten(dictionaries[locale]);
}

/**
 * Default locale
 */
export const defaultLocale: Locale = 'ja';

/**
 * Get browser locale, falling back to default
 */
export function getBrowserLocale(): Locale {
  const browserLang = navigator.language.split('-')[0];
  if (browserLang === 'ja' || browserLang === 'en') {
    return browserLang;
  }
  return defaultLocale;
}
