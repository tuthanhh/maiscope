// Stroke-only line icon set, ported from the React prototype's `I` map.
// Each value is the inner markup of a 24x24 viewBox SVG; MvIcon wraps it
// with stroke=currentColor so color follows CSS. `fill` is reserved as a
// state signal (bookmarked, playing) — see MvIcon's `filled` prop.
const icons = {
  search: '<circle cx="11" cy="11" r="7" /><path d="M21 21l-4-4" />',
  x: '<path d="M6 6l12 12M18 6L6 18" />',
  chevron: '<path d="M6 9l6 6 6-6" />',
  back: '<path d="M15 5l-7 7 7 7" />',
  sliders:
    '<line x1="4" y1="8" x2="20" y2="8" /><line x1="4" y1="16" x2="20" y2="16" /><circle cx="9" cy="8" r="2.4" /><circle cx="15" cy="16" r="2.4" />',
  list:
    '<line x1="4" y1="7" x2="20" y2="7" /><line x1="4" y1="12" x2="20" y2="12" /><line x1="4" y1="17" x2="20" y2="17" />',
  grid:
    '<rect x="4" y="4" width="7" height="7" /><rect x="13" y="4" width="7" height="7" /><rect x="4" y="13" width="7" height="7" /><rect x="13" y="13" width="7" height="7" />',
  bookmark: '<path d="M6 3h12v18l-6-4-6 4z" />',
  globe:
    '<circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18" />',
  sun:
    '<circle cx="12" cy="12" r="4" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2" />',
  moon: '<path d="M21 13A9 9 0 1 1 11 3a7 7 0 0 0 10 10z" />',
  ext:
    '<path d="M14 4h6v6M20 4l-9 9" /><path d="M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5" />',
  lock:
    '<rect x="5" y="11" width="14" height="9" rx="1" /><path d="M8 11V7a4 4 0 0 1 8 0v4" />',
  youtube:
    '<rect x="2" y="5" width="20" height="14" rx="4" /><path d="M10 9l5 3-5 3z" />',
  // Visualizer transport — used by the Player slice (next layer).
  play: '<path d="M7 5l12 7-12 7z" />',
  pause: '<rect x="6" y="5" width="4" height="14" /><rect x="14" y="5" width="4" height="14" />',
  prev: '<path d="M19 5v14L9 12z" /><rect x="5" y="5" width="2" height="14" />',
  next: '<path d="M5 5v14l10-7z" /><rect x="17" y="5" width="2" height="14" />',
  restart: '<path d="M3 12a9 9 0 1 0 3-6.7" /><path d="M3 4v5h5" />',
} as const;

export type IconName = keyof typeof icons;

export default icons;
