import { useMemo } from "react";
import { zhTW } from "./zh-TW";
import { enUS } from "./en-US";
import type { I18nKeys } from "./zh-TW";

const locales: Record<string, I18nKeys> = {
  "zh-TW": zhTW,
  en: enUS,
  "en-US": enUS,
};

function getBrowserLocale(): string {
  const lang = navigator.language ?? "zh-TW";
  if (lang in locales) return lang;
  const prefix = lang.split("-")[0];
  if (prefix && prefix in locales) return prefix;
  return "zh-TW";
}

/** 回傳目前語言的翻譯物件。語言由 navigator.language 決定，未來可從設定覆蓋。 */
export function useI18n(overrideLocale?: string): I18nKeys {
  return useMemo(() => {
    const key = overrideLocale ?? getBrowserLocale();
    return locales[key] ?? zhTW;
  }, [overrideLocale]);
}