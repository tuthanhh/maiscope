// The UI icon set: shapes from Lucide (https://lucide.dev, ISC licence;
// lucide-static 0.460.0), vendored as inner SVG markup so no icon library
// ships to the browser. Each value draws inside a 24x24 viewBox; MvIcon wraps
// it with stroke=currentColor so colour follows CSS. `fill` is reserved as a
// state signal (bookmarked, playing) — see MvIcon's `filled` prop. To add one,
// copy the icon's inner markup from lucide-static at the same version.
//
// ISC License — Copyright (c) for portions of Lucide are held by Cole Bemis
// 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are
// held by Lucide Contributors 2022. Permission to use, copy, modify, and/or
// distribute this software for any purpose with or without fee is hereby
// granted, provided that the above copyright notice and this permission
// notice appear in all copies. The built app carries it in
// public/THIRD_PARTY_NOTICES.txt, served at /THIRD_PARTY_NOTICES.txt.
const icons = {
  // lucide: search
  search: '<circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />',
  // lucide: x
  x: '<path d="M18 6 6 18" /><path d="m6 6 12 12" />',
  // lucide: chevron-down
  chevron: '<path d="m6 9 6 6 6-6" />',
  // lucide: sliders-horizontal
  sliders: '<line x1="21" x2="14" y1="4" y2="4" /><line x1="10" x2="3" y1="4" y2="4" /><line x1="21" x2="12" y1="12" y2="12" /><line x1="8" x2="3" y1="12" y2="12" /><line x1="21" x2="16" y1="20" y2="20" /><line x1="12" x2="3" y1="20" y2="20" /><line x1="14" x2="14" y1="2" y2="6" /><line x1="8" x2="8" y1="10" y2="14" /><line x1="16" x2="16" y1="18" y2="22" />',
  // lucide: list
  list: '<path d="M3 12h.01" /><path d="M3 18h.01" /><path d="M3 6h.01" /><path d="M8 12h13" /><path d="M8 18h13" /><path d="M8 6h13" />',
  // lucide: layout-grid
  grid: '<rect width="7" height="7" x="3" y="3" rx="1" /><rect width="7" height="7" x="14" y="3" rx="1" /><rect width="7" height="7" x="14" y="14" rx="1" /><rect width="7" height="7" x="3" y="14" rx="1" />',
  // lucide: bookmark
  bookmark: '<path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z" />',
  // lucide: sun
  sun: '<circle cx="12" cy="12" r="4" /><path d="M12 2v2" /><path d="M12 20v2" /><path d="m4.93 4.93 1.41 1.41" /><path d="m17.66 17.66 1.41 1.41" /><path d="M2 12h2" /><path d="M20 12h2" /><path d="m6.34 17.66-1.41 1.41" /><path d="m19.07 4.93-1.41 1.41" />',
  // lucide: moon
  moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />',
  // lucide: play
  play: '<polygon points="6 3 20 12 6 21 6 3" />',
  // lucide: pause
  pause: '<rect x="14" y="4" width="4" height="16" rx="1" /><rect x="6" y="4" width="4" height="16" rx="1" />',
  // lucide: rotate-ccw
  restart: '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path d="M3 3v5h5" />',
  // lucide: chevron-left
  chevronLeft: '<path d="m15 18-6-6 6-6" />',
  // lucide: chevron-right
  chevronRight: '<path d="m9 18 6-6-6-6" />',
  // lucide: pen-line
  pen: '<path d="M12 20h9" /><path d="M16.376 3.622a1 1 0 0 1 3.002 3.002L7.368 18.635a2 2 0 0 1-.855.506l-2.872.838a.5.5 0 0 1-.62-.62l.838-2.872a2 2 0 0 1 .506-.854z" />',
  // lucide: share
  share: '<path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8" /><polyline points="16 6 12 2 8 6" /><line x1="12" x2="12" y1="2" y2="15" />',
  // lucide: shuffle
  shuffle: '<path d="M2 18h1.4c1.3 0 2.5-.6 3.3-1.7l6.1-8.6c.7-1.1 2-1.7 3.3-1.7H22" /><path d="m18 2 4 4-4 4" /><path d="M2 6h1.9c1.5 0 2.9.9 3.6 2.2" /><path d="M22 18h-5.9c-1.3 0-2.6-.7-3.3-1.8l-.5-.8" /><path d="m18 14 4 4-4 4" />',
  // lucide: circle-user-round
  user: '<path d="M18 20a6 6 0 0 0-12 0" /><circle cx="12" cy="10" r="4" /><circle cx="12" cy="12" r="10" />',
  // lucide: library-big
  songs: '<rect width="8" height="18" x="3" y="3" rx="1" /><path d="M7 3v18" /><path d="M20.4 18.9c.2.5-.1 1.1-.6 1.3l-1.9.7c-.5.2-1.1-.1-1.3-.6L11.1 5.1c-.2-.5.1-1.1.6-1.3l1.9-.7c.5-.2 1.1.1 1.3.6Z" />',
  // lucide: settings
  settings: '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" /><circle cx="12" cy="12" r="3" />',
  // lucide: users
  community: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" /><circle cx="9" cy="7" r="4" /><path d="M22 21v-2a4 4 0 0 0-3-3.87" /><path d="M16 3.13a4 4 0 0 1 0 7.75" />',
} as const;

export type IconName = keyof typeof icons;

export default icons;
