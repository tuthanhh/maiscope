<script setup lang="ts">
// design-direction "Field": a label above a control, help or error under it.
// Errors are framed by the control's border, not coloured.
defineProps<{ label?: string; help?: string; error?: string; for?: string }>();
</script>

<template>
  <div class="mv-field">
    <label v-if="label" class="mv-field-label t-label t-muted" :for="$props.for">
      {{ label }}
    </label>
    <slot />
    <p v-if="error" class="mv-field-note t-meta">{{ error }}</p>
    <p v-else-if="help" class="mv-field-note t-meta t-muted">{{ help }}</p>
  </div>
</template>

<style scoped>
.mv-field {
  display: flex;
  flex-direction: column;
  gap: var(--s-1);
  margin-bottom: var(--s-4);
}
.mv-field-note {
  margin: 0;
}
</style>
