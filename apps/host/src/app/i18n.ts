import { computed, type ComputedRef } from "vue";
import { createI18n } from "vue-i18n";
import type { Composer } from "vue-i18n";
import locales from "~/locales";

export interface LocaleObject {
  code: string;
  iso?: string;
  abbr?: string;
  name?: string;
  file?: string;
  [key: string]: unknown;
}

export interface I18nFacade {
  t: Composer["t"];
  locale: string;
  locales: LocaleObject[];
  localeProperties: LocaleObject;
  setLocale: (code: string) => void;
  onLanguageSwitched: (oldLocale: string, newLocale: string) => void;
}

function createI18nFacade(
  composer: Composer,
  localeList: LocaleObject[],
): I18nFacade {
  let onSwitched: (oldLocale: string, newLocale: string) => void = () => {};

  const currentProps: ComputedRef<LocaleObject> = computed(
    () =>
      localeList.find((l) => l.code === composer.locale.value) ?? localeList[0],
  );

  return {
    t: composer.t.bind(composer),
    get locale() {
      return composer.locale.value;
    },
    get locales() {
      return localeList;
    },
    get localeProperties() {
      return currentProps.value;
    },
    setLocale(code: string) {
      const old = composer.locale.value;
      composer.locale.value = code;
      onSwitched(old, code);
    },
    set onLanguageSwitched(fn: (oldLocale: string, newLocale: string) => void) {
      onSwitched = fn;
    },
    get onLanguageSwitched() {
      return onSwitched;
    },
  };
}

const yamlModules = import.meta.glob("../locales/*.yaml", {
  eager: true,
  import: "default",
}) as Record<string, Record<string, unknown>>;

const messages: Record<string, any> = {};
for (const [path, mod] of Object.entries(yamlModules)) {
  const code = path.replace(/^.*\/(.+)\.yaml$/, "$1");
  messages[code] = mod;
}

const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: "en",
  fallbackLocale: "en",
  messages,
});

export const i18nFacade = createI18nFacade(i18n.global as never, locales);

export default i18n;
