<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId, watch } from "vue";
import useI18n from "~/composables/useI18n";
import useGtag from "~/composables/useGtag";
import config from "~/app/config";
import { GAME } from "~/app/game";
import type { LocaleObject } from "~/app/i18n";
import MvButton from "~/components/ui/MvButton.vue";
import MvSelect from "~/components/ui/MvSelect.vue";

// The account menu: Locale and About live here, and the external links the
// old header carried. Sign in and Reports join it when accounts reach the UI
// (frontend-redesign 05). A disclosure, not an ARIA menu: its contents are a
// select and links, reached with Tab.
const i18n = useI18n();
const { t } = i18n;
const gtag = useGtag();
const panelId = useId();
const root = ref<HTMLElement | null>(null);
const open = ref(false);

const locale = computed({
  get: () => i18n.localeProperties.code,
  set: (code: string) => {
    i18n.setLocale(code);
    gtag("event", "LocaleChanged", {
      gameCode: GAME.gameCode,
      eventSource: "MvMenu",
      locale: code,
    });
  },
});
const localeOptions = computed(() =>
  (i18n.locales as LocaleObject[]).map((l) => ({ value: l.code, text: l.name ?? l.code })),
);

// Close on a press outside. `pointerdown` rather than `focusout`: a click on
// the panel's padding, or a Safari click that never moves focus, would
// otherwise close it wrongly or not at all.
function onPointerDown(event: PointerEvent): void {
  if (!root.value?.contains(event.target as Node)) open.value = false;
}
// Escape is heard on the document: a click on the panel's padding leaves
// focus on <body>, outside this element.
function onKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  open.value = false;
  root.value?.querySelector<HTMLElement>(".mv-menu-trigger")?.focus();
}
function listen(on: boolean): void {
  if (on) {
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
  } else {
    document.removeEventListener("pointerdown", onPointerDown);
    document.removeEventListener("keydown", onKeyDown);
  }
}
watch(open, listen);
onBeforeUnmount(() => listen(false));

// Close when focus leaves by keyboard (Tab past the last link).
function onFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget as Node | null;
  if (next && !root.value?.contains(next)) open.value = false;
}
</script>

<template>
  <div ref="root" class="mv-menu" @focusout="onFocusOut">
    <MvButton
      kind="icon"
      icon="user"
      class="mv-menu-trigger"
      :label="t('nav.menu') as string"
      :aria-expanded="open"
      :aria-controls="panelId"
      @click="open = !open"
    />
    <div v-if="open" :id="panelId" class="mv-menu-panel">
      <label class="mv-menu-row">
        <span class="t-label t-muted">{{ t("nav.language") }}</span>
        <MvSelect v-model="locale" :options="localeOptions" :width="180" />
      </label>
      <MvButton kind="text" :to="{ name: 'about' }" @click="open = false">
        {{ t("page-title.about") }}
      </MvButton>
      <a class="mv-menu-link t-body" :href="config.siteReportUrl" target="_blank" rel="noopener">
        {{ t("page-title.bug-report") }}
      </a>
      <a class="mv-menu-link t-body" :href="config.sourceCodeUrl" target="_blank" rel="noopener">
        {{ t("page-title.source-code") }}
      </a>
    </div>
  </div>
</template>

<style scoped>
.mv-menu {
  position: relative;
}
.mv-menu-panel {
  position: absolute;
  top: calc(100% + var(--s-2));
  right: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--s-1);
  min-width: 212px;
  padding: var(--s-4);
  background: var(--bg-panel);
  border: var(--line-w) solid var(--line);
  border-radius: var(--r);
}
.mv-menu-row {
  display: flex;
  flex-direction: column;
  gap: var(--s-1);
  margin-bottom: var(--s-2);
}
.mv-menu-link {
  min-height: var(--control-h);
  display: flex;
  align-items: center;
  color: var(--fg);
  text-decoration: none;
}
.mv-menu-link:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
