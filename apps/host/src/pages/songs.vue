<script setup lang="ts">
import usePageTitle from "~/composables/usePageTitle";
import useI18n from "~/composables/useI18n";
import BrowseView from "~/components/BrowseView.vue";

defineOptions({ name: "GameSongsPage" });

const { t } = useI18n();

usePageTitle(() => ({ title: t("page-title.songs") as string }));
</script>

<template>
  <BrowseView />
</template>
