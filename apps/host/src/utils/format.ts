// Thousands grouped with a no-break space, as the design's "1 038", so a
// narrow row never breaks inside a number.
export function groupThousands(n: number) {
  return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
}
