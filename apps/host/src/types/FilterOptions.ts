import type { FilterOption } from './FilterOption';

// The select options the browse drawer and level range offer (utils/filter.ts).
export type FilterOptions = {
  categories: FilterOption<string>[] | null;
  versions: FilterOption<string>[] | null;
  levels: FilterOption<number>[] | null;
  noteDesigners: FilterOption<string>[] | null;
  regions: FilterOption<string>[] | null;
};
