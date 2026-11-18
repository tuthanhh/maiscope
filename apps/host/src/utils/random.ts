export function pickItem<T>(items: T[]) {
  return items[Math.trunc(items.length * Math.random())];
}
