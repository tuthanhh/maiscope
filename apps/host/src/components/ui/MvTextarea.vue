<script setup lang="ts">
import { inject } from "vue";
import { fieldKey } from "./fieldContext";
// Chart text uses the mono variant: `meta` size, line-height 1.6.
withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    rows?: number;
    mono?: boolean;
    invalid?: boolean;
    disabled?: boolean;
  }>(),
  { rows: 8, mono: false },
);
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

// Inside an MvField: take its id and describing note.
const field = inject(fieldKey, null);
</script>

<template>
  <textarea
    :id="field?.id"
    :aria-describedby="field?.describedBy()"
    class="mv-textarea"
    :class="{ 'mv-textarea--mono': mono }"
    :value="modelValue"
    :placeholder="placeholder"
    :rows="rows"
    :aria-invalid="invalid || field?.invalid() || undefined"
    :disabled="disabled"
    @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
  />
</template>

<style scoped lang="scss">
@use "./control";

.mv-textarea {
  @include control.control;
  width: 100%;
  height: auto;
  // The table's padding is `0 var(--control-px)`; a textarea's first line
  // needs the room a single-line control gets from its height, so it keeps
  // the vertical half of `--control-h`'s leading.
  padding: var(--s-2) var(--control-px);
  resize: vertical;
}
.mv-textarea--mono {
  font: 400 12px/1.6 var(--f-mono);
}
</style>
