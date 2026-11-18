// Thousands grouped with a no-break space, as the design's "1 038", so a
// narrow row never breaks inside a number.
export function groupThousands(n: number) {
  return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
}

// A release date as the screens show it: `2025/09/11`.
export function formatReleaseDate(date: string | undefined) {
  return date?.replaceAll('-', '/');
}

const NOTE_KEYS = ['total', 'tap', 'hold', 'slide', 'touch', 'break'];

// `TOTAL 1000 · TAP 500 · HOLD 100 …`, skipping counts the sheet lacks.
// `totalLabel` is the localised TOTAL; the note kinds stay as the game
// prints them.
export function formatNoteCounts(
  counts: Record<string, number | null> | undefined,
  totalLabel: string,
) {
  if (!counts) return '';
  return NOTE_KEYS.filter((key) => counts[key] != null)
    .map((key) => `${key === 'total' ? totalLabel : key.toUpperCase()} ${counts[key]}`)
    .join(' · ');
}
