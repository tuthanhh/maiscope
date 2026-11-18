<script setup lang="ts">
import { provide, useId } from "vue";
import { fieldKey } from "./fieldContext";

// design-direction "Field": a label above a control, help or error under it.
// Errors are framed by the control's border, not coloured. The control inside
// takes its id and `aria-describedby` from here, so the label and the note
// belong to it. A `group` field holds several controls (a range, a chip row):
// its label names the group, and each control carries its own name.
const props = defineProps<{ label?: string; help?: string; error?: string; group?: boolean }>();

const id = useId();
const noteId = `${id}-note`;
const labelId = `${id}-label`;
provide(
  fieldKey,
  props.group
    ? null
    : {
        id,
        describedBy: () => (props.error || props.help ? noteId : undefined),
        invalid: () => !!props.error,
      },
);
</script>

<template>
  <div
    class="mv-field"
    :role="group ? 'group' : undefined"
    :aria-labelledby="group && label ? labelId : undefined"
  >
    <span v-if="group && label" :id="labelId" class="mv-field-label t-label t-muted">
      {{ label }}
    </span>
    <label v-else-if="label" class="mv-field-label t-label t-muted" :for="id">
      {{ label }}
    </label>
    <slot />
    <p v-if="error" :id="noteId" class="mv-field-note t-meta" role="alert">{{ error }}</p>
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
