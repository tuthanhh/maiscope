import { computed } from "vue";
import { useRoute, useRouter, type LocationQuery } from "vue-router";
import type { Sheet } from "~/types";

// The song modal lives in the URL (design-direction "Modal"): `?song=<id>`,
// with `type` and `difficulty` naming the selected sheet. A card links to it,
// so the link is shareable and Back closes the modal.
const MODAL_KEYS = ["song", "type", "difficulty"] as const;

function withoutModal(query: LocationQuery): LocationQuery {
  const rest = { ...query };
  for (const key of MODAL_KEYS) delete rest[key];
  return rest;
}

function single(value: LocationQuery[string] | undefined): string | null {
  return typeof value === "string" && value !== "" ? value : null;
}

export default function useSongModal() {
  const route = useRoute();
  const router = useRouter();

  const songId = computed(() => single(route.query.song));
  const type = computed(() => single(route.query.type));
  const difficulty = computed(() => single(route.query.difficulty));

  /** Where a card for `sheet` links: this page, with the modal open on it. */
  function linkTo(sheet: Sheet) {
    return {
      query: {
        ...withoutModal(route.query),
        song: sheet.song.id,
        type: sheet.type,
        difficulty: sheet.difficulty,
      },
    };
  }

  function open(sheet: Sheet): void {
    router.push(linkTo(sheet));
  }

  /** Switch the selected sheet in place: no new history entry per chip. */
  function select(sheet: Sheet): void {
    router.replace(linkTo(sheet));
  }

  // Opened from this page, the entry before is the page without the modal:
  // go back to it, so Back afterwards does not reopen the modal. Opened from
  // a shared link there is no such entry; replace instead.
  function close(): void {
    const target = { query: withoutModal(route.query) };
    const back = (window.history.state as { back?: string } | null)?.back;
    // Both sides normalised: an entry loaded from a typed URL keeps its raw
    // spelling in `back`.
    const previous = back == null ? null : router.resolve(back).fullPath;
    if (previous !== null && previous === router.resolve(target).fullPath) router.back();
    else router.replace(target);
  }

  return { songId, type, difficulty, linkTo, open, select, close };
}
