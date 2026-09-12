import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import zhMessages from '../locales/zh.json';
import enMessages from '../locales/en.json';

export type Locale = 'zh' | 'en';

export const LOCALE_STORAGE_KEY = 'app_language';

const messagesMap: Record<Locale, Record<string, any>> = {
  zh: zhMessages,
  en: enMessages,
};

interface I18nContextType {
  locale: Locale;
  setLocale: (nextLocale: Locale) => void;
  t: (path: string, params?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextType | undefined>(undefined);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
      if (stored === 'zh' || stored === 'en') {
        return stored;
      }
      // 检查浏览器语言，默认优先简体中文
      const navLang = navigator.language || '';
      if (navLang.toLowerCase().startsWith('zh')) {
        return 'zh';
      }
    }
    return 'zh';
  });

  const setLocale = useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale);
    if (typeof window !== 'undefined') {
      localStorage.setItem(LOCALE_STORAGE_KEY, nextLocale);
      // 同时触发 storage 事件以同步其他 tab
      window.dispatchEvent(new Event('languagechange'));
    }
  }, []);

  useEffect(() => {
    const handleLangChange = () => {
      const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
      if (stored && (stored === 'zh' || stored === 'en') && stored !== locale) {
        setLocaleState(stored);
      }
    };
    window.addEventListener('storage', handleLangChange);
    window.addEventListener('languagechange', handleLangChange);
    return () => {
      window.removeEventListener('storage', handleLangChange);
      window.removeEventListener('languagechange', handleLangChange);
    };
  }, [locale]);

  const t = useCallback(
    (path: string, params?: Record<string, string | number>): string => {
      const parts = path.split('.');
      let current: any = messagesMap[locale] || messagesMap.zh;

      for (const part of parts) {
        if (current && typeof current === 'object' && part in current) {
          current = current[part];
        } else {
          // 尝试兜底用英文或中文
          let fallback: any = messagesMap.en;
          for (const fPart of parts) {
            if (fallback && typeof fallback === 'object' && fPart in fallback) {
              fallback = fallback[fPart];
            } else {
              fallback = null;
              break;
            }
          }
          current = fallback ?? path;
          break;
        }
      }

      if (typeof current !== 'string') {
        return path;
      }

      let result = current;
      if (params) {
        Object.entries(params).forEach(([key, val]) => {
          result = result.replace(new RegExp(`\\{${key}\\}`, 'g'), String(val));
        });
      }
      return result;
    },
    [locale]
  );

  return (
    <I18nContext.Provider value={{ locale, setLocale, t }}>
      {children}
    </I18nContext.Provider>
  );
}

/**
 * 全局 I18n hook
 */
export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}

/**
 * 兼容 storageui (next-intl) 规范的命名空间翻译 hook
 * 示例: const t = useTranslations('Sidebar'); t('allFiles');
 */
export function useTranslations(namespace?: string) {
  const { t, locale, setLocale } = useI18n();

  const scopedT = useCallback(
    (key: string, params?: Record<string, string | number>) => {
      const fullPath = namespace ? `${namespace}.${key}` : key;
      return t(fullPath, params);
    },
    [namespace, t]
  );

  return scopedT;
}
