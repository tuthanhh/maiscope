import type { InjectionKey } from "vue";

// What a Field tells the control inside it: the id its <label for> names, and
// the id of its help or error text for `aria-describedby`.
export type FieldContext = { id: string; describedBy: () => string | undefined; invalid: () => boolean };
export const fieldKey: InjectionKey<FieldContext> = Symbol("mv-field");
