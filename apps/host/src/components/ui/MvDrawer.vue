<script setup lang="ts">
import { ref, useId } from "vue";
import useDialogFocus from "~/composables/useDialogFocus";
import MvButton from "./MvButton.vue";

// design-direction "Drawer": filters, a dialog floating in the middle of the
// screen on desktop and a sheet from the bottom on phones, over the scrim. A heading, an action slot (Reset), the
// fields, no footer: whatever it holds applies live. A modal dialog while
// open (`useDialogFocus`).
defineProps<{ title: string; closeLabel: string }>();
const open = defineModel<boolean>({ required: true });

const titleId = useId();
const panel = ref<HTMLElement | null>(null);
const { onKeyDown } = useDialogFocus(open, panel, () => {
  open.value = false;
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
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--s-6);
  background: var(--scrim);
}
.mv-drawer {
  width: min(var(--modal-w), 100%);
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-panel);
  border: var(--line-w) solid var(--line);
  border-radius: var(--r);
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
  .mv-drawer-scrim {
    align-items: flex-end;
    padding: 0;
  }
  .mv-drawer {
    width: 100%;
    max-height: 85dvh;
    border: none;
    border-top: var(--line-w) solid var(--line);
    border-radius: 0;
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
