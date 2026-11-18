import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

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
        document.documentElement.style.overflow = "hidden";
        await nextTick();
        panel.value?.focus();
      } else {
        document.documentElement.style.overflow = "";
        // The opener may be gone (a card on a page that changed).
        if (opener?.isConnected) opener.focus();
        opener = null;
      }
    },
    { immediate: true },
  );
  onBeforeUnmount(() => {
    document.documentElement.style.overflow = "";
  });

  return { onKeyDown };
}
