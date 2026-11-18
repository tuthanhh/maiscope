export function toLocalDateString(date: Date) {
  return new Intl.DateTimeFormat().format(date);
}

export function toLocalISOTimeString(date: Date) {
  const d = new Date(date.getTime());
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, -1);
}

export function toLocalISODateString(date: Date) {
  return toLocalISOTimeString(date).slice(0, 10);
}

export function toPercentageString(n: number | null | undefined) {
  if (n == null) return n;
  if (Number.isNaN(n)) return '?';
  return `${(n * 100).toFixed(2)}%`;
}

// Thousands grouped with a no-break space, as the design's "1 038", so a
// narrow row never breaks inside a number.
export function groupThousands(n: number) {
  return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
}
