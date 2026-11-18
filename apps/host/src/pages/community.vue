<script setup lang="ts">
import useI18n from "~/composables/useI18n";
import usePageTitle from "~/composables/usePageTitle";
import MvEmpty from "~/components/ui/MvEmpty.vue";

// Placeholder for the community library: the destination exists in the
// navigation now; its screens are frontend-redesign 05, after community-charts.
defineOptions({ name: "CommunityPage" });
const { t } = useI18n();
usePageTitle(() => ({ title: t("page-title.community") as string }));
</script>

<template>
  <section class="mv-page">
    <h1 class="t-label t-muted mv-page-heading">{{ t("nav.community") }}</h1>
    <MvEmpty>{{ t("community.coming") }}</MvEmpty>
  </section>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-page {
  max-width: var(--max-w);
  margin: 0 auto;
  padding: var(--s-8) var(--s-6);
}
@media (max-width: #{bp.$bp-md - 1px}) {
  .mv-page {
    padding: var(--s-6) var(--s-4);
  }
}
.mv-page-heading {
  margin: 0;
}
</style>
