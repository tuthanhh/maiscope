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

  function stop(): void {
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
      await new Promise((r) => requestAnimationFrame(r));
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
