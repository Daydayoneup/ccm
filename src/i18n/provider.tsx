import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import { getAppSetting, setAppSetting } from '@/lib/tauri-api';
import { en, type I18nMessages } from './en';
import { zhCN } from './zh-CN';

export type Locale = 'system' | 'zh-CN' | 'en';
export type ResolvedLocale = Exclude<Locale, 'system'>;

const messageMap: Record<ResolvedLocale, I18nMessages> = {
  en,
  'zh-CN': zhCN,
};

interface I18nContextValue {
  locale: Locale;
  resolvedLocale: ResolvedLocale;
  setLocale: (locale: Locale) => Promise<void>;
  t: (key: string, params?: Record<string, string | number>) => string;
  formatRelativeTime: (value: string | Date | number) => string;
  formatNumber: (value: number) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function readPath(source: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc && typeof acc === 'object' && part in acc) {
      return (acc as Record<string, unknown>)[part];
    }
    return undefined;
  }, source);
}

function interpolate(template: string, params?: Record<string, string | number>) {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, key: string) => String(params[key] ?? `{${key}}`));
}

function detectSystemLocale(): ResolvedLocale {
  const locale = typeof navigator !== 'undefined' ? navigator.language : 'en';
  if (locale.startsWith('zh')) {
    return 'zh-CN';
  }
  return 'en';
}

function resolveLocale(locale: Locale): ResolvedLocale {
  if (locale === 'system') {
    return detectSystemLocale();
  }
  return locale;
}

function createTranslator(resolvedLocale: ResolvedLocale) {
  return (key: string, params?: Record<string, string | number>) => {
    const current = readPath(messageMap[resolvedLocale], key);
    const fallback = readPath(en, key);
    const template =
      typeof current === 'string'
        ? current
        : typeof fallback === 'string'
          ? fallback
          : key;
    return interpolate(template, params);
  };
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>('system');

  useEffect(() => {
    let mounted = true;
    getAppSetting('locale')
      .then((stored) => {
        if (!mounted) return;
        if (stored === 'zh-CN' || stored === 'en' || stored === 'system') {
          setLocaleState(stored);
        }
      })
      .catch(() => {
        // Ignore missing locale setting and continue following system preference.
      });
    return () => {
      mounted = false;
    };
  }, []);

  const resolvedLocale = resolveLocale(locale);
  const t = useMemo(() => createTranslator(resolvedLocale), [resolvedLocale]);

  const setLocale = useCallback(async (nextLocale: Locale) => {
    setLocaleState(nextLocale);
    try {
      await setAppSetting('locale', nextLocale);
    } catch (error) {
      console.error('Failed to persist locale setting:', error);
    }
  }, []);

  const formatRelativeTime = useCallback(
    (value: string | Date | number) => {
      const date = value instanceof Date ? value : new Date(value);
      const diffMs = date.getTime() - Date.now();
      const absMs = Math.abs(diffMs);
      const rtf = new Intl.RelativeTimeFormat(resolvedLocale, { numeric: 'auto' });

      const minute = 60_000;
      const hour = 60 * minute;
      const day = 24 * hour;

      if (absMs < minute) {
        return t('common.relative.justNow');
      }
      if (absMs < hour) {
        return rtf.format(Math.round(diffMs / minute), 'minute');
      }
      if (absMs < day) {
        return rtf.format(Math.round(diffMs / hour), 'hour');
      }
      if (absMs < day * 30) {
        return rtf.format(Math.round(diffMs / day), 'day');
      }
      return new Intl.DateTimeFormat(resolvedLocale, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      }).format(date);
    },
    [resolvedLocale, t]
  );

  const formatNumber = useCallback(
    (value: number) => new Intl.NumberFormat(resolvedLocale).format(value),
    [resolvedLocale]
  );

  const contextValue = useMemo<I18nContextValue>(
    () => ({
      locale,
      resolvedLocale,
      setLocale,
      t,
      formatRelativeTime,
      formatNumber,
    }),
    [formatNumber, formatRelativeTime, locale, resolvedLocale, setLocale, t]
  );

  return <I18nContext.Provider value={contextValue}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (context) return context;

  const fallbackLocale: Locale = 'system';
  const resolvedLocale = resolveLocale(fallbackLocale);
  return {
    locale: fallbackLocale,
    resolvedLocale,
    setLocale: async () => undefined,
    t: createTranslator(resolvedLocale),
    formatRelativeTime: (value: string | Date | number) => {
      const date = value instanceof Date ? value : new Date(value);
      return new Intl.DateTimeFormat(resolvedLocale, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      }).format(date);
    },
    formatNumber: (value: number) => new Intl.NumberFormat(resolvedLocale).format(value),
  } satisfies I18nContextValue;
}
