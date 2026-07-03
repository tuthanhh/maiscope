<script setup lang="ts">
defineProps<{ modelValue: string; placeholder?: string; width?: number }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}
</script>

<template>
  <input
    class="mv-input"
    :value="modelValue"
    :placeholder="placeholder"
    :style="{ width: width != null ? `${width}px` : undefined }"
    @input="onInput"
  />
</template>

<style scoped>
.mv-input {
  background: var(--mv-panel);
  border: 1px solid var(--mv-bd);
  border-radius: 3px;
  color: var(--mv-fg);
  font-family: var(--mv-mono);
  font-size: 12px;
  padding: 7px 9px;
  box-sizing: border-box;
}
</style>
