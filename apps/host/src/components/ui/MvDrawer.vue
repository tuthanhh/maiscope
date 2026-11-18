<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from "vue";
import MvButton from "./MvButton.vue";

// design-direction "Drawer": filters, from the right on desktop and from the
// bottom on phones, over the scrim. A heading, an action slot (Reset), the
// fields, no footer: whatever it holds applies live. A modal dialog while
// open — focus moves in, Tab stays in, Escape or the scrim closes it, and
// focus returns to whatever opened it.
defineProps<{ title: string; closeLabel: string }>();
const open = defineModel<boolean>({ required: true });

const titleId = useId();
const panel = ref<HTMLElement | null>(null);
let opener: HTMLElement | null = null;

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function onKeyDown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    open.value = false;
    return;
  }
  if (event.key !== "Tab" || !panel.value) return;
  const items = [...panel.value.querySelectorAll<HTMLElement>(FOCUSABLE)];
  if (items.length === 0) return;
  const first = items[0];
  const last = items[items.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

watch(open, async (isOpen) => {
  if (isOpen) {
    opener = document.activeElement as HTMLElement | null;
    document.documentElement.style.overflow = "hidden";
    await nextTick();
    panel.value?.focus();
  } else {
    document.documentElement.style.overflow = "";
    opener?.focus();
    opener = null;
  }
});
onBeforeUnmount(() => {
  document.documentElement.style.overflow = "";
});
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="mv-drawer-scrim" @click.self="open = false" @keydown="onKeyDown">
      <section
        ref="panel"
        class="mv-drawer"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        tabindex="-1"
      >
        <header class="mv-drawer-head">
          <h2 :id="titleId" class="t-heading mv-drawer-title">{{ title }}</h2>
          <slot name="actions" />
          <MvButton kind="icon" icon="x" :label="closeLabel" @click="open = false" />
        </header>
        <div class="mv-drawer-body">
          <slot />
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-drawer-scrim {
  position: fixed;
  inset: 0;
  z-index: 40;
  background: var(--scrim);
}
.mv-drawer {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: min(var(--drawer-w), 100%);
  display: flex;
  flex-direction: column;
  background: var(--bg-panel);
  border-left: var(--line-w) solid var(--line);
  animation: mv-fade var(--dur-slow) ease-out;
}
.mv-drawer:focus {
  outline: none;
}
.mv-drawer-head {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  padding: var(--s-4) var(--s-4) var(--s-4) var(--s-6);
  border-bottom: var(--line-w) solid var(--line);
}
.mv-drawer-title {
  flex: 1;
  margin: 0;
}
.mv-drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--s-6);
}

@media (max-width: #{bp.$bp-md - 1px}) {
  .mv-drawer {
    top: auto;
    left: 0;
    width: 100%;
    max-height: 85dvh;
    border-left: none;
    border-top: var(--line-w) solid var(--line);
    padding-bottom: env(safe-area-inset-bottom);
  }
  .mv-drawer-head {
    padding-left: var(--s-4);
  }
  .mv-drawer-body {
    padding: var(--s-4);
  }
}
</style>
