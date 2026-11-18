import { onBeforeUnmount, ref, useId, watch } from "vue";

// A top-bar popover's open/close behaviour, shared by the settings and the
// account menus (design-direction "Navigation"). A disclosure, not an ARIA
// menu: its contents are buttons, selects and links, reached with Tab.
// Bind `root` on the wrapper, `onFocusOut` on it, and give the trigger the
// class `mv-menu-trigger` with `aria-controls="panelId"`.
export default function useDisclosure() {
  const root = ref<HTMLElement | null>(null);
  const open = ref(false);
  const panelId = useId();

  function focusTrigger(): void {
    root.value?.querySelector<HTMLElement>(".mv-menu-trigger")?.focus();
  }

  // Close on a press outside. `pointerdown` rather than `focusout`: a click on
  // the panel's padding, or a Safari click that never moves focus, would
  // otherwise close it wrongly or not at all.
  function onPointerDown(event: PointerEvent): void {
    if (!root.value?.contains(event.target as Node)) open.value = false;
  }
  // Escape is heard on the document: a click on the panel's padding leaves
  // focus on <body>, outside the wrapper.
  function onKeyDown(event: KeyboardEvent): void {
    if (event.key !== "Escape") return;
    open.value = false;
    focusTrigger();
  }
  function listen(on: boolean): void {
    if (on) {
      document.addEventListener("pointerdown", onPointerDown);
      document.addEventListener("keydown", onKeyDown);
    } else {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    }
  }
  watch(open, listen);
  onBeforeUnmount(() => listen(false));

  // Close when focus leaves by keyboard (Tab past the last control).
  function onFocusOut(event: FocusEvent): void {
    const next = event.relatedTarget as Node | null;
    if (next && !root.value?.contains(next)) open.value = false;
  }

  return { root, open, panelId, onFocusOut, focusTrigger };
}
