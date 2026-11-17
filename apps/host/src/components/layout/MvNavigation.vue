<script setup lang="ts">
import { computed } from "vue";
import useI18n from "~/composables/useI18n";
import useDarkMode from "~/composables/useDarkMode";
import useEngineInView from "~/composables/useEngineInView";
import MvButton from "~/components/ui/MvButton.vue";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvMenu from "./MvMenu.vue";
import type { IconName } from "~/components/ui/icons";

// design-direction "Navigation": three destinations. A top bar on desktop; on
// phones a slim top bar (brand, menu) and a bottom bar of icon-above-label
// destinations that hides while the engine canvas is in view.
const { t } = useI18n();
const { isDarkMode } = useDarkMode();
const engineInView = useEngineInView();

const destinations = computed<{ name: string; label: string; icon: IconName }[]>(() => [
  { name: "songs", label: t("nav.songs") as string, icon: "songs" },
  { name: "community", label: t("nav.community") as string, icon: "community" },
  { name: "visualizer", label: t("nav.play") as string, icon: "play" },
]);
</script>

<template>
  <header class="mv-topbar">
    <RouterLink :to="{ name: 'home' }" class="mv-brand t-strong">maiscope</RouterLink>
    <nav class="mv-topbar-links" aria-label="Main">
      <RouterLink
        v-for="d in destinations"
        :key="d.name"
        :to="{ name: d.name }"
        class="mv-topbar-link t-body"
      >
        {{ d.label }}
      </RouterLink>
    </nav>
    <div class="mv-topbar-actions">
      <MvButton
        kind="icon"
        :icon="isDarkMode ? 'sun' : 'moon'"
        :label="t('nav.theme') as string"
        @click="isDarkMode = !isDarkMode"
      />
      <MvMenu />
    </div>
  </header>

  <nav class="mv-bottombar" :class="{ 'mv-bottombar--hidden': engineInView }" aria-label="Main">
    <RouterLink v-for="d in destinations" :key="d.name" :to="{ name: d.name }" class="mv-bottombar-link">
      <MvIcon :name="d.icon" :size="20" />
      <span class="t-label">{{ d.label }}</span>
    </RouterLink>
  </nav>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-topbar {
  position: sticky;
  top: 0;
  z-index: 20;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  height: var(--nav-h);
  padding: 0 var(--s-6);
  background: var(--bg);
  border-bottom: var(--line-w) solid var(--line);
}
.mv-brand {
  color: var(--fg);
  text-decoration: none;
}
.mv-topbar-links {
  display: flex;
  gap: var(--s-6);
}
.mv-topbar-link {
  color: var(--fg-muted);
  text-decoration: none;
  transition: color var(--dur);
}
.mv-topbar-link:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
.mv-topbar-link.router-link-active {
  color: var(--fg);
}
.mv-topbar-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--s-1);
}

.mv-bottombar {
  display: none;
}

@media (max-width: #{bp.$bp-md - 1px}) {
  .mv-topbar {
    grid-template-columns: 1fr auto;
    height: 44px;
    padding: 0 var(--s-4);
  }
  .mv-topbar-links {
    display: none;
  }
  .mv-bottombar {
    position: fixed;
    inset: auto 0 0 0;
    z-index: 20;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    height: var(--nav-h);
    padding-bottom: env(safe-area-inset-bottom);
    background: var(--bg);
    border-top: var(--line-w) solid var(--line);
    transition: transform var(--dur-slow);
  }
  .mv-bottombar--hidden {
    transform: translateY(100%);
  }
  .mv-bottombar-link {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    color: var(--fg-muted);
    text-decoration: none;
  }
  .mv-bottombar-link.router-link-active {
    color: var(--fg);
  }
}
</style>
