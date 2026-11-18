<script setup lang="ts">
import { computed, inject, nextTick, ref, useId, watch } from "vue";
import { fieldKey } from "./fieldContext";
import MvIcon from "./MvIcon.vue";
import type { IconName } from "./icons";

// A text input with a list of known values (ARIA 1.2 combobox, list
// autocomplete). Focus or click drops the whole list open, like a select;
// typing narrows it; arrows and Enter choose; Escape closes the list. Free
// text stays allowed: the value is whatever is typed or chosen.
const props = withDefaults(
  defineProps<{
    modelValue: string;
    options: string[];
    placeholder?: string;
    /** The accessible name when no Field labels this control. */
    label?: string;
    /** Rendered at most; more matches ask for more typing. */
    limit?: number;
    /** Shown under a capped list, e.g. "Type to narrow". */
    moreLabel?: string;
    /** A leading icon in `--fg-muted`, e.g. search. */
    icon?: IconName;
  }>(),
  { limit: 200, moreLabel: "…" },
);
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const field = inject(fieldKey, null);
const listId = useId();
const open = ref(false);
const active = ref(-1);
const list = ref<HTMLElement | null>(null);

const matches = computed(() => {
  const query = props.modelValue.trim().toLowerCase();
  if (query === "") return props.options;
  // Names that start with what was typed first, then the rest that contain
  // it, each in the options' own order.
  const starts: string[] = [];
  const contains: string[] = [];
  for (const option of props.options) {
    const lower = option.toLowerCase();
    if (lower.startsWith(query)) starts.push(option);
    else if (lower.includes(query)) contains.push(option);
  }
  return [...starts, ...contains];
});
const shown = computed(() => matches.value.slice(0, props.limit));
const optionId = (i: number) => `${listId}-${i}`;

const expanded = computed(() => open.value && shown.value.length > 0);

watch(shown, () => {
  active.value = -1;
});
watch(active, (i) => {
  if (i >= 0) document.getElementById(optionId(i))?.scrollIntoView({ block: "nearest" });
});
// A closed list forgets its highlight; an opened one is scrolled into the
// dialog's view, which may have it below the fold.
watch(expanded, async (isOpen) => {
  if (!isOpen) {
    active.value = -1;
    return;
  }
  await nextTick();
  list.value?.scrollIntoView({ block: "nearest" });
});

function choose(value: string): void {
  emit("update:modelValue", value);
  open.value = false;
}
function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
  open.value = true;
}
function onKeyDown(event: KeyboardEvent): void {
  const count = shown.value.length;
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      open.value = true;
      if (count) active.value = (active.value + 1) % count;
      break;
    case "ArrowUp":
      event.preventDefault();
      open.value = true;
      if (count) active.value = active.value <= 0 ? count - 1 : active.value - 1;
      break;
    case "Enter":
      if (open.value && active.value >= 0) {
        event.preventDefault();
        choose(shown.value[active.value]);
      }
      break;
    case "Escape":
      // Close the list, not the dialog the combobox sits in.
      if (open.value) {
        event.stopPropagation();
        open.value = false;
      }
      break;
    case "Tab":
      open.value = false;
      break;
  }
}
// A tap on the list that still moved focus (some touch browsers ignore the
// list's `mousedown.prevent`) must not close it before the choice lands.
function onBlur(event: FocusEvent): void {
  if (list.value?.contains(event.relatedTarget as Node | null)) return;
  open.value = false;
}
function clear(): void {
  emit("update:modelValue", "");
}
</script>

<template>
  <div class="mv-combobox">
    <span v-if="icon" class="mv-combobox-icon" aria-hidden="true">
      <MvIcon :name="icon" :size="15" />
    </span>
    <input
      :id="field?.id"
      :aria-describedby="field?.describedBy()"
      :aria-label="field ? undefined : label"
      class="mv-combobox-input"
      :class="{ 'mv-combobox-input--icon': icon }"
      role="combobox"
      aria-autocomplete="list"
      :aria-expanded="expanded"
      :aria-controls="listId"
      :aria-activedescendant="expanded && active >= 0 ? optionId(active) : undefined"
      autocomplete="off"
      :value="modelValue"
      :placeholder="placeholder"
      @input="onInput"
      @focus="open = true"
      @click="open = true"
      @blur="onBlur"
      @keydown="onKeyDown"
    />
    <button
      v-if="modelValue"
      type="button"
      class="mv-combobox-clear"
      tabindex="-1"
      aria-hidden="true"
      @mousedown.prevent="clear"
    >
      <MvIcon name="x" :size="14" />
    </button>
    <span v-else class="mv-combobox-chevron" aria-hidden="true">
      <MvIcon name="chevron" :size="14" />
    </span>
    <!-- `mousedown.prevent` on the whole list — rows, padding, scrollbar,
         the "keep typing" line: none may blur the input, or the list closes
         before the click lands. -->
    <ul
      v-show="expanded"
      :id="listId"
      ref="list"
      class="mv-combobox-list t-body"
      role="listbox"
      tabindex="-1"
      @mousedown.prevent
    >
      <li
        v-for="(option, i) in shown"
        :id="optionId(i)"
        :key="option"
        class="mv-combobox-option"
        :class="{
          'mv-combobox-option--active': i === active,
          'mv-combobox-option--current': option === modelValue,
        }"
        role="option"
        :aria-selected="i === active"
        @click="choose(option)"
      >
        {{ option }}
      </li>
      <li v-if="matches.length > shown.length" class="mv-combobox-more t-meta t-muted" role="presentation">
        {{ moreLabel }}
      </li>
    </ul>
  </div>
</template>

<style scoped lang="scss">
@use "./control";

.mv-combobox {
  position: relative;
}
.mv-combobox-input {
  @include control.control;
  width: 100%;
  padding-right: calc(var(--control-px) + 22px);
}
.mv-combobox-input--icon {
  padding-left: calc(var(--control-px) + 15px + var(--s-2));
}
.mv-combobox-icon {
  position: absolute;
  left: var(--control-px);
  top: calc(var(--control-h) / 2);
  transform: translateY(-50%);
  display: flex;
  color: var(--fg-muted);
  pointer-events: none;
}
.mv-combobox-chevron,
.mv-combobox-clear {
  position: absolute;
  right: var(--control-px);
  top: calc(var(--control-h) / 2);
  transform: translateY(-50%);
  display: flex;
  color: var(--fg-muted);
}
.mv-combobox-chevron {
  pointer-events: none;
}
.mv-combobox-clear {
  // A 28px target around the 14px icon, centred on the same spot.
  right: calc(var(--control-px) - 7px);
  padding: 7px;
  border: none;
  background: none;
  cursor: pointer;
}
.mv-combobox-list {
  position: absolute;
  z-index: 1;
  top: calc(100% + var(--s-1));
  left: 0;
  right: 0;
  max-height: 260px;
  overflow-y: auto;
  margin: 0;
  padding: var(--s-1) 0;
  list-style: none;
  background: var(--bg);
  border: var(--line-w) solid var(--line-strong);
  border-radius: var(--r);
}
.mv-combobox-option {
  padding: var(--s-2) var(--control-px);
  cursor: pointer;
  overflow-wrap: anywhere;
}
.mv-combobox-option:hover,
.mv-combobox-option--active {
  background: var(--bg-panel);
}
.mv-combobox-option--current {
  font-weight: 600;
}
.mv-combobox-more {
  padding: var(--s-2) var(--control-px);
}
</style>
