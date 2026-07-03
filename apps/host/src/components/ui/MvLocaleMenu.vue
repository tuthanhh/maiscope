<script setup lang="ts">
import { ref, computed } from "vue";
import useI18n from "~/composables/useI18n";
import useGtag from "~/composables/useGtag";
import { GAME } from "~/app/game";
import type { LocaleObject } from "~/app/i18n";
import MvIcon from "./MvIcon.vue";

const i18n = useI18n();
const gtag = useGtag();

const open = ref(false);
const current = computed(() => i18n.localeProperties);
const localeOptions = computed(() => i18n.locales as LocaleObject[]);

function choose(code: string): void {
  i18n.setLocale(code);
  open.value = false;
  gtag("event", "LocaleChanged", {
    gameCode: GAME.gameCode,
    eventSource: "MvLocaleMenu",
    locale: code,
  });
}
</script>

<template>
  <div class="mv-locale" @focusout="open = false" tabindex="-1">
    <button type="button" class="mv-btn mv-locale-trigger" @click="open = !open">
      <MvIcon name="globe" :size="14" />
      <span>{{ current.abbr }}</span>
      <MvIcon name="chevron" :size="13" />
    </button>
    <div v-if="open" class="mv-locale-menu">
      <button
        v-for="o in localeOptions"
        :key="o.code"
        type="button"
        class="mv-btn mv-locale-item"
        :class="{ active: o.code === current.code }"
        @mousedown.prevent="choose(o.code)"
      >
        <span>{{ o.name }}</span>
        <span class="mv-locale-abbr">{{ o.abbr }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.mv-locale {
  position: relative;
  outline: none;
}
.mv-locale-trigger {
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: 1px solid var(--mv-bd);
  color: var(--mv-fg);
  font-family: var(--mv-mono);
  font-size: 11px;
  padding: 5px 8px;
  border-radius: 3px;
}
.mv-locale-menu {
  position: absolute;
  right: 0;
  top: 34px;
  z-index: 60;
  min-width: 150px;
  background: var(--mv-panel);
  border: 1px solid var(--mv-bd);
  border-radius: 3px;
  overflow: hidden;
}
.mv-locale-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  background: none;
  border: none;
  border-bottom: 1px solid var(--mv-bd2);
  color: var(--mv-fg);
  font-family: var(--mv-mono);
  font-size: 11px;
  padding: 7px 10px;
  text-align: left;
}
.mv-locale-item:last-child {
  border-bottom: none;
}
.mv-locale-item.active {
  background: var(--mv-invbg);
  color: var(--mv-inv);
}
.mv-locale-abbr {
  font-size: 9px;
  color: var(--mv-mut);
  text-transform: uppercase;
}
.mv-locale-item.active .mv-locale-abbr {
  color: var(--mv-inv);
}
</style>
