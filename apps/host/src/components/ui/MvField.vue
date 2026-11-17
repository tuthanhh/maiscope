<script setup lang="ts">
import { provide, useId } from "vue";
import { fieldKey } from "./fieldContext";

// design-direction "Field": a label above a control, help or error under it.
// Errors are framed by the control's border, not coloured. The control inside
// takes its id and `aria-describedby` from here, so the label and the note
// belong to it.
const props = defineProps<{ label?: string; help?: string; error?: string }>();

const id = useId();
const noteId = `${id}-note`;
provide(fieldKey, {
  id,
  describedBy: () => (props.error || props.help ? noteId : undefined),
  invalid: () => !!props.error,
});
</script>

<template>
  <div class="mv-field">
    <label v-if="label" class="mv-field-label t-label t-muted" :for="id">
      {{ label }}
    </label>
    <slot />
    <p v-if="error" :id="noteId" class="mv-field-note t-meta">{{ error }}</p>
    <p v-else-if="help" :id="noteId" class="mv-field-note t-meta t-muted">{{ help }}</p>
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
