<script setup lang="ts">
import MvIcon from "./MvIcon.vue";

export type SelectOption = { value: string; text: string };

defineProps<{
  modelValue: string;
  options: SelectOption[];
  placeholder?: string;
  width?: number;
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

function onChange(event: Event): void {
  emit("update:modelValue", (event.target as HTMLSelectElement).value);
}
</script>

<template>
  <div class="mv-select-wrap">
    <select
      class="mv-select"
      :class="{ 'mv-select--empty': modelValue === '' }"
      :value="modelValue"
      :style="{ width: width != null ? `${width}px` : undefined }"
      @change="onChange"
    >
      <option value="">{{ placeholder }}</option>
      <option v-for="o in options" :key="o.value" :value="o.value">
        {{ o.text }}
      </option>
    </select>
    <span class="mv-select-chevron">
      <MvIcon name="chevron" :size="13" />
    </span>
  </div>
</template>

<style scoped>
.mv-select-wrap {
  position: relative;
  display: inline-block;
}
.mv-select {
  background: var(--mv-panel);
  border: 1px solid var(--mv-bd);
  border-radius: 3px;
  color: var(--mv-fg);
  font-family: var(--mv-mono);
  font-size: 12px;
  padding: 7px 26px 7px 9px;
  box-sizing: border-box;
}
.mv-select--empty {
  color: var(--mv-mut);
}
.mv-select-chevron {
  position: absolute;
  right: 8px;
  top: 9px;
  pointer-events: none;
  color: var(--mv-mut);
}
/* Native option list can't inherit the dark canvas reliably; keep readable. */
.mv-select option {
  color: #111;
}
</style>
