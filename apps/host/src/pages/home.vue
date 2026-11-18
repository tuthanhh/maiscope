<script setup lang="ts">
import usePageTitle from "~/composables/usePageTitle";
import { computed } from "vue";
import useI18n from "~/composables/useI18n";
import useDarkMode from "~/composables/useDarkMode";
import { GAME } from "~/app/game";
import config from "~/app/config";
import MvButton from "~/components/ui/MvButton.vue";
import logoLightUrl from "~/assets/brand/logo.svg";
import logoDarkUrl from "~/assets/brand/logo-dark.svg";

// design-direction "Home": a picture, the brand in `display`, the subtitle,
// and two buttons — primary for Songs, ghost for Play. Community is in the
// bar. The picture is whatever `assets/home/home-hero.*` holds (see the
// README there), else the maiscope logo.
defineOptions({ name: "HomePage" });

const { t } = useI18n();
const { isDarkMode } = useDarkMode();
const logoUrl = computed(() => (isDarkMode.value ? logoDarkUrl : logoLightUrl));

const heroes = import.meta.glob<string>("../assets/home/home-hero.{webp,avif,png,jpg,jpeg}", {
  eager: true,
  query: "?url",
  import: "default",
});
// One picture is expected; with several, this order decides (the README's).
const ORDER = ["webp", "avif", "png", "jpg", "jpeg"];
const heroPath = Object.keys(heroes).sort(
  (a, b) => ORDER.indexOf(a.split(".").pop()!) - ORDER.indexOf(b.split(".").pop()!),
)[0];
if (import.meta.env.DEV && Object.keys(heroes).length > 1) {
  console.warn(`assets/home holds ${Object.keys(heroes).length} home-hero files; showing ${heroPath}`);
}
const hero = heroPath ? heroes[heroPath] : null;

usePageTitle(() => ({
  titleTemplate: "%s",
  title: `${GAME.gameTitle} | ${config.siteTitle}`,
}));
</script>

<template>
  <section class="mv-home">
    <img v-if="hero" :src="hero" alt="" class="mv-home-hero" />
    <img v-else :src="logoUrl" alt="" class="mv-home-logo" width="200" height="200" />
    <h1 class="t-display mv-home-brand">maiscope</h1>
    <p class="t-heading t-muted mv-home-subtitle">{{ t("home.subtitle") }}</p>
    <div class="mv-home-actions">
      <MvButton kind="primary" :to="{ name: 'songs' }">{{ t("nav.songs") }}</MvButton>
      <MvButton kind="ghost" :to="{ name: 'visualizer' }">{{ t("nav.play") }}</MvButton>
    </div>
  </section>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-home {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--s-4);
  // One screen less the top bar; on phones the slim top bar and the bottom bar.
  min-height: calc(100dvh - var(--nav-h));
  padding: var(--s-8) var(--s-4);
  text-align: center;
}
@media (max-width: #{bp.$bp-md - 1px}) {
  .mv-home {
    min-height: calc(100dvh - 44px - var(--nav-h) - env(safe-area-inset-bottom));
  }
}
// A fixed height, whatever the picture's shape, so nothing below moves
// when it loads; `contain` keeps it whole inside that box.
.mv-home-hero {
  width: min(100%, 420px);
  height: min(280px, 36vh);
  object-fit: contain;
  border-radius: var(--r);
  margin-bottom: var(--s-4);
}
.mv-home-logo {
  width: 200px;
  height: 200px;
}
.mv-home-brand,
.mv-home-subtitle {
  margin: 0;
}
.mv-home-actions {
  display: flex;
  gap: var(--s-3);
  margin-top: var(--s-6);
}
@media (max-width: #{bp.$bp-md - 1px}) {
  .mv-home-logo {
    width: 144px;
    height: 144px;
  }
}
</style>
