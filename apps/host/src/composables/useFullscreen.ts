import { getCurrentScope, onScopeDispose, ref, type Ref } from "vue";

// Fullscreen for one element (visualizer-majdata 04): the Play page's stage.
// `active` follows the browser, not `toggle`, so an exit made with Escape
// shows too. `supported` is false where the API is missing — iPhone Safari
// offers fullscreen to video elements only — and the page hides the toggle.
export default function useFullscreen(target: Ref<HTMLElement | null>): {
  supported: boolean;
  active: Ref<boolean>;
  toggle: () => Promise<void>;
} {
  const supported = typeof document !== "undefined" && document.fullscreenEnabled === true;
  const active = ref(false);

  function sync(): void {
    active.value = target.value !== null && document.fullscreenElement === target.value;
  }

  async function toggle(): Promise<void> {
    if (!supported) return;
    if (document.fullscreenElement) await document.exitFullscreen();
    else await target.value?.requestFullscreen();
  }

  if (supported) {
    document.addEventListener("fullscreenchange", sync);
    if (getCurrentScope()) {
      onScopeDispose(() => document.removeEventListener("fullscreenchange", sync));
    }
  }

  return { supported, active, toggle };
}
