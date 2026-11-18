<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import useI18n from "~/composables/useI18n";
import useDarkMode from "~/composables/useDarkMode";
import useEngineInView from "~/composables/useEngineInView";
import MvButton from "~/components/ui/MvButton.vue";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvMenu from "./MvMenu.vue";
import type { IconName } from "~/components/ui/icons";
import logoLightUrl from "~/assets/brand/logo.svg";
import logoDarkUrl from "~/assets/brand/logo-dark.svg";

// design-direction "Navigation": three destinations. A top bar on desktop; on
// phones a slim top bar (brand, menu) and a bottom bar of icon-above-label
// destinations that hides while the engine canvas is in view.
const { t } = useI18n();
const { isDarkMode } = useDarkMode();
// The dark theme's logo lightens the handle, which is ~2.3:1 on black.
const logoUrl = computed(() => (isDarkMode.value ? logoDarkUrl : logoLightUrl));
const engineInView = useEngineInView();
const route = useRoute();

function isActive(name: string): boolean {
  return route.name === name;
}

const destinations = computed<{ name: string; label: string; icon: IconName }[]>(() => [
  { name: "songs", label: t("nav.songs") as string, icon: "songs" },
  { name: "community", label: t("nav.community") as string, icon: "community" },
  { name: "visualizer", label: t("nav.play") as string, icon: "play" },
]);
</script>

<template>
  <header class="mv-topbar">
    <RouterLink :to="{ name: 'home' }" class="mv-brand">
      <img :src="logoUrl" alt="" class="mv-brand-logo" width="28" height="28" />
      maiscope
    </RouterLink>
    <nav class="mv-topbar-links" aria-label="Main">
      <RouterLink
        v-for="d in destinations"
        :key="d.name"
        :to="{ name: d.name }"
        class="mv-topbar-link t-body"
        :class="{ 'mv-active': isActive(d.name) }"
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

  <!-- Hidden outright while the engine is in view, not slid away: the design
       names no motion for it, and `inert` keeps it out of Tab order and the
       accessibility tree while it is gone. -->
  <nav
    class="mv-bottombar"
    :class="{ 'mv-bottombar--hidden': engineInView }"
    :inert="engineInView"
    aria-label="Main"
  >
    <RouterLink
      v-for="d in destinations"
      :key="d.name"
      :to="{ name: d.name }"
      class="mv-bottombar-link"
      :class="{ 'mv-active': isActive(d.name) }"
    >
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
  display: inline-flex;
  align-items: center;
  gap: var(--s-2);
  justify-self: start;
  font: 700 18px/24px var(--f-display);
  letter-spacing: -0.01em;
  color: var(--fg);
  text-decoration: none;
}
// The logo is the one coloured thing in the shell, by the maintainer's
// choice; it reads on both themes as drawn.
.mv-brand-logo {
  display: block;
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
.mv-topbar-link.mv-active {
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
    height: calc(var(--nav-h) + env(safe-area-inset-bottom));
    padding-bottom: env(safe-area-inset-bottom);
    background: var(--bg);
    border-top: var(--line-w) solid var(--line);
  }
  .mv-bottombar--hidden {
    visibility: hidden;
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
  .mv-bottombar-link.mv-active {
    color: var(--fg);
  }
}
</style>
