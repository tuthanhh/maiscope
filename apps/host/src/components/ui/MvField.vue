<script setup lang="ts">
// Labelled field wrapper: tiny uppercase label above a control slot.
defineProps<{ label: string }>();
</script>

<template>
  <div class="mv-field">
    <span class="mv-label">{{ label }}</span>
    <slot />
  </div>
</template>

<style scoped>
.mv-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
</style>
