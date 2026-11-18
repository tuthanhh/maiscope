import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

// How many dialogs hold the page's scroll lock. The drawer and the modal can
// be mounted together, so the last to let go unlocks, and a closed dialog
// mounting never touches a lock another holds.
let locks = 0;

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

// A modal dialog's focus contract, shared by the drawer and the modal: while
// `open`, focus moves into `panel`, Tab wraps inside it, Escape calls
// `close`, the page behind does not scroll; on closing, focus returns to
// whatever had it before. Bind the returned `onKeyDown` on the dialog.
export default function useDialogFocus(
  open: Ref<boolean>,
  panel: Ref<HTMLElement | null>,
  close: () => void,
) {
  let opener: HTMLElement | null = null;
  let holding = false;

  function lock(): void {
    if (holding) return;
    holding = true;
    locks++;
    document.documentElement.style.overflow = "hidden";
  }
  function unlock(): void {
    if (!holding) return;
    holding = false;
    locks--;
    if (locks === 0) document.documentElement.style.overflow = "";
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      close();
      return;
    }
    if (event.key !== "Tab" || !panel.value) return;
    const items = [...panel.value.querySelectorAll<HTMLElement>(FOCUSABLE)];
    if (items.length === 0) return;
    const first = items[0];
    const last = items[items.length - 1];
    // Focus on the panel itself (as on opening) or somewhere it should not
    // be: the next Tab lands inside.
    if (!items.includes(document.activeElement as HTMLElement)) {
      event.preventDefault();
      (event.shiftKey ? last : first).focus();
      return;
    }
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  watch(
    open,
    async (isOpen) => {
      if (isOpen) {
        opener = document.activeElement as HTMLElement | null;
        lock();
        await nextTick();
        panel.value?.focus();
      } else {
        if (!holding) return;
        unlock();
        // The opener may be gone (a card on a page that changed).
        if (opener?.isConnected) opener.focus();
        opener = null;
      }
    },
    { immediate: true },
  );
  onBeforeUnmount(unlock);

  return { onKeyDown };
}
