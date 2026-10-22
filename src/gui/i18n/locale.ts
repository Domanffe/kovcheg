import type { Messages } from "./en";
import { en } from "./en";
import { ru } from "./ru";

export type Locale = "en" | "ru";
export type LocalePref = "system" | Locale;

export const LOCALE_STORAGE_KEY = "kovcheg.locale";

const catalogs: Record<Locale, Messages> = { en, ru };

export function detectSystemLocale(): Locale {
  const langs =
    typeof navigator !== "undefined"
      ? [navigator.language, ...(navigator.languages ?? [])]
      : [];
  for (const lang of langs) {
    if (lang.toLowerCase().startsWith("ru")) return "ru";
  }
  return "en";
}

export function readLocalePref(): LocalePref {
  if (typeof localStorage === "undefined") return "system";
  const raw = localStorage.getItem(LOCALE_STORAGE_KEY);
  if (raw === "en" || raw === "ru" || raw === "system") return raw;
  return "system";
}

export function writeLocalePref(pref: LocalePref): void {
  localStorage.setItem(LOCALE_STORAGE_KEY, pref);
}

export function resolveLocale(pref: LocalePref): Locale {
  return pref === "system" ? detectSystemLocale() : pref;
}

export function getMessages(locale: Locale): Messages {
  return catalogs[locale];
}

export function format(
  template: string,
  vars: Record<string, string | number>,
): string {
  return template.replace(/\{(\w+)\}/g, (_, key: string) => String(vars[key] ?? ""));
}
