<script setup lang="ts">
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
</script>

<template>
  <textarea
    class="mv-textarea"
    :class="{ 'mv-textarea--mono': mono }"
    :value="modelValue"
    :placeholder="placeholder"
    :rows="rows"
    :aria-invalid="invalid || undefined"
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
  padding: var(--s-2) var(--control-px);
  resize: vertical;
}
.mv-textarea--mono {
  font: 400 12px/1.6 var(--f-mono);
}
</style>
