<script setup lang="ts">
import { ref, toRef } from "vue";
import useDialogFocus from "~/composables/useDialogFocus";

// design-direction "Modal": `--modal-w`, at most 90vh, over the scrim; closes
// on a scrim tap, Escape and the icon button the content places. On phones a
// full-height sheet from the bottom with the same content. Whoever renders it
// decides when it is open (`open`) and what closing means (`close`).
const props = defineProps<{
  open: boolean;
  /** The id of the element that names the dialog, e.g. the song title. */
  labelledby?: string;
  /** The dialog's name when no element on it names it. */
  label?: string;
}>();
const emit = defineEmits<{ close: [] }>();

const panel = ref<HTMLElement | null>(null);
const { onKeyDown } = useDialogFocus(toRef(props, "open"), panel, () => emit("close"));
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="mv-modal-scrim" @click.self="emit('close')" @keydown="onKeyDown">
      <section
        ref="panel"
        class="mv-modal"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="labelledby"
        :aria-label="labelledby ? undefined : label"
        tabindex="-1"
      >
        <slot />
      </section>
    </div>
  </Teleport>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-modal-scrim {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--s-4);
  background: var(--scrim);
}
.mv-modal {
  width: var(--modal-w);
  max-width: 100%;
  max-height: 90vh;
  overflow-y: auto;
  background: var(--bg);
  border: var(--line-w) solid var(--line);
  border-radius: var(--r);
  animation: mv-fade var(--dur-slow) ease-out;
}
.mv-modal:focus {
  outline: none;
}

@media (max-width: #{bp.$bp-md - 1px}) {
  .mv-modal-scrim {
    align-items: flex-end;
    padding: 0;
  }
  .mv-modal {
    width: 100%;
    height: 100%;
    max-height: 100dvh;
    border: none;
    border-radius: 0;
    padding-bottom: env(safe-area-inset-bottom);
  }
}
</style>
