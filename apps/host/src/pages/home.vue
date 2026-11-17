<script setup lang="ts">
import usePageTitle from "~/composables/usePageTitle";
import useI18n from "~/composables/useI18n";
import { GAME } from "~/app/game";
import config from "~/app/config";
import MvButton from "~/components/ui/MvButton.vue";

// design-direction "Home": the brand in `display`, the subtitle, and two
// buttons — primary for Songs, ghost for Play. Community is in the bar.
defineOptions({ name: "HomePage" });

const { t } = useI18n();

usePageTitle(() => ({
  titleTemplate: "%s",
  title: `${GAME.gameTitle} | ${config.siteTitle}`,
}));
</script>

<template>
  <section class="mv-home">
    <h1 class="t-display mv-home-brand">maiscope</h1>
    <p class="t-body t-muted mv-home-subtitle">maimai visualization project</p>
    <div class="mv-home-actions">
      <MvButton kind="primary" :to="{ name: 'songs' }">{{ t("nav.songs") }}</MvButton>
      <MvButton kind="ghost" :to="{ name: 'visualizer' }">{{ t("nav.play") }}</MvButton>
    </div>
  </section>
</template>

<style scoped>
.mv-home {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--s-4);
  min-height: calc(100vh - var(--nav-h) * 2);
  padding: var(--s-8) var(--s-4);
  text-align: center;
}
.mv-home-brand,
.mv-home-subtitle {
  margin: 0;
}
.mv-home-actions {
  display: flex;
  gap: var(--s-3);
  margin-top: var(--s-4);
}
</style>
