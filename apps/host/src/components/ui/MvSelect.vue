<script setup lang="ts">
import { inject } from "vue";
import { fieldKey } from "./fieldContext";
import MvIcon from "./MvIcon.vue";

export type SelectOption = { value: string; text: string };

defineProps<{
  modelValue: string;
  options: SelectOption[];
  placeholder?: string;
  width?: number;
  disabled?: boolean;
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

function onChange(event: Event): void {
  emit("update:modelValue", (event.target as HTMLSelectElement).value);
}

// Inside an MvField: take its id and describing note.
const field = inject(fieldKey, null);
</script>

<template>
  <div class="mv-select-wrap" :style="{ width: width != null ? `${width}px` : undefined }">
    <select
      :id="field?.id"
      :aria-describedby="field?.describedBy()"
      class="mv-select"
      :class="{ 'mv-select--empty': modelValue === '' }"
      :value="modelValue"
      :disabled="disabled"
      @change="onChange"
    >
      <option value="">{{ placeholder }}</option>
      <option v-for="o in options" :key="o.value" :value="o.value">
        {{ o.text }}
      </option>
    </select>
    <span class="mv-select-chevron">
      <MvIcon name="chevron" :size="14" />
    </span>
  </div>
</template>

<style scoped lang="scss">
@use "./control";

.mv-select-wrap {
  position: relative;
  display: inline-block;
}
.mv-select {
  @include control.control;
  width: 100%;
  padding-right: calc(var(--control-px) + 14px + var(--s-1));
  appearance: none;
  -webkit-appearance: none;
}
.mv-select--empty {
  color: var(--fg-muted);
}
.mv-select-chevron {
  position: absolute;
  right: var(--control-px);
  top: 50%;
  transform: translateY(-50%);
  pointer-events: none;
  color: var(--fg-muted);
}
.mv-select option {
  color: var(--fg);
  background: var(--bg);
}
</style>
