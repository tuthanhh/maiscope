import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import { useRoute } from "vue-router";

// Whether the engine canvas is on screen. The phone bottom bar hides while it
// is (design-direction, "Navigation"), so playback gets the whole height.
// Observes `#bevy` only while the Play route is open; the canvas is parked
// off-screen everywhere else.
export default function useEngineInView(): Ref<boolean> {
  const route = useRoute();
  const inView = ref(false);
  let observer: IntersectionObserver | null = null;
  // Bumped on every route change and on unmount, so a frame wait that
  // resolves after the user has moved on does not attach a stale observer.
  let generation = 0;

  function stop(): void {
    generation++;
    observer?.disconnect();
    observer = null;
    inView.value = false;
  }

  watch(
    () => route.name,
    async (name) => {
      stop();
      if (name !== "visualizer" || typeof IntersectionObserver === "undefined") return;
      // The page mounts the canvas on its own mount; wait a frame for it.
      const mine = generation;
      await new Promise((r) => requestAnimationFrame(r));
      if (mine !== generation || route.name !== "visualizer") return;
      const canvas = document.getElementById("bevy");
      if (!canvas) return;
      observer = new IntersectionObserver(
        ([entry]) => {
          inView.value = entry.isIntersecting;
        },
        { threshold: 0.25 },
      );
      observer.observe(canvas);
    },
    { immediate: true },
  );
  onBeforeUnmount(stop);
  return inView;
}
