<script setup lang="ts">
import MvButton from "./MvButton.vue";

// design-direction "Error state": inline, never a toast, framed not coloured.
// The short form is a label, a sentence and an optional retry; the upload
// rejection fills the default slot with its full form.
defineProps<{ label: string; message?: string; retryLabel?: string }>();
defineEmits<{ retry: [] }>();
</script>

<template>
  <div class="mv-error" role="alert">
    <div class="t-label">{{ label }}</div>
    <p v-if="message" class="mv-error-message t-body">{{ message }}</p>
    <slot />
    <MvButton v-if="retryLabel" kind="text" @click="$emit('retry')">{{ retryLabel }}</MvButton>
  </div>
</template>

<style scoped>
.mv-error {
  border: var(--line-w) solid var(--danger-fg);
  border-radius: var(--r);
  padding: var(--s-4);
  color: var(--danger-fg);
}
.mv-error-message {
  margin: var(--s-2) 0 0;
}
</style>
