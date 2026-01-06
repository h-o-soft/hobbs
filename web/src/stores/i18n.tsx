/**
 * i18n Store for HOBBS Web UI
 *
 * Provides internationalization support using @solid-primitives/i18n
 */
import { createContext, useContext, type ParentComponent, createSignal, createMemo } from 'solid-js';
import * as i18n from '@solid-primitives/i18n';
import { type Locale, type Dictionary, getDictionary, getBrowserLocale } from '../locales';

type I18nContextValue = {
  /** Current locale */
  locale: () => Locale;
  /** Set locale */
  setLocale: (locale: Locale) => void;
  /** Translation function */
  t: i18n.Translator<Dictionary>;
  /** Translate API error code */
  translateError: (code: string, fallback?: string) => string;
};

const I18nContext = createContext<I18nContextValue>();

/**
 * Storage key for persisting locale preference
 */
const LOCALE_STORAGE_KEY = 'hobbs_locale';

/**
 * Get initial locale from storage or browser
 */
function getInitialLocale(): Locale {
  try {
    const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (stored === 'ja' || stored === 'en') {
      return stored;
    }
  } catch {
    // localStorage might not be available
  }
  return getBrowserLocale();
}

/**
 * I18n Provider component
 */
export const I18nProvider: ParentComponent = (props) => {
  const [locale, setLocaleSignal] = createSignal<Locale>(getInitialLocale());

  const dict = createMemo(() => getDictionary(locale()));

  const t = i18n.translator(dict);

  const setLocale = (newLocale: Locale) => {
    setLocaleSignal(newLocale);
    try {
      localStorage.setItem(LOCALE_STORAGE_KEY, newLocale);
    } catch {
      // localStorage might not be available
    }
  };

  const translateError = (code: string, fallback?: string): string => {
    const key = `apiErrors.${code}` as any;
    const translated = t(key);
    // If translation returns the key itself (not found), use fallback
    if (translated === key || translated === undefined) {
      return fallback || t('errors.generic');
    }
    return translated;
  };

  const value: I18nContextValue = {
    locale,
    setLocale,
    t,
    translateError,
  };

  return (
    <I18nContext.Provider value={value}>
      {props.children}
    </I18nContext.Provider>
  );
};

/**
 * Hook to use i18n context
 */
export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}

/**
 * Shorthand hook to get just the translation function
 */
export function useTranslation() {
  return useI18n().t;
}
