<script setup lang="ts">
import { computed, ref } from "vue";
import useI18n from "~/composables/useI18n";
import config from "~/app/config";
import type { LocaleObject } from "~/app/i18n";
import MvButton from "~/components/ui/MvButton.vue";
import MvSelect from "~/components/ui/MvSelect.vue";

// The account menu: Locale and About live here, and the external links the
// old header carried. Sign in and Reports join it when accounts reach the UI
// (frontend-redesign 05).
const { t, ...i18n } = useI18n();
const open = ref(false);

const locale = computed({
  get: () => i18n.localeProperties.code,
  set: (code: string) => i18n.setLocale(code),
});
const localeOptions = computed(() =>
  (i18n.locales as LocaleObject[]).map((l) => ({ value: l.code, text: l.name ?? l.code })),
);

function close(event: FocusEvent): void {
  const next = event.relatedTarget as Node | null;
  if (!next || !(event.currentTarget as Node).contains(next)) open.value = false;
}
</script>

<template>
  <div class="mv-menu" @focusout="close" @keydown.escape="open = false">
    <MvButton
      kind="icon"
      icon="user"
      :label="t('nav.menu') as string"
      :aria-expanded="open"
      @click="open = !open"
    />
    <div v-if="open" class="mv-menu-panel" role="menu">
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
