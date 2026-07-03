<script setup lang="ts">
import useI18n from "~/composables/useI18n";

const { t } = useI18n();
</script>

<template>
  <div class="mv-loading">
    <div class="mv-spinner" />
    <span class="mv-label">{{ t("description.loading") }}</span>
  </div>
</template>

<style scoped>
.mv-loading {
  position: fixed;
  inset: 0;
  z-index: 80;
  background: var(--mv-ov);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}
.mv-spinner {
  width: 40px;
  height: 40px;
  border: 2px solid var(--mv-bd);
  border-top-color: var(--mv-fg);
  border-radius: 50%;
  animation: mv-spin 0.8s linear infinite;
}
@keyframes mv-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
